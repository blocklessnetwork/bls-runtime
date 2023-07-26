use std::{collections::HashMap, sync::{Mutex, Once}, cmp::min};
use json::JsonValue;
use lazy_static::lazy_static;
use anyhow::Context;
use wasi_common::{
    WasiCtx, 
    BlocklessModule, 
    ModuleType
};
use wasmtime::{
    Func, 
    Store, 
    Module, 
    Linker, 
    Extern, 
    Caller, 
    Memory, 
    TypedFunc, 
    AsContext,
    AsContextMut, StoreContextMut, 
};

struct StorePtr(*mut Store<WasiCtx>);

impl StorePtr {
    #[inline]
    unsafe fn store(&self) -> &Store<WasiCtx> {
        &*self.0
    }

    #[inline]
    unsafe fn store_mut(&self) -> &mut Store<WasiCtx> {
        &mut *self.0
    }
}

fn get_store() ->  &'static Store<WasiCtx> {
    unsafe {STORE_PTR.store()}
}


fn get_store_mut() ->  &'static mut Store<WasiCtx> {
    unsafe {STORE_PTR.store_mut()}
}

unsafe impl Sync for StorePtr {}
unsafe impl Send for StorePtr {}

static mut STORE_PTR: StorePtr = StorePtr(std::ptr::null_mut());

static STORE_PTR_ONCE: Once = Once::new();

lazy_static! {
    static ref INS_CTX: Mutex<InstanceCtx> = Mutex::new(InstanceCtx::new());
}

struct InstanceCtx {
    //key is module::method.
    module_caller: HashMap<String, InstanceCaller>,
    //key is module name.
    instance_infos: HashMap<String, InstanceInfo>,
}

impl InstanceCtx {
    fn new() -> Self {
        Self { 
            module_caller: HashMap::new(), 
            instance_infos: HashMap::new(),
        }
    }
}

struct InstanceInfo {
    mem: Memory,
    alloc: Option<TypedFunc<u32, i32>>,
    dealloc: Option<TypedFunc<(i32, u32), ()>>,
    export_funcs: HashMap<String, Func>,
}

impl InstanceInfo {
    fn instance_caller(&self, method: &str) -> Option<InstanceCaller> {
        let mem = self.mem.clone();
        self.export_funcs.get(method)
            .map(|func| InstanceCaller {
                mem,
                alloc: self.alloc.unwrap().clone(),
                dealloc: self.dealloc.unwrap().clone(),
                func: func.typed::<(i32, u32), u32>(get_store()).unwrap(),
            })
    }
}

struct InstanceCaller {
    mem: Memory,
    alloc: TypedFunc<u32, i32>,
    dealloc: TypedFunc<(i32, u32), ()>,
    func: TypedFunc<(i32, u32), u32>
}

struct RegisterReq {
    module: String,
    methods: Vec<String>,
}

fn process_register_req(json_str: &str) -> anyhow::Result<RegisterReq> {
    let json_obj = json::parse(json_str)?;
    let module = match json_obj["module"].as_str() {
        Some(m) => m.to_lowercase(),
        None => anyhow::bail!("not found module node"),
    };
    let methods = json_obj["methods"]
        .members()
        .map(|m| m.to_string())
        .collect::<Vec<_>>();
    Ok(RegisterReq {
        module,
        methods
    })
}

enum RegisterErrorKind {
    Success,
    JSON_ERROR,
}

fn error_json(msg: &str) -> String {
    let mut obj = json::object::Object::new();
    let code: JsonValue = json::number::Number::from(-1).into();
    obj["code"] = code;
    obj["message"] = msg.to_string().into();
    obj.dump()
}

struct ResponseJson<'a> {
    mem: &'a Memory,
    store: StoreContextMut<'a, WasiCtx>,
    ptr: u32,
    len: u32,
}

impl<'a> ResponseJson<'a> {
    fn new(mem: &'a Memory, store: StoreContextMut<'a, WasiCtx>, ptr: u32, len: u32) -> Self {
        Self { 
            store,
            mem, 
            ptr,
            len
        }
    }
    fn response(&mut self, msg: &str) {
        let mem = self.mem.data_mut(self.store.as_context_mut());
        let start = self.ptr as usize;
        let json = error_json(msg);
        let bs = json.as_bytes();
        let len = min(self.len as usize, bs.len());
        let end = start + len;
        let data = &mut mem[start..end];
        data.copy_from_slice(&bs[0..len]);
    }
}


