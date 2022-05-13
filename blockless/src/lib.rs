mod config;
use blockless_env;
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

pub use config::BlocklessConfig;

const ENTRY: &str = "_start";

pub async fn blockless_run(b_conf: BlocklessConfig) {
    let mut conf = Config::new();
    conf.async_support(true);
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
    let mut builder = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()
        .unwrap();
    if let Some(d) = root_dir {
        builder = builder.preopened_dir(d, "/").unwrap();
    }
    let ctx = builder.build();
    let mut store = Store::new(&engine, ctx);
    // Instantiate our module with the imports we've created, and run it.
    let module = Module::from_file(&engine, b_conf.wasm_file_ref()).unwrap();
    linker.module(&mut store, "", &module).unwrap();
    let inst = linker.instantiate_async(&mut store, &module).await.unwrap();
    let func = inst.get_typed_func::<(), (), _>(&mut store, ENTRY).unwrap();
    let _ = func.call_async(&mut store, ()).await.unwrap();
}
