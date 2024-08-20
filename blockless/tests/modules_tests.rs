mod common;
use std::fs;

use common::run_blockless;
use tempdir::TempDir;
use wasi_common::{BlocklessConfig, BlocklessConfigVersion, BlocklessModule, ModuleType};

#[test]
fn test_linker_module() {
    let guest_wasm = r#"
    (module
        (type $fd_write_ty (func (param i32 i32 i32 i32) (result i32)))
        (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (type $fd_write_ty)))
        
        (func $log (export "log") (param i32 i32)
            ;; store the pointer in the first iovec field
            i32.const 4
            local.get 0
            i32.store
        
            ;; store the length in the first iovec field
            i32.const 4
            local.get 1
            i32.store offset=4
        
            ;; call the `fd_write` import
            i32.const 1     ;; stdout fd
            i32.const 4     ;; iovs start
            i32.const 1     ;; number of iovs
            i32.const 0     ;; where to write nwritten bytes
            call $fd_write
            drop
        )
        (func $initialize (export "_start")
            i32.const 1024  ;; pass offset 0 to log
            i32.const 15  ;; pass length 2 to log
            call $log
        )
        
        (memory (export "memory") 2)
        (global (export "memory_offset") i32 (i32.const 65536))
        
        (data (i32.const 1024) "Hello, world2!\n")
    )
    "#;

    let temp_dir = TempDir::new("blockless_run").unwrap();
    let guest_path = temp_dir.path().join("test_linker_module.wasm");

    fs::write(&guest_path, guest_wasm).unwrap();

    let modules = vec![BlocklessModule {
        module_type: ModuleType::Entry,
        name: "".to_string(),
        file: guest_path.to_str().unwrap().to_string(),
        md5: format!("{:x}", md5::compute(guest_wasm)),
    }];
    let mut config = BlocklessConfig::new("_start");
    config.set_version(BlocklessConfigVersion::Version1);
    config.set_modules(modules);
    let code = run_blockless(config).unwrap();
    assert_eq!(code.code, 0);
}

#[test]
fn test_blockless_extension_http_req() {
    let guest_wasm = r#"
    (module
        (type $http_req_ty (func (param i32 i32 i32 i32 i32 i32) (result i32)))
        (import "blockless_http" "http_req" (func $http_req (type $http_req_ty)))

        (memory (export "memory") 10)

        (func (export "_start")
            (drop
                (call $http_req 
                    (global.get $url_ptr)
                    (i32.const 30)
                    (global.get $param_ptr)
                    (i32.const 31)
                    (global.get $handle_ptr)
                    (global.get $status_ptr)
                )
            )
        )

        (global $url_ptr i32 (i32.const 10256))
        (global $param_ptr i32 (i32.const 10330))
        (global $handle_ptr i32 (i32.const 112400))
        (global $status_ptr i32 (i32.const 112940))

        (data (i32.const 10256) "https://reqres.in/api/products")
        (data (i32.const 10330) "{\"method\":\"get\",\"headers\":\"{}\"}")
    )
    "#;

    let temp_dir = TempDir::new("blockless_run").unwrap();
    let guest_path = temp_dir
        .path()
        .join("test_blockless_extension_http_req.wasm");

    fs::write(&guest_path, guest_wasm).unwrap();

    let modules = vec![BlocklessModule {
        module_type: ModuleType::Entry,
        name: "".to_string(),
        file: guest_path.to_str().unwrap().to_string(),
        md5: format!("{:x}", md5::compute(guest_wasm)),
    }];
    let mut config = BlocklessConfig::new("_start");
    config.set_version(BlocklessConfigVersion::Version1);
    config.set_modules(modules);
    let code = run_blockless(config).unwrap();
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
            md5: format!("{:x}", md5::compute(reactor_1_code)),
        },
    ];
    let mut config = BlocklessConfig::new("_start");
    config.set_version(BlocklessConfigVersion::Version1);
    config.set_modules(modules);
    let code = run_blockless(config).unwrap();
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
            md5: format!("{:x}", md5::compute(reactor_1_code)),
        },
        BlocklessModule {
            module_type: ModuleType::Module,
            name: "reactor2".to_string(),
            file: reactor_2_path.to_str().unwrap().to_string(),
            md5: format!("{:x}", md5::compute(reactor_2_code)),
        },
    ];
    let mut config = BlocklessConfig::new("_start");
    config.set_version(BlocklessConfigVersion::Version1);
    config.set_modules(modules);
    let code = run_blockless(config).unwrap();
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
            md5: format!("{:x}", md5::compute(reactor_2_code)),
        },
        BlocklessModule {
            module_type: ModuleType::Module,
            name: "reactor1".to_string(),
            file: reactor_1_path.to_str().unwrap().to_string(),
            md5: format!("{:x}", md5::compute(reactor_1_code)),
        },
    ];
    let mut config = BlocklessConfig::new("_start");
    config.set_version(BlocklessConfigVersion::Version1);
    config.set_modules(modules);
    let code = run_blockless(config).unwrap();
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
            md5: format!("{:x}", md5::compute(reactor_1_code)),
        },
        BlocklessModule {
            module_type: ModuleType::Module,
            name: "reactor2".to_string(),
            file: reactor_2_path.to_str().unwrap().to_string(),
            md5: format!("{:x}", md5::compute(reactor_2_code)),
        },
    ];
    let mut config = BlocklessConfig::new("_start");
    config.set_version(BlocklessConfigVersion::Version1);
    config.set_modules(modules);
    let code = run_blockless(config).unwrap();
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
            md5: format!("{:x}", md5::compute(reactor_1_code)),
        },
        BlocklessModule {
            module_type: ModuleType::Module,
            name: "reactor2".to_string(),
            file: reactor_2_path.to_str().unwrap().to_string(),
            md5: format!("{:x}", md5::compute(reactor_2_code)),
        },
    ];
    let mut config = BlocklessConfig::new("_start");
    config.set_version(BlocklessConfigVersion::Version1);
    config.set_modules(modules);
    let code = run_blockless(config).unwrap();
    assert_eq!(code.code, 0);
}
