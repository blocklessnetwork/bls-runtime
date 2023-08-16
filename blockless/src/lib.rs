mod modules;
pub mod error;
pub use error::*;
use blockless_drivers::{
    CdylibDriver, 
    DriverConetxt
};
use blockless_env;
use cap_std::ambient_authority;
use log::{debug, error};
use modules::ModuleLinker;
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

trait BlocklessConfig2WasiBuilder {
    fn to_builder(&self) -> WasiCtxBuilder;
    fn set_stdouterr(&self, builder: WasiCtxBuilder, is_err: bool) -> WasiCtxBuilder;
}


impl BlocklessConfig2WasiBuilder for BlocklessConfig {

    fn set_stdouterr(
        &self, 
        mut builder: WasiCtxBuilder, 
        is_err: bool,
    ) -> WasiCtxBuilder 
    {
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
                        builder = if is_err {
                            builder.stdout(f)
                        } else {
                            builder.stderr(f)
                        }
                    }
                }
                if !is_set_fileout {
                    builder = if is_err { 
                        builder.inherit_stdout()
                    } else {
                        builder.inherit_stderr()
                    }
                }
            }
            &Stdout::Inherit => {
                builder = if is_err { 
                    builder.inherit_stdout()
                } else {
                    builder.inherit_stderr()
                }
            }
            &Stdout::Null => {}
        }
        builder
    }

    fn to_builder(&self) -> WasiCtxBuilder {
        let b_conf = self;
        let root_dir = b_conf.fs_root_path_ref()
        .and_then(|path| {
            wasmtime_wasi::Dir::open_ambient_dir(path, ambient_authority()).ok()
        });
        let mut builder = WasiCtxBuilder::new();
        //stdout file process for setting.
        builder = b_conf.set_stdouterr(builder, false);
        builder = b_conf.set_stdouterr(builder, true);
        let entry_module = b_conf.entry_module().unwrap();
        let mut args = vec![entry_module];
        args.extend_from_slice(&b_conf.stdin_args_ref()[..]);
        builder = builder.args(&args[..]).unwrap();
        builder = builder.envs(&b_conf.envs_ref()[..]).unwrap();
        if let Some(d) = root_dir {
            builder = builder.preopened_dir(d, "/").unwrap();
        }
        builder
    }
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
    add_host_modules_to_linker(&mut linker);
    blockless_env::add_drivers_to_linker(&mut linker);
    blockless_env::add_http_to_linker(&mut linker);
    blockless_env::add_ipfs_to_linker(&mut linker);
    blockless_env::add_s3_to_linker(&mut linker);
    blockless_env::add_memory_to_linker(&mut linker);
    blockless_env::add_cgi_to_linker(&mut linker);
    blockless_env::add_socket_to_linker(&mut linker);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s).unwrap();
    let builder = b_conf.to_builder();
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
            let mut module_linker = ModuleLinker::new(&mut linker, &mut store);
            let module = module_linker.link_modules().await.unwrap();
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

fn add_host_modules_to_linker(linker: &mut Linker<WasiCtx>) {
    // (i32, i32, i32, i32) -> (i32)
    linker.func_new_async(
        "blockless",
        "module",
        wasmtime::FuncType::new([wasmtime::ValType::I32; 4], [wasmtime::ValType::I32]),
        |mut caller: wasmtime::Caller<'_, WasiCtx>, params: &[wasmtime::Val], results: &mut [wasmtime::Val]| {
            Box::new(async move {
                results[0] = wasmtime::Val::from(1); // store non-zero exit code

                println!("blockless module called.");
                println!("params: {:?}", params);
                let (call_ptr, call_ptr_len, result_ptr, result_ptr_len) = {
                    (params[0].unwrap_i32() as usize, params[1].unwrap_i32() as usize, params[2].unwrap_i32() as usize, params[3].unwrap_i32() as usize)
                };
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .ok_or(anyhow::anyhow!("failed to find host memory"))?;
        
                let param_bytes = memory.data(&caller)
                    .get(call_ptr..)
                    .and_then(|arr| arr.get(..call_ptr_len))
                    .ok_or(anyhow::anyhow!("pointer/length out of bounds"))?;
                let param_str = std::str::from_utf8(param_bytes).map_err(|e| anyhow::anyhow!("invalid utf-8: {}", e))?;
                println!("param_str: {:?}", param_str);

                let module_call: ModuleCallType = param_str.to_string().try_into().unwrap();
                match module_call {
                    ModuleCallType::HTTP(url, opts) => {
                        let (fd, code) = blockless_drivers::http_driver::http_req(&url, &opts).await.unwrap();
                        println!("fd: {}; code: {}", fd, code);
                        let mut dest_buf = vec![0u8; result_ptr_len];
                        let _bytes_read: u32 = blockless_drivers::http_driver::http_read_body(fd.into(), &mut dest_buf[..]).await.unwrap();

                        // write dest_buf to wasm memory
                        memory.data_mut(&mut caller)
                            .get_mut(result_ptr..)
                            .and_then(|arr| arr.get_mut(..result_ptr_len))
                            .ok_or(anyhow::anyhow!("pointer/length out of bounds"))?
                            .copy_from_slice(&dest_buf);
                    },
                }
                results[0] = wasmtime::Val::from(0); // set zero exit code (success)
                Ok(())
            })
        },
    )
    .unwrap(); // TODO: handle error
}


