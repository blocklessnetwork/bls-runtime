mod modules;
mod context;
pub mod error;

use context::BlocklessContext;
use blockless_drivers::{
    CdylibDriver, 
    DriverConetxt
};
use blockless_env;
use cap_std::ambient_authority;
pub use error::*;
use log::{debug, error};
use modules::ModuleLinker;
use wasmtime_wasi_threads::WasiThreadsCtx;
use std::{env, path::Path, sync::Arc};
pub use wasi_common::*;
use wasmtime::{
    Config, 
    Engine,
    PoolingAllocationConfig, 
    InstanceAllocationStrategy, 
    Linker, 
    Module,
    Store,
    Trap,
};
use wasmtime_wasi::sync::WasiCtxBuilder;

const ENTRY: &str = "_start";

pub struct ExitStatus {
    pub fuel: Option<u64>,
    pub code: i32,
}

trait BlocklessConfig2Preview1WasiBuilder {
    fn preview1_builder(&self) -> WasiCtxBuilder;
    fn preview1_set_stdouterr(&self, builder: WasiCtxBuilder, is_err: bool) -> WasiCtxBuilder;
    fn preview1_engine_config(&self) -> Config;
}

impl BlocklessConfig2Preview1WasiBuilder for BlocklessConfig {
    fn preview1_set_stdouterr(&self, mut builder: WasiCtxBuilder, is_err: bool) -> WasiCtxBuilder {
        let b_conf = self;
        match b_conf.stdout_ref() {
            &Stdout::FileName(ref file_name) => {
                let mut is_set_fileout = false;
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
                        is_set_fileout = true;
                        if is_err {
                            builder.stdout(f);
                        } else {
                            builder.stderr(f);
                        }
                    }
                }
                if !is_set_fileout {
                    if is_err {
                        builder.inherit_stdout();
                    } else {
                        builder.inherit_stderr();
                    }
                }
            }
            &Stdout::Inherit => {
                if is_err {
                    builder.inherit_stdout();
                } else {
                    builder.inherit_stderr();
                }
            }
            &Stdout::Null => {}
        }
        builder
    }

    fn preview1_builder(&self) -> WasiCtxBuilder {
        let b_conf = self;
        let root_dir = b_conf
            .fs_root_path_ref()
            .and_then(|path| wasmtime_wasi::Dir::open_ambient_dir(path, ambient_authority()).ok());
        let mut builder = WasiCtxBuilder::new();
        //stdout file process for setting.
        builder = b_conf.preview1_set_stdouterr(builder, false);
        builder = b_conf.preview1_set_stdouterr(builder, true);
        let entry_module = b_conf.entry_module().unwrap();
        let mut args = vec![entry_module];
        args.extend_from_slice(&b_conf.stdin_args_ref()[..]);
        builder.args(&args[..]).unwrap();
        builder.envs(&b_conf.envs_ref()[..]).unwrap();
        if let Some(d) = root_dir {
            builder.preopened_dir(d, "/").unwrap();
        }
        builder
    }

    fn preview1_engine_config(&self) -> Config {
        let mut conf = Config::new();

        conf.debug_info(self.get_debug_info());

        if let Some(_) = self.get_limited_fuel() {
            //fuel is enable.
            conf.consume_fuel(true);
        }

        if let Some(m) = self.get_limited_memory() {
            let mut allocation_config = PoolingAllocationConfig::default();
            allocation_config.memory_pages(m);
            conf.allocation_strategy(InstanceAllocationStrategy::Pooling(allocation_config));
        }
        
        if self.feature_thread() {
            conf.wasm_threads(true);
        } else {
            conf.async_support(true);
        }
        conf
    }
}

struct BlocklessRunner(BlocklessConfig);

