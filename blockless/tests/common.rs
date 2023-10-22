use blockless::{ExitStatus, blockless_run};
use tokio::runtime::Builder;
use wasi_common::BlocklessConfig;

pub fn run_blockless(config: BlocklessConfig) -> ExitStatus {
    let rt = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async { blockless_run(config).await })
}