use blockless::{blockless_run, ExitStatus};
use tokio::runtime::Builder;
use wasi_common::BlocklessConfig;

/// runing environment for test.
#[allow(dead_code)]
pub fn run_blockless(config: BlocklessConfig) -> anyhow::Result<ExitStatus> {
    let rt = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async { blockless_run(config).await })
}

/// multi-thread runing environment for test.
#[allow(dead_code)]
pub fn multi_threads_run_blockless(config: BlocklessConfig) -> anyhow::Result<ExitStatus> {
    let rt = Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async { blockless_run(config).await })
}
