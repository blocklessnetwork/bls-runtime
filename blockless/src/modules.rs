use anyhow::{anyhow, Context};
use json::JsonValue;
use lazy_static::lazy_static;
use std::future::Future;
use std::sync::Arc;
use std::{cmp::min, collections::HashMap};
use tokio::sync::Mutex;
use wasi_common::{BlocklessModule, ModuleType};
use wasmtime::{
    AsContext, AsContextMut, Caller, Extern, Func, Linker, Memory, Module, Store, StoreContextMut,
    TypedFunc,
};

use crate::context::BlocklessContext as BSContext;
use crate::error::McallError;

lazy_static! {
    static ref INS_CTX: Mutex<InstanceCtx> = Mutex::new(InstanceCtx::new());
}

struct InstanceCtx {
    //key is mem, value is the register module name,
    modules: HashMap<usize, String>,
    //key is module::method.
    module_caller: HashMap<String, InstanceCaller>,
    //key is module name.
    instance_infos: HashMap<String, InstanceInfo>,
}

impl InstanceCtx {
    fn new() -> Self {
        Self {
            modules: HashMap::new(),
            module_caller: HashMap::new(),
            instance_infos: HashMap::new(),
        }
    }
}

type AllocTypedFunc = TypedFunc<u32, i32>;
type DeallocTypedFunc = TypedFunc<(i32, u32), ()>;
type CallerTypedFunc = TypedFunc<(i32, u32, i32, u32), u32>;

struct InstanceInfo {
    mem: Option<Memory>,
    alloc: Option<Arc<AllocTypedFunc>>,
    dealloc: Option<Arc<DeallocTypedFunc>>,
    export_funcs: HashMap<String, Func>,
}

impl InstanceInfo {
    fn instance_caller(
        &self,
        method: &str,
        store: impl AsContext<Data = BSContext>,
    ) -> anyhow::Result<InstanceCaller> {
        let export_func = self
            .export_funcs
            .get(method)
            .ok_or(anyhow::anyhow!(format!("method: {method} not found")))?;

        let mem = self
            .mem
            .ok_or(anyhow::anyhow!("memory is not exported in module."))?
            .clone();
        let alloc = self
            .alloc
            .as_ref()
            .ok_or(anyhow::anyhow!("alloc is not exported in module."))?
            .clone();
        let dealloc = self
            .dealloc
            .as_ref()
            .ok_or(anyhow::anyhow!("dealloc is not exported in module."))?
            .clone();
        let func: Arc<CallerTypedFunc> = Arc::new(export_func.typed(store)?);
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
    alloc: Arc<AllocTypedFunc>,
    dealloc: Arc<DeallocTypedFunc>,
    func: Arc<CallerTypedFunc>,
}

struct MemBuf<'a> {
    mem: &'a Memory,
    buf: u32,
    buf_len: u32,
}

/// The mem wrapper for memory.
impl<'a> MemBuf<'a> {
    fn new(mem: &'a Memory, buf: u32, buf_len: u32) -> Self {
        Self { mem, buf, buf_len }
    }

    fn copy_to_slice(&self, mut store: impl AsContextMut<Data = BSContext>, dest: &mut [u8]) {
        let len = min(self.buf_len as usize, dest.len());
        let from_mem_slice = self.mem.data(store.as_context_mut());
        let from_start = self.buf as usize;
        let from_end = from_start + len;
        let from = &from_mem_slice[from_start..from_end];
        dest.copy_from_slice(from);
    }

    fn copy_from(&self, mut store: impl AsContextMut<Data = BSContext>, other: &MemBuf) {
        let len = min(self.buf_len, other.buf_len) as usize;
        let mut temp = vec![0u8; len];
        other.copy_to_slice(store.as_context_mut(), &mut temp);
        self.copy_from_slice(store, &temp);
    }

    fn copy_from_slice(&self, mut store: impl AsContextMut<Data = BSContext>, from: &[u8]) {
        let to_mem_slice = self.mem.data_mut(store.as_context_mut());
        let len = min(self.buf_len as usize, from.len());
        let to_start = self.buf as usize;
        let to_end = to_start + len;
        let to = &mut to_mem_slice[to_start..to_end];
        to.copy_from_slice(&from[..len]);
    }
}

