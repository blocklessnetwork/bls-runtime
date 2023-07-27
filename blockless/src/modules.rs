use std::{collections::HashMap, cmp::min};
use tokio::sync::Mutex;
use json::JsonValue;
use lazy_static::lazy_static;
use anyhow::Context;
use std::future::Future;
use crate::{
    RegisterResult, 
    MCallResult
};
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
    AsContextMut, 
    StoreContextMut, 
};

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

type AllocTypedFunc = TypedFunc<u32, i32>;
type DeallocTypedFunc = TypedFunc<(i32, u32), ()>;

struct InstanceInfo {
    mem: Memory,
    alloc: Option<AllocTypedFunc>,
    dealloc: Option<DeallocTypedFunc>,
    export_funcs: HashMap<String, Func>,
}

impl InstanceInfo {
    fn instance_caller(&self, method: &str, store: impl AsContext<Data = WasiCtx>) -> anyhow::Result<InstanceCaller> {
        let export_func = self.export_funcs.get(method)
            .ok_or(anyhow::anyhow!(format!("method: {method} not found")))?;
            
        let mem = self.mem.clone();
        let alloc = self.alloc.ok_or(anyhow::anyhow!("alloc is not found"))?.clone();
        let dealloc = self.dealloc.ok_or(anyhow::anyhow!("dealloc is not found"))?.clone();
        let func = export_func.typed::<(i32, u32), u32>(store)?;
        Ok(InstanceCaller {
            mem,
            func,
            alloc,
            dealloc,
        })
    }
}

struct InstanceCaller {
    mem: Memory,
    alloc: AllocTypedFunc,
    dealloc: DeallocTypedFunc,
    func: TypedFunc<(i32, u32), u32>
}

impl InstanceCaller {
    async fn call(&self, 
        mut store: impl AsContextMut<Data = WasiCtx>,
        param: &str,
    ) -> u32 {
        let mut result = MCallResult::Success;
        let bs = param.as_bytes();
        let ptr = self.alloc.call_async(store.as_context_mut(), bs.len() as u32).await;
        let ptr = match ptr {
            Ok(ptr) => ptr,
            Err(_) => return MCallResult::AllocError.into(),
        };
        let mem_slice = self.mem.data_mut(store.as_context_mut());
        let start = ptr as usize;
        let end = start + bs.len();
        mem_slice[start..end].copy_from_slice(&bs);
        let rs = self.func.call_async(store.as_context_mut(), (ptr, bs.len() as u32)).await;
        if rs.is_err() {
            result = MCallResult::MCallError.into();
        }
        let rs = self.dealloc.call_async(store.as_context_mut(), (ptr, bs.len() as u32)).await;
        if rs.is_err() {
            if let MCallResult::Success = result {
                result = MCallResult::DeallocError;
            }
        }
        result.into()
    }
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

fn error_json(msg: &str) -> String {
    let mut obj = json::object::Object::new();
    let code: JsonValue = json::number::Number::from(-1).into();
    obj["code"] = code;
    obj["message"] = msg.to_string().into();
    obj.dump()
}

struct ResponseErrorJson<'a> {
    mem: &'a Memory,
    store: StoreContextMut<'a, WasiCtx>,
    ptr: u32,
    len: u32,
}

