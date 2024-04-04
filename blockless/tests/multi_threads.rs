use std::fs;

use blockless::BlocklessConfig;
use tempdir::TempDir;

mod common;

#[test]
fn multi_threads() {
    let temp_dir = TempDir::new("blockless_run").unwrap();
    let code = r#"
    (module
        (func (export "_start"))
        (memory (export "memory") 1)
    )
    "#;
    let file_path = temp_dir.path().join("test_multi_threads.wat");
    fs::write(&file_path, &code).unwrap();
    let entry = file_path.to_str().unwrap();
    let mut cfg = BlocklessConfig::new(&entry);
    cfg.set_feature_thread(true);
    common::multi_threads_run_blockless(cfg);
}