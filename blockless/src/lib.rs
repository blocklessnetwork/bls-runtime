use blockless_drivers::{CdylibDriver, DriverConetxt};
use blockless_env;
use log::{debug, error};
use std::{env, path::Path};
pub use wasi_common::*;
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

const ENTRY: &str = "_start";

pub struct ExitStatus {
    pub fuel: Option<u64>,
    pub code: i32,
}

pub async fn blockless_run(b_conf: BlocklessConfig) -> ExitStatus {
    //set the drivers root path, if not setting use exe file path.
    let drivers_root_path = b_conf
        .drivers_root_path_ref()
        .map(|p| p.into())
        .unwrap_or_else(|| {
            let mut current_exe_path = env::current_exe().unwrap();
            current_exe_path.pop();
            String::from(current_exe_path.to_str().unwrap())
        });
    DriverConetxt::init_built_in_drivers(drivers_root_path);

    let mut conf = Config::new();
    conf.debug_info(b_conf.get_debug_info());
    
    if let Some(_) = b_conf.get_limited_fuel() {
        //fuel is enable.
        conf.consume_fuel(true);
    }

    if let Some(m) = b_conf.get_limited_memory() {
        let mut instance_limits = InstanceLimits::default();
        instance_limits.memory_pages = m;
        let pool = InstanceAllocationStrategy::Pooling {
            strategy: PoolingAllocationStrategy::default(),
            instance_limits,
        };
        conf.allocation_strategy(pool);
    }
    conf.async_support(true);
    let engine = Engine::new(&conf).unwrap();
    let mut linker = Linker::new(&engine);
    blockless_env::add_drivers_to_linker(&mut linker);
    blockless_env::add_http_to_linker(&mut linker);
    blockless_env::add_ipfs_to_linker(&mut linker);
    blockless_env::add_s3_to_linker(&mut linker);
    blockless_env::add_memory_to_linker(&mut linker);
    blockless_env::add_cgi_to_linker(&mut linker);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s).unwrap();
    let root_dir = b_conf.fs_root_path_ref()
        .and_then(|path| {
            std::fs::File::open(path)
                .ok()
                .map(|path| wasmtime_wasi::Dir::from_std_file(path))
        });
    let mut builder = WasiCtxBuilder::new().inherit_args().unwrap();
    //stdout file process for setting.
    match b_conf.stdout_ref() {
        &Stdout::FileName(ref file_name) => {
            let mut is_set_stdout = false;
            if let Some(r) = b_conf.fs_root_path_ref() {
                let root = Path::new(r);
                let file_name = root.join(file_name);
                let mut file_opts = std::fs::File::options();
                file_opts.create(true);
                file_opts.append(true);
                file_opts.write(true);

                if let Some(f) = file_opts.open(file_name).ok().map(|file| {
                    let file = cap_std::fs::File::from_std(file);
                    let f = wasmtime_wasi::file::File::from_cap_std(file);
                    Box::new(f)
                }) {
                    is_set_stdout = true;
                    builder = builder.stdout(f)
                }
            }
            if !is_set_stdout {
                builder = builder.inherit_stdout();
            }
        }
        &Stdout::Inherit => {
            builder = builder.inherit_stdout();
        }
        &Stdout::Null => {}
    }
    if let Some(d) = root_dir {
        builder = builder.preopened_dir(d, "/").unwrap();
    }
    let mut ctx = builder.build();

    let drivers = b_conf.drivers_ref();
    load_driver(drivers);
    let fuel = b_conf.get_limited_fuel();
    let wasm_file: String = b_conf.wasm_file_ref().into();
    let is_carfile = b_conf.is_carfile();
    ctx.blockless_config = Some(b_conf);

    let mut store = Store::new(&engine, ctx);
    //set the fuel from the configure.
    if let Some(f) = fuel {
        let _ = store.add_fuel(f).map_err(|e| {
            error!("add fuel error: {}", e);
        });
    }
    
    let inst = if is_carfile {
        let module = link_modules(&mut linker, &mut store).await.unwrap();
        linker.instantiate_async(&mut store, &module).await.unwrap()
    } else {
        // Instantiate our module with the imports we've created, and run it.
        let module = Module::from_file(&engine, &wasm_file).unwrap();
        linker.module_async(&mut store, "", &module).await.unwrap();
        linker.instantiate_async(&mut store, &module).await.unwrap()
    };
    
    let func = inst.get_typed_func::<(), (), _>(&mut store, ENTRY).unwrap();
    let exit_code = match func.call_async(&mut store, ()).await {
        Err(ref t) => {
            let fuel = fuel.unwrap_or(0);
            trap_info(t, store.fuel_consumed(), fuel);
            t.i32_exit_status().unwrap_or(-1)
        }
        Ok(_) => {
            debug!("program exit normal.");
            0
        }
    };
    ExitStatus {
        fuel: store.fuel_consumed(),
        code: exit_code,
    }
}


async fn link_modules(linker: &mut Linker<WasiCtx>, store: &mut Store<WasiCtx>) -> Option<Module> {
    let cfg = store.data().blockless_config.as_ref().unwrap();
    let mut modules: Vec<BlocklessModule> = cfg.modules_ref().iter().map(|m| (*m).clone()).collect();
    modules.sort_by(|a, b| a.module_type.partial_cmp(&b.module_type).unwrap());
    // let modus = modules.iter()
    //     .filter(|m| matches!(m.module_type, ModuleType::Module))
    //     .chain(
    //         modules.iter()
    //             .filter(|m| matches!(m.module_type, ModuleType::Entry))
    //     );
    let mut entry = None;
    for m in modules {
        let (m_name, is_entry) = match m.module_type {
            ModuleType::Module => (m.name.as_str(), false),
            ModuleType::Entry => ("", true),
        };
        let module = Module::from_file(store.engine(), &m.file).unwrap();
        linker.module_async(store.as_context_mut(), m_name, &module).await.unwrap();
        if is_entry {
            entry = Some(module);
        }
    }
    entry
}

fn load_driver(cfs: &[DriverConfig]) {
    cfs.iter().for_each(|cfg| {
        let drv = CdylibDriver::load(cfg.path(), cfg.schema()).unwrap();
        DriverConetxt::insert_driver(drv);
    });
}

fn trap_info(t: &Trap, fuel: Option<u64>, max_fuel: u64) {
    if let Some(fuel) = fuel {
        if fuel >= max_fuel {
            error!(
                "All fuel is consumed, the app exited, fuel consumed {}, Max Fuel is {}.",
                fuel, max_fuel
            );
        } else {
            error!("Fuel {}:{}. {}", fuel, max_fuel, t);
        }
    } else {
        error!("error: {}", t);
    }
}