impl InstanceCaller {
    /// call the module function which registered
    async fn call<'a>(
        &self,
        mut store: impl AsContextMut<Data = BSContext>,
        param: &str,
        caller_mem: MemBuf<'a>,
    ) -> u32 {
        let mut result = McallError::None;
        let params_bs = param.as_bytes();
        let params_len = params_bs.len() as u32;
        let ptr = self
            .alloc
            .call_async(store.as_context_mut(), params_len)
            .await;
        let ptr = match ptr {
            Ok(ptr) => ptr,
            Err(_) => return McallError::AllocError.into(),
        };
        let caller_result_len = caller_mem.buf_len;
        let caller_result_ptr = self
            .alloc
            .call_async(store.as_context_mut(), caller_result_len)
            .await;
        let caller_result_ptr = match caller_result_ptr {
            Ok(ptr) => ptr,
            Err(_) => {
                let _ = self
                    .dealloc
                    .call_async(store.as_context_mut(), (ptr, params_len))
                    .await;
                return McallError::AllocError.into();
            }
        };
        let param_buf = MemBuf::new(&self.mem, ptr as u32, params_len);
        param_buf.copy_from_slice(store.as_context_mut(), &params_bs);
        let rs = self
            .func
            .call_async(
                store.as_context_mut(),
                (ptr, params_len, caller_result_ptr, caller_result_len),
            )
            .await;
        if rs.is_err() {
            result = McallError::MCallError.into();
        } else {
            let result_mem = MemBuf::new(&self.mem, caller_result_ptr as u32, caller_result_len);
            caller_mem.copy_from(store.as_context_mut(), &result_mem);
        }
        let rs = self
            .dealloc
            .call_async(store.as_context_mut(), (ptr, params_len))
            .await;
        if rs.is_err() {
            if let McallError::None = result {
                result = McallError::DeallocError;
            }
        }
        let rs = self
            .dealloc
            .call_async(
                store.as_context_mut(),
                (caller_result_ptr, caller_result_len),
            )
            .await;
        if rs.is_err() {
            if let McallError::None = result {
                result = McallError::DeallocError;
            }
        }
        result.into()
    }
}

struct RegisterReq {
    module: String,
    methods: Vec<String>,
}

fn process_register_req(module: &str, json_str: &str) -> anyhow::Result<RegisterReq> {
    let json_obj = json::parse(json_str)?;
    let module = module.to_string();
    let methods = json_obj["methods"]
        .members()
        .map(|m| m.to_string())
        .collect::<Vec<_>>();
    Ok(RegisterReq { module, methods })
}

struct ResponseErrorJson<'a> {
    mem: &'a Memory,
    store: StoreContextMut<'a, BSContext>,
    ptr: u32,
    len: u32,
}

impl<'a> ResponseErrorJson<'a> {
    fn new(mem: &'a Memory, store: StoreContextMut<'a, BSContext>, ptr: u32, len: u32) -> Self {
        Self {
            store,
            mem,
            ptr,
            len,
        }
    }
    fn error_json(msg: &str) -> String {
        let mut obj = json::object::Object::new();
        let code: JsonValue = json::number::Number::from(-1).into();
        obj["code"] = code;
        obj["message"] = msg.to_string().into();
        obj.dump()
    }

    fn response(&mut self, msg: &str) {
        let mem = self.mem.data_mut(self.store.as_context_mut());
        let start = self.ptr as usize;
        let json = Self::error_json(msg);
        let bs = json.as_bytes();
        let len = min(self.len as usize, bs.len());
        let end = start + len;
        let data = &mut mem[start..end];
        data.copy_from_slice(&bs[0..len]);
    }
}

pub(crate) struct ModuleLinker<'a> {
    linker: &'a mut Linker<BSContext>,
    store: &'a mut Store<BSContext>,
}

