use blockless::{blockless_run, BlocklessConfig};
use tokio::runtime::Builder;

fn main() {
    let rt = Builder::new_current_thread().enable_io().build().unwrap();
    rt.block_on(async {
        let bc = BlocklessConfig::new("/Users/join/Downloads/main.wasi");
        blockless_run(bc).await;
    });
}
