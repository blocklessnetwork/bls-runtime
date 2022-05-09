use blockless_env;
use tokio::runtime::Builder;
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;
use wasmtime_wasi::{Dir, WasiCtx};

async fn test_wrap_func(mut _caller: Caller<'_, WasiCtx>) -> i32 {
    1
}

async fn run() {
    let mut conf = Config::new();

    conf.async_support(true);
    let engine = Engine::new(&conf).unwrap();
    let mut linker = Linker::new(&engine);
    blockless_env::add_to_linker(&mut linker);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s).unwrap();
    let dir = std::fs::File::open("/Users/join/Downloads/").unwrap();
    let dir = Dir::from_std_file(dir);
    // Create a WASI context and put it in a Store; all instances in the store
    // share this context. `WasiCtxBuilder` provides a number of ways to
    // configure what the target program will have access to.
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()
        .unwrap()
        .preopened_dir(dir, "/Users/join/Downloads/")
        .unwrap()
        .build();

    let mut store = Store::new(&engine, wasi);

    // Instantiate our module with the imports we've created, and run it.
    let module = Module::from_file(&engine, "/Users/join/Downloads/main.wasi").unwrap();
    linker
        .func_wrap0_async("env", "call_test", |caller: Caller<'_, WasiCtx>| {
            Box::new(test_wrap_func(caller))
        })
        .unwrap();
    linker.module(&mut store, "", &module).unwrap();
    let inst = linker.instantiate_async(&mut store, &module).await.unwrap();
    let func = inst
        .get_typed_func::<(), (), _>(&mut store, "_start")
        .unwrap();
    let _ = func.call_async(&mut store, ()).await.unwrap();
}

fn main() {
    let rt = Builder::new_current_thread().enable_io().build().unwrap();
    rt.block_on(async {
        run().await;
    });
}