pub(crate) async fn link_modules(linker: &mut Linker<WasiCtx>, store_ptr: * mut Store<WasiCtx>) -> Option<Module> {
    STORE_PTR_ONCE.call_once(|| unsafe {STORE_PTR = StorePtr(store_ptr)});
    let store = unsafe {STORE_PTR.store_mut()};
    let mut modules: Vec<BlocklessModule> = {
        let lock = store.data().blockless_config.lock().unwrap();
        let cfg = lock.as_ref().unwrap();
        cfg.modules_ref().iter().map(|m| (*m).clone()).collect()
    };
    modules.sort_by(|a, b| a.module_type.partial_cmp(&b.module_type).unwrap());
    let mut entry = None;
    linker.func_wrap4_async("blockless", "register", |mut caller: Caller<'_, WasiCtx>, addr: u32, addr_len: u32, buf: u32, buf_len: u32| {
        Box::new(async move {
            if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
                let mem_slice = mem.data(caller.as_context());
                let start = addr as usize;
                let end = (addr + addr_len) as usize;
                let req_mem = &mem_slice[start..end];
                let str = unsafe {
                    std::str::from_utf8_unchecked(req_mem)
                };
                macro_rules! responseError {
                    ($msg: literal) => {
                        ResponseJson::new(&mem, caller.as_context_mut(), buf, buf_len)
                            .response($msg);
                        return 1;
                    };
                }
                let req = match process_register_req(str) {
                    Ok(req) => req,
                    Err(_) => {
                        responseError!("error parse json");
                    },
                };
                
                return INS_CTX.lock().map(|mut ctx| {
                    for method in req.methods.iter() {
                        let module = ctx.instance_infos.get_mut(&req.module);
                        if module.is_none() {
                            responseError!("no module found");
                        }
                        let module = module.unwrap();
                        let caller = module.instance_caller(method);
                        ctx.module_caller.insert(format!("{}::{method}", &req.module), caller.unwrap());
                    }
                    0
                }).unwrap();
            }
            1
        })
    }).unwrap();
    for m in modules {
        let (m_name, is_entry) = match m.module_type {
            ModuleType::Module => (m.name.as_str(), false),
            ModuleType::Entry => ("", true),
        };
        let module = Module::from_file(store.engine(), &m.file).unwrap();
        if is_entry {
            entry = Some(module);
        } else {
            instance_module(linker, m_name, store.as_context_mut(), &module).await.unwrap();
        }
    }
    entry
}

async fn instance_module(
    linker: &mut Linker<WasiCtx>,
    m_name: &str,
    mut store: impl AsContextMut<Data = WasiCtx>, 
    module: &Module
) -> anyhow::Result<()> {
    let instance = linker.instantiate_async(&mut store, module).await?;
    let mut initial = None;
    let mut funcs = HashMap::<String, Func>::new();
    let mut alloc = None;
    let mut dealloc = None;
    let mut mem = None;
    for export in instance.exports(store.as_context_mut()) {
        let name = export.name().to_string();
        if &name == "memory" {
            mem = export.into_memory();
            continue;
        }
        if let Some(func) = export.into_func() {
            match name.as_str() {
                "_initialize" => {
                    initial = Some(func);
                },
                "alloc" => {
                    alloc = Some(func);
                },
                "dealloc" => {
                    dealloc = Some(func);
                },
                _ => {
                    funcs.insert(name, func);
                },
            };
        }
    }

    let alloc = match alloc.map(|alloc| alloc
        .typed::<u32, i32>(&mut store)
        .context("loading the alloc function")) {
        Some(Ok(r)) => Some(r),
        Some(Err(e)) => return Err(e),
        None => None,
    };

    let dealloc = match dealloc.map(|dealloc| dealloc
        .typed::<(i32, u32), ()>(&mut store)
        .context("loading the dealloc function")) {
        Some(Ok(r)) => Some(r),
        Some(Err(e)) => return Err(e),
        None => None,
    };
    if mem.is_none() {
        anyhow::bail!("memory is not export in module.");
    }
    linker.instance(store.as_context_mut(), m_name, instance)?;
    let mod_info = InstanceInfo {
        alloc,
        export_funcs: funcs,
        dealloc,
        mem: mem.unwrap(),
    };
    //must release the lock, the initial method will access the modules.
    INS_CTX.lock()
        .as_mut()
        .map(|mods| mods.instance_infos.insert(m_name.to_string(), mod_info))
        .unwrap();
    if let Some(func) = initial {
        let func = func
            .typed::<(), ()>(store.as_context())
            .context("loading the Reactor initialization function")?;
        func.call_async(store.as_context_mut(), ())
            .await
            .context("calling the Reactor initialization function")?;
    }
    Ok(())
}