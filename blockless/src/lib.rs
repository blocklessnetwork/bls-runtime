mod config;
use blockless_drivers::{CdylibDriver, DriverConetxt};
use blockless_env;
pub use config::Stdout;
use log::{error, info};
use std::{path::Path, env};
use wasmtime::*;
use wasmtime_wasi::{sync::WasiCtxBuilder, WasiCtx};

pub use config::{BlocklessConfig, DriverConfig};

const ENTRY: &str = "_start";

pub async fn blockless_run(b_conf: BlocklessConfig) {
    
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
    conf.async_support(true);
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

    let engine = Engine::new(&conf).unwrap();
    let mut linker = Linker::new(&engine);
    blockless_env::add_drivers_to_linker(&mut linker);
    blockless_env::add_http_to_linker(&mut linker);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s).unwrap();
    let root_dir = b_conf
        .fs_root_path_ref()
        .map(|path| {
            std::fs::File::open(path)
                .ok()
                .map(|path| wasmtime_wasi::Dir::from_std_file(path))
        })
        .flatten();
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
        Stdout::Null => {}
    }
    if let Some(d) = root_dir {
        builder = builder.preopened_dir(d, "/").unwrap();
    }
    let ctx = builder.build();
    let drivers = b_conf.drivers_ref();
    load_driver(&ctx, drivers);
    let mut store = Store::new(&engine, ctx);
    //set the fuel from the configure.
    if let Some(f) = b_conf.get_limited_fuel() {
        let _ = store.add_fuel(f).map_err(|e| {
            error!("add fuel error: {}", e);
        });
    }

    // Instantiate our module with the imports we've created, and run it.
    let module = Module::from_file(&engine, b_conf.wasm_file_ref()).unwrap();
    linker.module(&mut store, "", &module).unwrap();
    let inst = linker.instantiate_async(&mut store, &module).await.unwrap();
    let func = inst.get_typed_func::<(), (), _>(&mut store, ENTRY).unwrap();
    match func.call_async(&mut store, ()).await {
        Err(ref t) => trap_info(t, store.fuel_consumed(), b_conf.get_limited_fuel().unwrap()),
        Ok(_) => info!("program exit normal."),
    }
}

fn load_driver(ctx: &WasiCtx, cfs: &[DriverConfig]) {
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
