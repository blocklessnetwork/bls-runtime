mod config;
use blockless_env;
pub use config::Stdout;
use std::path::Path;
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;
use log::{info, error};

pub use config::BlocklessConfig;

const ENTRY: &str = "_start";

pub async fn blockless_run(b_conf: BlocklessConfig) {
    let mut conf = Config::new();
    conf.async_support(true);
    conf.consume_fuel(false);
    let engine = Engine::new(&conf).unwrap();
    let mut linker = Linker::new(&engine);
    blockless_env::add_to_linker(&mut linker);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s).unwrap();
    let root_dir = b_conf
        .root_path_ref()
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
            if let Some(r) = b_conf.root_path_ref() {
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
    let mut store = Store::new(&engine, ctx);
    // store.add_fuel(1_0_0000).unwrap();
    // Instantiate our module with the imports we've created, and run it.
    let module = Module::from_file(&engine, b_conf.wasm_file_ref()).unwrap();
    linker.module(&mut store, "", &module).unwrap();
    let inst = linker.instantiate_async(&mut store, &module).await.unwrap();
    let func = inst.get_typed_func::<(), (), _>(&mut store, ENTRY).unwrap();
    match func.call_async(&mut store, ()).await {
        Err(ref t) => trap_info(t, store.fuel_consumed()),
        Ok(_) => info!("program exit normal."),
    }
}


fn trap_info(t: &Trap, fuel: Option<u64>) {
    if let Some(0) = fuel {
        error!("all fuel is consumed, the app exited. {:?}", t);
    } else {
        error!("{:?}", t);
    }
}