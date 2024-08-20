mod common;

use std::fs;

use common::run_blockless;
use tempdir::TempDir;
use wasi_common::{BlocklessConfig, BlocklessConfigVersion};

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
    let mut config = BlocklessConfig::new(path);
    config.limited_fuel(Some(1));
    config.set_version(BlocklessConfigVersion::Version0);
    let code = run_blockless(config).unwrap();
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
    let mut config = BlocklessConfig::new(path);
    config.set_version(BlocklessConfigVersion::Version0);
    let code = run_blockless(config).unwrap();
    assert_eq!(code.code, 0);
}