impl<'a> ModuleLinker<'a> {
    pub(crate) fn new(linker: &'a mut Linker<BSContext>, store: &'a mut Store<BSContext>) -> Self {
        Self { linker, store }
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

    /// async function for invoke mcall .
    #[inline]
    fn mcall_fn<'b>(
        mut caller: Caller<'b, BSContext>,
        addr: u32,
        addr_len: u32,
        buf: u32,
        buf_len: u32,
    ) -> Box<dyn Future<Output = u32> + Send + 'b> {
        Box::new(async move {
            if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
                let mem_slice = mem.data(caller.as_context());
                let start = addr as usize;
                let end = (addr + addr_len) as usize;
                let req_mem = &mem_slice[start..end];
                let json_str = unsafe { std::str::from_utf8_unchecked(req_mem) };
                macro_rules! responseError {
                    ($msg: literal) => {
                        ResponseErrorJson::new(&mem, caller.as_context_mut(), buf, buf_len)
                            .response($msg);
                        return McallError::Fail.into();
                    };
                    ($msg: expr) => {
                        ResponseErrorJson::new(&mem, caller.as_context_mut(), buf, buf_len)
                            .response($msg);
                        return McallError::Fail.into();
                    };
                }
                let (mcall_name, params) = match Self::parse_mcall(json_str) {
                    Ok((n, k)) => (n, k),
                    Err(e) => {
                        let emsg = format!("error parse json: {}", e.to_string());
                        responseError!(&emsg);
                    }
                };
                let ctx = INS_CTX.lock().await;
                let mcaller = ctx.module_caller.get(&mcall_name);
                let mcaller = if mcaller.is_none() {
                    responseError!("no mcall register.");
                } else {
                    mcaller.unwrap()
                };
                let dest_mem = MemBuf::new(&mem, buf, buf_len);
                return mcaller
                    .call(caller.as_context_mut(), &params, dest_mem)
                    .await;
            }
            McallError::MemoryNotFound.into()
        })
    }

    /// async function for register the mcall for modules.
    #[inline]
    fn register_fn<'b>(
        mut caller: Caller<'b, BSContext>,
        addr: u32,
        addr_len: u32,
        buf: u32,
        buf_len: u32,
    ) -> Box<dyn Future<Output = u32> + Send + 'b> {
        Box::new(async move {
            if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
                let mem_slice = mem.data(caller.as_context());
                let start = addr as usize;
                let end = (addr + addr_len) as usize;
                let req_mem = &mem_slice[start..end];
                let str = unsafe { std::str::from_utf8_unchecked(req_mem) };
                macro_rules! responseError {
                    ($msg: literal) => {
                        ResponseErrorJson::new(&mem, caller.as_context_mut(), buf, buf_len)
                            .response($msg);
                        return McallError::Fail.into();
                    };
                    ($msg: expr) => {
                        ResponseErrorJson::new(&mem, caller.as_context_mut(), buf, buf_len)
                            .response($msg);
                        return McallError::Fail.into();
                    };
                }
                let mem_ptr = mem_slice.as_ptr() as usize;
                let module = INS_CTX.lock().await.modules.get(&mem_ptr).map(String::from);
                let module = match module {
                    Some(m) => m,
                    None => return McallError::MCallMemoryNotFound.into(),
                };
                let req = match process_register_req(&module, str) {
                    Ok(req) => req,
                    Err(_) => {
                        responseError!("error parse json");
                    }
                };
                let mut ctx = INS_CTX.lock().await;
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
                        }
                    };
                    ctx.module_caller
                        .insert(format!("{}::{method}", &req.module), caller);
                }

                McallError::None.into()
            } else {
                McallError::MemoryNotFound.into()
            }
        })
    }

    /// export the ```blockless.mcall``` and ```blockless.register``` in the runtime.
    /// The modules can be use the register to register the moudle's function for mcall.
    pub(crate) async fn link_modules(&mut self) -> anyhow::Result<Module> {
        let mut modules: Vec<BlocklessModule> = {
            let preview1 = self
                .store
                .data()
                .preview1_ctx
                .as_ref()
                .ok_or(anyhow!("get preview1_ctx fail"))?;
            let lock = preview1.blockless_config.lock().unwrap();
            let cfg = lock.as_ref().ok_or(anyhow!("get the lock fail"))?;
            cfg.modules_ref().iter().map(|m| (*m).clone()).collect()
        };
        modules.sort_by(|a, b| a.module_type.partial_cmp(&b.module_type).unwrap());
        let mut entry = None;
        self.linker
            .func_wrap_async(
                "blockless",
                "mcall",
                |caller: Caller<'_, BSContext>,
                 (addr, addr_len, buf, buf_len): (u32, u32, u32, u32)| {
                    Self::mcall_fn(caller, addr, addr_len, buf, buf_len)
                },
            )?;
        self.linker
            .func_wrap_async(
                "blockless",
                "register",
                |caller: Caller<'_, BSContext>,
                 (addr, addr_len, buf, buf_len): (u32, u32, u32, u32)| {
                    Self::register_fn(caller, addr, addr_len, buf, buf_len)
                },
            )?;
        for m in modules {
            let (m_name, is_entry) = match m.module_type {
                ModuleType::Module => (m.name.as_str(), false),
                ModuleType::Entry => ("", true),
            };
            let module = Module::from_file(self.store.engine(), &m.file)?;
            if is_entry {
                entry = Some(module);
            } else {
                self.instance_module(m_name, &module).await?;
            }
        }
        entry.ok_or(anyhow!("can't find the entry"))
    }

    ///instance module and inital the context.
    async fn instance_module(&mut self, m_name: &str, module: &Module) -> anyhow::Result<()> {
        let instance = self
            .linker
            .instantiate_async(self.store.as_context_mut(), module)
            .await?;
        let mut initial = None;
        let mut funcs = HashMap::<String, Func>::new();
        let mut alloc = None;
        let mut dealloc = None;
        let mut mem = None;
        for export in instance.exports(self.store.as_context_mut()) {
            let name = export.name().to_string();
            if &name == "memory" {
                mem = export.into_memory();
                continue;
            }

            if let Some(func) = export.into_func() {
                match name.as_str() {
                    "_initialize" => {
                        initial = Some(func);
                    }
                    "alloc" => {
                        alloc = Some(func);
                    }
                    "dealloc" => {
                        dealloc = Some(func);
                    }
                    _ => {
                        funcs.insert(name, func);
                    }
                };
            }
        }

        let mem_ptr = mem.map(|m| m.data_ptr(self.store.as_context_mut()) as usize);
        if let Some(mem_ptr) = mem_ptr {
            INS_CTX
                .lock()
                .await
                .modules
                .insert(mem_ptr, m_name.to_string());
        }

        let alloc: Option<Arc<AllocTypedFunc>> = match alloc.map(|alloc| {
            alloc
                .typed(self.store.as_context_mut())
                .context("loading the alloc function")
        }) {
            Some(Ok(r)) => Some(Arc::new(r)),
            Some(Err(e)) => return Err(e),
            None => None,
        };

        let dealloc: Option<Arc<DeallocTypedFunc>> = match dealloc.map(|dealloc| {
            dealloc
                .typed(self.store.as_context_mut())
                .context("loading the dealloc function")
        }) {
            Some(Ok(r)) => Some(Arc::new(r)),
            Some(Err(e)) => return Err(e),
            None => None,
        };
        self.linker
            .instance(self.store.as_context_mut(), m_name, instance)?;
        let mod_info = InstanceInfo {
            alloc,
            export_funcs: funcs,
            dealloc,
            mem: mem,
        };
        //must release the lock, the initial method will access the modules.
        INS_CTX
            .lock()
            .await
            .instance_infos
            .insert(m_name.to_string(), mod_info);
        if let Some(func) = initial {
            let func = func
                .typed::<(), ()>(self.store.as_context())
                .context("loading the Reactor initialization function")?;
            func.call_async(self.store.as_context_mut(), ())
                .await
                .context("calling the Reactor initialization function")?;
        }
        Ok(())
    }
}