impl<'a> ResponseErrorJson<'a> {
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

fn parse_mcall(param: &str) -> anyhow::Result<(String, String)> {
    let mcall_json = json::parse(param)?;
    let mcall_name = match mcall_json["mcall"].as_str() {
        Some(n) => n.to_string(),
        None => anyhow::bail!("mcall node not found"),
    };
    let params = mcall_json["params"].dump();
    Ok((mcall_name, params))
}

fn mcall_fn<'a>(mut caller: Caller<'a, WasiCtx>, addr: u32, addr_len: u32, buf: u32, buf_len: u32) -> Box<dyn Future<Output = u32> + Send + 'a> {
    Box::new(async move {
        if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
            let mem_slice = mem.data(caller.as_context());
            let start = addr as usize;
            let end = (addr + addr_len) as usize;
            let req_mem = &mem_slice[start..end];
            let json_str = unsafe {
                std::str::from_utf8_unchecked(req_mem)
            };
            macro_rules! responseError {
                ($msg: literal) => {
                    ResponseErrorJson::new(&mem, caller.as_context_mut(), buf, buf_len)
                        .response($msg);
                    return RegisterResult::Fail.into();
                };
                ($msg: expr) => {
                    ResponseErrorJson::new(&mem, caller.as_context_mut(), buf, buf_len)
                        .response($msg);
                    return RegisterResult::Fail.into();
                };
            }
            let (mcall_name, params) = match parse_mcall(json_str) {
                Ok((n, k)) => (n, k),
                Err(e) => {
                    let emsg = format!("error parse json: {}", e.to_string());
                    responseError!(&emsg);
                },
            };
            let ctx = INS_CTX.lock().await;
            let mcaller = ctx.module_caller.get(&mcall_name);
            let mcaller = if mcaller.is_none() {
                responseError!("no mcall register.");
            } else {
                mcaller.unwrap()
            };
            return mcaller.call(caller.as_context_mut(), &params).await;
        }
        RegisterResult::MemoryNotFound.into()
    })
}

fn register_fn<'a>(mut caller: Caller<'a, WasiCtx>, addr: u32, addr_len: u32, buf: u32, buf_len: u32) -> Box<dyn Future<Output = u32> + Send + 'a> {
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
                    ResponseErrorJson::new(&mem, caller.as_context_mut(), buf, buf_len)
                        .response($msg);
                    return MCallResult::Fail.into();
                };
                ($msg: expr) => {
                    ResponseErrorJson::new(&mem, caller.as_context_mut(), buf, buf_len)
                        .response($msg);
                    return MCallResult::Fail.into();
                };
            }
            let req = match process_register_req(str) {
                Ok(req) => req,
                Err(_) => {
                    responseError!("error parse json");
                },
            };
            let  mut ctx = INS_CTX.lock().await;
            for method in req.methods.iter() {
                let module = ctx.instance_infos.get_mut(&req.module);
                if module.is_none() {
                    responseError!("no module found");
                }
                let module = module.unwrap();
                let caller = module.instance_caller(method, caller.as_context_mut());
                let caller = match caller {
                    Ok(c) => c,
                    Err(e) => {
                        let e = format!("caller instance fail, {}", e.to_string());
                        responseError!(&e);
                    },
                };
                ctx.module_caller.insert(format!("{}::{method}", &req.module), caller);
            }
            MCallResult::Success.into()
        } else {
            MCallResult::MemoryNotFound.into()
        }
    })
}

pub(crate) async fn link_modules(linker: &mut Linker<WasiCtx>, store: & mut Store<WasiCtx>) -> Option<Module> {
    let mut modules: Vec<BlocklessModule> = {
        let lock = store.data().blockless_config.lock().unwrap();
        let cfg = lock.as_ref().unwrap();
        cfg.modules_ref().iter().map(|m| (*m).clone()).collect()
    };
    modules.sort_by(|a, b| a.module_type.partial_cmp(&b.module_type).unwrap());
    let mut entry = None;
    linker.func_wrap4_async("blockless", "mcall", |caller: Caller<'_, WasiCtx>, addr: u32, addr_len: u32, buf: u32, buf_len: u32| {
        mcall_fn(caller, addr, addr_len, buf, buf_len)
    }).unwrap();
    linker.func_wrap4_async("blockless", "register", |caller: Caller<'_, WasiCtx>, addr: u32, addr_len: u32, buf: u32, buf_len: u32| {
        register_fn(caller, addr, addr_len, buf, buf_len)
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
        .typed::<(i32, u32), ()>(store.as_context_mut())
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
    INS_CTX.lock().await.instance_infos.insert(m_name.to_string(), mod_info);
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