pub enum ModuleCallType {
    HTTP(String, String),
    // IPFS(IPFSOpts),
    // TODO: other BLS extensions..
}

// TODO: error handling
// path {"module":"blockless::http_req", "params": {"url": "https://jsonplaceholder.typicode.com/todos/1", "opts": {"method":"GET","connectTimeout":30,"readTimeout":10,"headers":"{}","body":null}}}, opts 
impl TryFrom<String> for ModuleCallType {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let v: serde_json::Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
        let module = v["module"].as_str().unwrap(); // TODO propagate up error
        let params = v["params"].as_object().unwrap(); // TODO propagate up error
        match module {
            "blockless::http_req" => {
                // TODO: permission validation
                println!("params: {}", params["opts"]);
                let url = params["url"].as_str().unwrap().to_string();
                let params = params["opts"].to_string();
                Ok(ModuleCallType::HTTP(url, params))
            }
            _ => Err("unknown module".to_string()),
        }
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
    use std::fs;
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

    #[test]
    fn test_outof_fuel() {
        let temp_dir = TempDir::new("blockless_run").unwrap();
        let file_path = temp_dir.path().join("test_blockless_run.wasm");
        let code = r#"
        (module
            (func (export "_start"))
            (memory (export "memory") 1)
        )
        "#;
        fs::write(&file_path, code).unwrap();
        let path = file_path.to_str().unwrap();
        let mut config =  BlocklessConfig::new(path);
        config.limited_fuel(Some(1));
        config.set_version(BlocklessConfigVersion::Version0);
        let code = run_blockless(config);
        assert_eq!(code.code, 1);
    }

    #[test]
    fn test_blockless_normal() {
        let temp_dir = TempDir::new("blockless_run").unwrap();
        let file_path = temp_dir.path().join("test_blockless_run.wasm");
        let code = r#"
        (module
            (func (export "_start"))
            (memory (export "memory") 1)
        )
        "#;
        fs::write(&file_path, code).unwrap();
        let path = file_path.to_str().unwrap();
        let mut config =  BlocklessConfig::new(path);
        config.set_version(BlocklessConfigVersion::Version0);
        let code = run_blockless(config);
        assert_eq!(code.code, 0);
    }

