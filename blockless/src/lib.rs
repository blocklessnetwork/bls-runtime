mod context;
pub mod error;
mod modules;

use blockless_drivers::{CdylibDriver, DriverConetxt};
use blockless_env;
pub use blockless_multiaddr::MultiAddr;
use cap_std::ambient_authority;
use context::BlocklessContext;
pub use error::*;
use log::{debug, error};
use modules::ModuleLinker;
use std::{env, path::{Path, PathBuf}, sync::Arc};
use wasi_common::sync::WasiCtxBuilder;
pub use wasi_common::*;
use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, Linker, Module, PoolingAllocationConfig, Store,
    Trap,
};
use wasmtime_wasi_threads::WasiThreadsCtx;

// the default wasm entry name.
const ENTRY: &str = "_start";

pub struct ExitStatus {
    pub fuel: Option<u64>,
    pub code: i32,
}

trait BlocklessConfig2Preview1WasiBuilder {
    fn preview1_builder(&self) -> WasiCtxBuilder;
    fn preview1_set_stdio(&self, builder: &mut WasiCtxBuilder);
    fn preview1_engine_config(&self) -> Config;
}

impl BlocklessConfig2Preview1WasiBuilder for BlocklessConfig {
    /// set the stdout and stderr for the wasm.
    /// the stdout adn stderr can be setting to file or inherit the stdout and stderr.
    fn preview1_set_stdio(&self, builder: &mut WasiCtxBuilder) {
        let b_conf = self;
        macro_rules! process_output {
            ($out_ref: expr, $out_expr: tt, $stdout: tt, $inherit_stdout: tt) => {
                //$out_ref is b_conf.stdout_ref() or b_conf.stderr_ref()
                match $out_ref {
                    &$out_expr::FileName(ref file_name) => {
                        let mut is_set_fileout = false;
                        if let Some(r) = b_conf.fs_root_path_ref() {
                            let root = Path::new(r);
                            let file_name = root.join(file_name);
                            if let Some(f) = {
                                let mut file_opts = std::fs::File::options();
                                file_opts.create(true);
                                file_opts.append(true);
                                file_opts.write(true);
                                file_opts.open(file_name).ok().map(|file| {
                                    let file = cap_std::fs::File::from_std(file);
                                    let f = wasi_common::sync::file::File::from_cap_std(file);
                                    Box::new(f)
                                })
                            } {
                                is_set_fileout = true;
                                //builder.stdout() or builder.stderr()
                                builder.$stdout(f);
                            }
                        }
                        if !is_set_fileout {
                            //$inherit_stdout is inherit_stdout() or inherit_stderr()
                            builder.$inherit_stdout();
                        }
                    }
                    &$out_expr::Inherit => {
                        builder.$inherit_stdout();
                    }
                    &$out_expr::Null => {}
                }
            };
        }
        process_output!(b_conf.stdout_ref(), Stdout, stdout, inherit_stdout);
        process_output!(b_conf.stderr_ref(), Stderr, stderr, inherit_stderr);
    }

    /// create the preview1_builder by the configure.
    fn preview1_builder(&self) -> WasiCtxBuilder {
        let b_conf = self;
        let root_dir = b_conf.fs_root_path_ref().and_then(|path| {
            wasi_common::sync::Dir::open_ambient_dir(path, ambient_authority()).ok()
        });
        let mut builder = WasiCtxBuilder::new();
        //stdout file process for setting.
        b_conf.preview1_set_stdio(&mut builder);
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

    /// convert the blockless configure  to wasmtime configure.
    fn preview1_engine_config(&self) -> Config {
        let mut conf = Config::new();

        conf.debug_info(self.get_debug_info());

        if let Some(_) = self.get_limited_fuel() {
            //fuel is enable.
            conf.consume_fuel(true);
        }

        if let Some(m) = self.get_limited_memory() {
            let mut allocation_config = PoolingAllocationConfig::default();
            allocation_config.max_memory_size(m as _);
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
    /// blockless run method, it execute the wasm program with configure file.
    async fn run(self) -> ExitStatus {
        let b_conf = self.0;
        let max_fuel = b_conf.get_limited_fuel();
        // set the drivers root path, if not setting use exe file path.
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
        let mut linker = wasmtime::Linker::new(&engine);
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
        Self::preview1_setup_linker(&mut linker, support_thread);
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
        // if thread multi thread use sync model.
        // The multi-thread model is used for the cpu intensive program.
        let result = if support_thread {
            func.call(&mut store, ())
        } else {
            func.call_async(&mut store, ()).await
        };
        let exit_code = match result {
            Err(ref t) => Self::error_process(t, || store.get_fuel().unwrap(), max_fuel),
            Ok(_) => {
                debug!("program exit normal.");
                0
            }
        };
        ExitStatus {
            fuel: store.get_fuel().ok(),
            code: exit_code,
        }
    }

    fn preview1_setup_linker(linker: &mut Linker<BlocklessContext>, support_thread: bool) {
        if !support_thread {
            // define the macro of extends.
            macro_rules! add_to_linker {
                ($method:expr) => {
                    $method(linker, |s| s.preview1_ctx.as_mut().unwrap()).unwrap()
                };
            }
            add_to_linker!(blockless_env::add_drivers_to_linker);
            add_to_linker!(blockless_env::add_http_to_linker);
            add_to_linker!(blockless_env::add_ipfs_to_linker);
            add_to_linker!(blockless_env::add_s3_to_linker);
            add_to_linker!(blockless_env::add_memory_to_linker);
            add_to_linker!(blockless_env::add_cgi_to_linker);
            add_to_linker!(blockless_env::add_socket_to_linker);
        }
        wasi_common::sync::add_to_linker(linker, |host| host.preview1_ctx.as_mut().unwrap())
            .unwrap();
    }

    fn preview1_setup_thread_support(
        linker: &mut Linker<BlocklessContext>,
        store: &mut Store<BlocklessContext>,
        module: &Module,
    ) {
        wasmtime_wasi_threads::add_to_linker(linker, store, &module, |ctx| {
            ctx.wasi_threads.as_ref().unwrap()
        })
        .unwrap();
        store.data_mut().wasi_threads = Some(Arc::new(
            WasiThreadsCtx::new(module.clone(), Arc::new(linker.clone()))
                .expect("wasi thread ctx new fail."),
        ));
    }

    async fn module_linker<'a>(
        version: BlocklessConfigVersion,
        mut entry: String,
        store: &'a mut Store<BlocklessContext>,
        linker: &'a mut Linker<BlocklessContext>,
    ) -> (Module, String) {
        match version {
            // this is older configure for bls-runtime, this only run single wasm.
            BlocklessConfigVersion::Version0 => {
                let module = Module::from_file(store.engine(), &entry).unwrap();
                (module, ENTRY.to_string())
            }
            BlocklessConfigVersion::Version1 => {
                if entry.is_empty() {
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

    /// the error code process.
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
