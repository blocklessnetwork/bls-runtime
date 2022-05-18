use blockless::{blockless_run, BlocklessConfig};
use tokio::runtime::Builder;

fn main() {
    let rt = Builder::new_current_thread().enable_io().build().unwrap();
    rt.block_on(async {
        let env = env_logger::Env::default();
        env_logger::init_from_env(env);
        let mut bc = BlocklessConfig::new("/Users/join/Downloads/main.wasi");
        bc.root_path("/Users/join/Downloads");
        bc.limited_fuel(Some(2000_0000));
        bc.limited_memory(Some(3));
        blockless_run(bc).await;
    });
}