    #[test]
    fn test_blockless_run_primary_module_can_call_reactor_module() {
        let primary_code = r#"
        (module
            (import "reactor1" "double" (func $double (param i32) (result i32)))
            (func (export "_start")
                i32.const 2
                call $double
                drop
            )
        )
        "#;
        let reactor_1_code = r#"
        (module
            (func (export "double") (param i32) (result i32)
                local.get 0
                i32.const 2
                i32.mul
            )
        )
        "#;
        let temp_dir = TempDir::new("blockless_run").unwrap();

        let primary_path = temp_dir.path().join("run.wasm");
        let reactor_1_path = temp_dir.path().join("reactor1.wasm");

        fs::write(&primary_path, primary_code).unwrap();
        fs::write(&reactor_1_path, reactor_1_code).unwrap();

        let modules = vec![
            BlocklessModule { 
                module_type: ModuleType::Entry, 
                name: "".to_string(), 
                file: primary_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(primary_code)), 
            },
            BlocklessModule { 
                module_type: ModuleType::Module, 
                name: "reactor1".to_string(), 
                file: reactor_1_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(reactor_1_code)) 
            },
        ];
        let mut config =  BlocklessConfig::new("_start");
        config.set_version(BlocklessConfigVersion::Version1);
        config.set_modules(modules);
        let code = run_blockless(config);
        assert_eq!(code.code, 0);
    }

    #[test]
    fn test_blockless_primary_module_can_call_multiple_reactor_modules() {
        let primary_code = r#"
        (module
            (import "reactor1" "double1" (func $double1 (param i32) (result i32)))
            (import "reactor2" "double2" (func $double2 (param i32) (result i32)))
            (func (export "_start")
                i32.const 2
                call $double1
                drop
            
                i32.const 4
                call $double2
                drop
            )
        )
        "#;
        let reactor_1_code = r#"
        (module
            (func (export "double1") (param i32) (result i32)
                local.get 0
                i32.const 2
                i32.mul
            )
        )
        "#;
        let reactor_2_code = r#"
        (module
            (func (export "double2") (param i32) (result i32)
                local.get 0
                i32.const 2
                i32.mul
            )
        )
        "#;

        let temp_dir = TempDir::new("blockless_run").unwrap();

        let primary_path = temp_dir.path().join("run.wasm");
        let reactor_1_path = temp_dir.path().join("reactor1.wasm");
        let reactor_2_path = temp_dir.path().join("reactor2.wasm");
        
        fs::write(&primary_path, primary_code).unwrap();
        fs::write(&reactor_1_path, reactor_1_code).unwrap();
        fs::write(&reactor_2_path, reactor_2_code).unwrap();
        
        let modules = vec![
            BlocklessModule { 
                module_type: ModuleType::Entry, 
                name: "".to_string(), 
                file: primary_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(primary_code)), 
            },
            BlocklessModule { 
                module_type: ModuleType::Module, 
                name: "reactor1".to_string(), 
                file: reactor_1_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(reactor_1_code)) 
            },
            BlocklessModule { 
                module_type: ModuleType::Module, 
                name: "reactor2".to_string(), 
                file: reactor_2_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(reactor_2_code)) 
            },
        ];
        let mut config =  BlocklessConfig::new("_start");
        config.set_version(BlocklessConfigVersion::Version1);
        config.set_modules(modules);
        let code = run_blockless(config);
        assert_eq!(code.code, 0);
    }

    #[test]
    fn test_blockless_reactor_module_can_call_reactor_module() {
        let primary_code = r#"
        (module
            (import "reactor1" "double1" (func $double1 (param i32) (result i32)))
            (func (export "_start")
                i32.const 2
                call $double1
                drop
            )
        )
        "#;
        let reactor_1_code = r#"
        (module
            (import "reactor2" "double2" (func $double2 (param i32) (result i32)))
            (func (export "double1") (param i32) (result i32)
                local.get 0
                call $double2
            )
        )
        "#;
        let reactor_2_code = r#"
        (module
            (func $double2 (export "double2") (param i32) (result i32)
                local.get 0
                i32.const 2
                i32.mul
            )
        )
        "#;

        let temp_dir = TempDir::new("blockless_run").unwrap();

        let primary_path = temp_dir.path().join("run.wasm");
        let reactor_1_path = temp_dir.path().join("reactor1.wasm");
        let reactor_2_path = temp_dir.path().join("reactor2.wasm");
        
        fs::write(&primary_path, primary_code).unwrap();
        fs::write(&reactor_1_path, reactor_1_code).unwrap();
        fs::write(&reactor_2_path, reactor_2_code).unwrap();
        
        let modules = vec![
            BlocklessModule { 
                module_type: ModuleType::Entry, 
                name: "".to_string(), 
                file: primary_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(primary_code)), 
            },
            // ensure we load/link reactor2 before reactor1 since reactor1 depends on it
            BlocklessModule { 
                module_type: ModuleType::Module, 
                name: "reactor2".to_string(), 
                file: reactor_2_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(reactor_2_code)) 
            },
            BlocklessModule { 
                module_type: ModuleType::Module, 
                name: "reactor1".to_string(), 
                file: reactor_1_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(reactor_1_code)) 
            },
        ];
        let mut config =  BlocklessConfig::new("_start");
        config.set_version(BlocklessConfigVersion::Version1);
        config.set_modules(modules);
        let code = run_blockless(config);
        assert_eq!(code.code, 0);
    }

    #[test]
    #[ignore = "cross imports not supported"]
    fn test_blockless_reactor_module_can_call_reactor_module_with_callback_support() {
        let primary_code = r#"
        (module
            (import "reactor1" "double1" (func $double1 (param i32) (result i32)))
            (func (export "_start")
                i32.const 2
                call $double1
                drop
            )
        )
        "#;
        let reactor_1_code = r#"
        (module
            (import "reactor2" "double2" (func $double2 (param i32) (result i32)))
            (func (export "double1") (param i32) (result i32)
                local.get 0
                call $double2
            )
            (func (export "double1callback") (param i32) (result i32)
                local.get 0
                i32.const 2
                i32.mul
            )
        )
        "#;
        let reactor_2_code = r#"
        (module
            (import "reactor1" "double1callback" (func $double1callback (param i32) (result i32)))
            (func (export "double2") (param i32) (result i32)
                local.get 0
                call $double1callback
            )
        )
        "#;

        let temp_dir = TempDir::new("blockless_run").unwrap();

        let primary_path = temp_dir.path().join("run.wasm");
        let reactor_1_path = temp_dir.path().join("reactor1.wasm");
        let reactor_2_path = temp_dir.path().join("reactor2.wasm");
        
        fs::write(&primary_path, primary_code).unwrap();
        fs::write(&reactor_1_path, reactor_1_code).unwrap();
        fs::write(&reactor_2_path, reactor_2_code).unwrap();
        
        let modules = vec![
            BlocklessModule { 
                module_type: ModuleType::Entry, 
                name: "".to_string(), 
                file: primary_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(primary_code)), 
            },
            BlocklessModule { 
                module_type: ModuleType::Module, 
                name: "reactor1".to_string(), 
                file: reactor_1_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(reactor_1_code)) 
            },
            BlocklessModule { 
                module_type: ModuleType::Module, 
                name: "reactor2".to_string(), 
                file: reactor_2_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(reactor_2_code)) 
            },
        ];
        let mut config =  BlocklessConfig::new("_start");
        config.set_version(BlocklessConfigVersion::Version1);
        config.set_modules(modules);
        let code = run_blockless(config);
        assert_eq!(code.code, 0);
    }

    #[test]
    #[ignore = "cross imports and callback loops not supported"]
    fn test_blockless_reactor_module_can_call_reactor_module_with_callback_endless_loop() {
        let primary_code = r#"
        (module
            (import "reactor1" "double1" (func $double1 (param i32) (result i32)))
            (func (export "_start")
                i32.const 2
                call $double1
                drop
            )
          )
        "#;
        let reactor_1_code = r#"
        (module
            (import "reactor2" "double2" (func $double2 (param i32) (result i32)))
            (func (export "double1") (param i32) (result i32)
                local.get 0
                call $double2
            )
        )
        "#;
        let reactor_2_code = r#"
        (module
            (import "reactor1" "double1" (func $double1 (param i32) (result i32)))
            (func (export "double2") (param i32) (result i32)
                local.get 0
                call $double1
            )
        )
        "#;

        let temp_dir = TempDir::new("blockless_run").unwrap();

        let primary_path = temp_dir.path().join("run.wasm");
        let reactor_1_path = temp_dir.path().join("reactor1.wasm");
        let reactor_2_path = temp_dir.path().join("reactor2.wasm");
        
        fs::write(&primary_path, primary_code).unwrap();
        fs::write(&reactor_1_path, reactor_1_code).unwrap();
        fs::write(&reactor_2_path, reactor_2_code).unwrap();
        
        let modules = vec![
            BlocklessModule { 
                module_type: ModuleType::Entry, 
                name: "".to_string(), 
                file: primary_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(primary_code)), 
            },
            BlocklessModule { 
                module_type: ModuleType::Module, 
                name: "reactor1".to_string(), 
                file: reactor_1_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(reactor_1_code)) 
            },
            BlocklessModule { 
                module_type: ModuleType::Module, 
                name: "reactor2".to_string(), 
                file: reactor_2_path.to_str().unwrap().to_string(), 
                md5: format!("{:x}", md5::compute(reactor_2_code)) 
            },
        ];
        let mut config =  BlocklessConfig::new("_start");
        config.set_version(BlocklessConfigVersion::Version1);
        config.set_modules(modules);
        let code = run_blockless(config);
        assert_eq!(code.code, 0);
    }
}