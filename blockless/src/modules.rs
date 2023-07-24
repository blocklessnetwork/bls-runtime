use std::collections::HashMap;

use anyhow::Context;
use wasi_common::{
    WasiCtx, 
    BlocklessModule, 
    ModuleType
};
use wasmtime::{
    Linker, 
    Store, 
    Module, 
    Extern, 
    Caller, 
    AsContext,
    AsContextMut, Memory, TypedFunc, Func, 
};

struct ModuleInfo {
    mem: Memory,
    alloc: Option<TypedFunc<u32, u32>>,
    dealloc: Option<TypedFunc<u32, ()>>,
    funcs: HashMap<String, Func>,
}

pub(crate) async fn link_modules(linker: &mut Linker<WasiCtx>, store: &mut Store<WasiCtx>) -> Option<Module> {
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
                let mem = &mem_slice[(addr as usize)..((addr+addr_len) as usize)];
                unsafe {
                    let str = std::str::from_utf8_unchecked(mem);
                    println!("{str}")   
                }
            }
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
    for export in instance.exports(store.as_context_mut()) {
        let name = export.name().to_string();
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
        .typed::<i32, i32>(&mut store)
        .context("loading the alloc function")) {
        Some(Ok(r)) => Some(r),
        Some(Err(e)) => return Err(e),
        None => None,
    };

    let dealloc = match dealloc.map(|dealloc| dealloc
        .typed::<i32, ()>(&mut store)
        .context("loading the dealloc function")) {
        Some(Ok(r)) => Some(r),
        Some(Err(e)) => return Err(e),
        None => None,
    };
    
    linker.instance(store.as_context_mut(), m_name, instance)?;
    if let Some(func) = initial {
        let func = func
            .typed::<(), ()>(&mut store)
            .context("loading the Reactor initialization function")?;
        func.call_async(store.as_context_mut(), ())
            .await
            .context("calling the Reactor initialization function")?;
    }
    Ok(())
}