impl BlocklessRunner {
    async fn run(self) -> ExitStatus {
        let b_conf = self.0;
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

        let conf = b_conf.preview1_engine_config();
        let engine = Engine::new(&conf).unwrap();
        let mut linker = Linker::new(&engine);
        let support_thread = b_conf.feature_thread();
        let mut builder = b_conf.preview1_builder();
        let mut preview1_ctx = builder.build();
        let drivers = b_conf.drivers_ref();
        Self::load_driver(drivers);
        let entry: String = b_conf.entry_ref().into();
        let version = b_conf.version();
        preview1_ctx.set_blockless_config(Some(b_conf));
        let mut ctx = BlocklessContext::default();
        ctx.preview1_ctx = Some(preview1_ctx);
        let mut store = Store::new(&engine, ctx);
        Self::preview1_setup_linker(&mut linker);
        let (module, entry) = Self::module_linker(version, entry, &mut store, &mut linker).await;
        // support thread.
        if support_thread {
            Self::preview1_setup_thread_support(&mut linker, &mut store, &module);
        }
        let inst = if support_thread {
            linker.instantiate(&mut store, &module).unwrap()
        } else {
            linker.instantiate_async(&mut store, &module).await.unwrap()
        };
        let func = inst.get_typed_func::<(), ()>(&mut store, &entry).unwrap();
        let result = if support_thread {
            func.call(&mut store, ())
        } else {
            func.call_async(&mut store, ()).await
        };
        let exit_code = match result {
            Err(ref t) => Self::error_process(t, || store.fuel_consumed().unwrap(), max_fuel),
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

    /// setup functions the preview1 for linker
    fn preview1_setup_linker(
        linker: &mut Linker<BlocklessContext>, 
    ) {
        // define the macro of extends.
        macro_rules! add_to_linker {
            ($method:expr) => {
                $method(linker, |s|  s.preview1_ctx.as_mut().unwrap()).unwrap()
            };
        }
        add_to_linker!(blockless_env::add_drivers_to_linker);
        add_to_linker!(blockless_env::add_http_to_linker);
        add_to_linker!(blockless_env::add_ipfs_to_linker);
        add_to_linker!(blockless_env::add_s3_to_linker);
        add_to_linker!(blockless_env::add_memory_to_linker);
        add_to_linker!(blockless_env::add_cgi_to_linker);
        add_to_linker!(blockless_env::add_socket_to_linker);
        add_to_linker!(wasmtime_wasi::add_to_linker);
    }

    fn preview1_setup_thread_support(
        linker: &mut Linker<BlocklessContext>,
        store: &mut Store<BlocklessContext>,
        module: &Module,
    ) {
        wasmtime_wasi_threads::add_to_linker(linker, store, &module, |ctx| {
            ctx.wasi_threads.as_ref().unwrap()
        }).unwrap();
        store.data_mut().wasi_threads = Some(Arc::new(
            WasiThreadsCtx::new(
                module.clone(), 
                Arc::new(linker.clone())
            ).unwrap()
        ));
    }



    async fn module_linker<'a>(
        version: BlocklessConfigVersion,
        mut entry: String,
        store: &'a mut Store<BlocklessContext>,
        linker: &'a mut Linker<BlocklessContext>
    ) -> (Module, String) {
        match version {
            // this is older configure for bls-runtime, this only run single wasm.
            BlocklessConfigVersion::Version0 => {
                let module = Module::from_file(store.engine(), &entry).unwrap();
                (module, ENTRY.to_string())
            }
            BlocklessConfigVersion::Version1 => {
                if entry == "" {
                    entry = ENTRY.to_string();
                }
                let mut module_linker = ModuleLinker::new(linker, store);
                let module = module_linker.link_modules().await.unwrap();
                (module, entry)
            }
        }
    }

    fn load_driver(cfs: &[DriverConfig]) {
        cfs.iter().for_each(|cfg| {
            let drv = CdylibDriver::load(cfg.path(), cfg.schema()).unwrap();
            DriverConetxt::insert_driver(drv);
        });
    }

    fn error_process<F>(t: &anyhow::Error, used_fuel: F, max_fuel: Option<u64>) -> i32
    where
        F: FnOnce() -> u64,
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
}

pub async fn blockless_run(b_conf: BlocklessConfig) -> ExitStatus {
    BlocklessRunner(b_conf).run().await
}


#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use super::*;
    
    //inner test
    #[test]
    fn test_exit_code() {
        let err = Trap::OutOfFuel.into();
        let rs = BlocklessRunner::error_process(&err, || 20u64, Some(30));
        assert_eq!(rs, 1);
    }

}
