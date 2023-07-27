mod modules;
mod error;
pub use error::*;
use blockless_drivers::{
    CdylibDriver, 
    DriverConetxt
};
use blockless_env;
use cap_std::ambient_authority;
use log::{debug, error};
use modules::link_modules;
use wasmtime::{
    Config, 
    PoolingAllocationConfig, 
    InstanceAllocationStrategy, 
    Store, 
    Trap, Engine, Linker, Module
};
use std::{env, path::Path};
pub use wasi_common::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

const ENTRY: &str = "_start";

pub struct ExitStatus {
    pub fuel: Option<u64>,
    pub code: i32,
}

pub async fn blockless_run(b_conf: BlocklessConfig) -> ExitStatus {
    let max_fuel = b_conf.get_limited_fuel();
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
        let mut allocation_config = PoolingAllocationConfig::default();
        allocation_config.instance_memory_pages(m);
        conf.allocation_strategy(InstanceAllocationStrategy::Pooling(allocation_config));
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
    blockless_env::add_socket_to_linker(&mut linker);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s).unwrap();
    let root_dir = b_conf.fs_root_path_ref()
        .and_then(|path| {
            wasmtime_wasi::Dir::open_ambient_dir(path, ambient_authority()).ok()
        });
    let mut builder = WasiCtxBuilder::new();
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
    let entry_module = b_conf.entry_module().unwrap();
    let mut args = vec![entry_module];
    args.extend_from_slice(&b_conf.stdin_args()[..]);
    builder = builder.args(&args[..]).unwrap();
    builder = builder.envs(&b_conf.envs()[..]).unwrap();
    if let Some(d) = root_dir {
        builder = builder.preopened_dir(d, "/").unwrap();
    }
    let mut ctx = builder.build();

    let drivers = b_conf.drivers_ref();
    load_driver(drivers);
    let fuel = b_conf.get_limited_fuel();
    let mut entry: String = b_conf.entry_ref().into();
    let version = b_conf.version();

    ctx.set_blockless_config(Some(b_conf));
    let mut store = Store::new(&engine, ctx);
    //set the fuel from the configure.
    if let Some(f) = fuel {
        let _ = store.add_fuel(f).map_err(|e| {
            error!("add fuel error: {}", e);
        });
    }

    let (module, entry) = match version {
        BlocklessConfigVersion::Version0 => {
            let module = Module::from_file(store.engine(), &entry).unwrap();
            (module, ENTRY.to_string())
        },
        BlocklessConfigVersion::Version1 => {
            if entry == "" {
                entry = ENTRY.to_string();
            }
            let module = link_modules(&mut linker, &mut store).await.unwrap();
            (module, entry)
        },
    };
    
    let inst = linker.instantiate_async(&mut store, &module).await.unwrap();
    let func = inst.get_typed_func::<(), ()>(&mut store, &entry).unwrap();
    let exit_code = match func.call_async(&mut store, ()).await {
        Err(ref t) => {
            error_process(t, || store.fuel_consumed().unwrap(), max_fuel)
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

fn load_driver(cfs: &[DriverConfig]) {
    cfs.iter().for_each(|cfg| {
        let drv = CdylibDriver::load(cfg.path(), cfg.schema()).unwrap();
        DriverConetxt::insert_driver(drv);
    });
}

fn error_process<F>(
    t: &anyhow::Error, 
    used_fuel: F,
    max_fuel: Option<u64>,
) -> i32 
where
    F: FnOnce() -> u64
{
    let trap_code_2_exit_code = |trap_code: &Trap| -> Option<i32> {
        match *trap_code {
            Trap::OutOfFuel => Some(1),
            Trap::StackOverflow => Some(2),
            Trap::MemoryOutOfBounds => Some(3),
            Trap::HeapMisaligned => Some(4),
            Trap::TableOutOfBounds => Some(5),
            Trap::IndirectCallToNull => Some(6),
            Trap::BadSignature => Some(7),
            Trap::IntegerOverflow => Some(8),
            Trap::IntegerDivisionByZero => Some(9),
            Trap::BadConversionToInteger => Some(10),
            Trap::UnreachableCodeReached => Some(11),
            Trap::Interrupt => Some(12),
            Trap::AlwaysTrapAdapter => Some(13),
            _ => None,
        }
    };
    let trap = t.downcast_ref::<Trap>();
    let rs = trap.and_then(|t| trap_code_2_exit_code(t)).unwrap_or(-1);
    match trap {
        Some(Trap::OutOfFuel) => {
            let used_fuel = used_fuel();
            let max_fuel = match max_fuel {
                Some(m) => m,
                None => 0,
            };
            error!(
                "All fuel is consumed, the app exited, fuel consumed {}, Max Fuel is {}.",
                used_fuel, max_fuel
            );
        }
        _ => error!("error: {}", t),
        
    };
    rs
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use super::*;
    use std::{fs, path::PathBuf};
    use tempdir::TempDir;
    use tokio::runtime::Builder;
    
    #[test]
    fn test_exit_code() {
        let err = Trap::OutOfFuel.into();
        let rs = error_process(&err, || 20u64, Some(30));
        assert_eq!(rs, 1);
    }

    fn run_blockless(config: BlocklessConfig) -> ExitStatus {
        let rt = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        
        rt.block_on(async {
            blockless_run(config).await
        })
    }

    fn simple_run_wat(temp_dir: &TempDir) -> PathBuf {
        let file_path = temp_dir.path().join("test_blockless_run.wasm");
        let code = r#"
        (module
            (func (export "_start")
            )
            (memory (export "memory") 1)
        )
        "#;
        fs::write(&file_path, code).unwrap();
        file_path
    }

    #[test]
    fn test_outof_fuel() {
        let temp = TempDir::new("blockless_run").unwrap();
        let file_path = simple_run_wat(&temp);
        let path = file_path.to_str().unwrap();
        let mut config =  BlocklessConfig::new(path);
        config.limited_fuel(Some(1));
        config.set_version(BlocklessConfigVersion::Version0);
        let code = run_blockless(config);
        assert_eq!(code.code, 1);
    }

    #[test]
    fn test_blockless_normal() {
        let temp = TempDir::new("blockless_run").unwrap();
        let file_path = simple_run_wat(&temp);
        let path = file_path.to_str().unwrap();
        let mut config =  BlocklessConfig::new(path);
        config.set_version(BlocklessConfigVersion::Version0);
        let code = run_blockless(config);
        assert_eq!(code.code, 0);
    }

    #[test]
    fn test_blockless_run_modules() {
        let temp_dir = TempDir::new("blockless_run").unwrap();
        let run_wasm = temp_dir.path().join("run.wasm");
        let code = r#"
        (module
            (import "module" "double" (func $double (param i32) (result i32)))
            (func (export "_start")
                i32.const 2
                call $double
                drop
            )
          )
        "#;
        let run_md5 = format!("{:x}", md5::compute(code));
        fs::write(&run_wasm, code).unwrap();
        let run_wasm_str = run_wasm.to_str().unwrap();
        
        let file_path = temp_dir.path().join("module.wasm");
        let code = r#"
        (module
            (func (export "double") (param i32) (result i32)
                local.get 0
                i32.const 2
                i32.mul
            )
            (memory (export "memory") 2)
          )
        "#;
        let module_md5 = format!("{:x}", md5::compute(code));
        fs::write(&file_path, code).unwrap();
        let module_wasm  = file_path.to_str().unwrap();
        let modules = vec![
            BlocklessModule { 
                module_type: ModuleType::Entry, 
                name: "".to_string(), 
                file: run_wasm_str.to_string(), 
                md5: run_md5 
            },
            BlocklessModule { 
                module_type: ModuleType::Module, 
                name: "module".to_string(), 
                file: module_wasm.to_string(), 
                md5: module_md5 
            },
        ];
        let mut config =  BlocklessConfig::new("_start");
        config.set_version(BlocklessConfigVersion::Version1);
        config.set_modules(modules);
        let code = run_blockless(config);
        assert_eq!(code.code, 0);
    }

}