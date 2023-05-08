mod v86;
mod cli_clap;
mod config;
mod v86config;
use blockless::{
    blockless_run, 
    LoggerLevel
};
use clap::Parser;
use cli_clap::CliCommandOpts;
#[allow(unused_imports)]
use config::CliConfig;
use anyhow::Result;
use config::load_cli_config_extract_from_car;
use v86::V86Lib;
use v86config::load_v86conf_extract_from_car;
use std::{
    io::{self, Read}, 
    path::PathBuf, 
    time::Duration, 
    process::ExitCode,
};
use env_logger::Target;
use tokio::runtime::Builder;
use log::{
    error, 
    info, 
    LevelFilter,
};
use std::fs;
use std::path::Path;

fn logger_init_with_config(cfg: &CliConfig) {
    let rt_logger = cfg.0.runtime_logger_path();
    let rt_logger_level = cfg.0.get_runtime_logger_level();
    logger_init(rt_logger, rt_logger_level);
}

fn logger_init(rt_logger: Option<PathBuf>, rt_logger_level: LoggerLevel) {
    let mut builder = env_logger::Builder::from_default_env();
    let filter_level = match rt_logger_level {
        LoggerLevel::INFO => LevelFilter::Info,
        LoggerLevel::WARN => LevelFilter::Warn,
        LoggerLevel::DEBUG => LevelFilter::Debug,
        LoggerLevel::ERROR => LevelFilter::Error,
        LoggerLevel::TRACE => LevelFilter::Trace,
    };
    builder.filter_level(filter_level);
    let target = match rt_logger {
        None => Target::default(),
        Some(f) => {
            builder.is_test(true);
            let file = fs::OpenOptions::new()
                .append(true)
                .create(true)
                .write(true)
                .open(f)
                .unwrap();
            Target::Pipe(Box::new(file))
        },
    };

    builder.target(target);
    builder.init();
}

fn file_md5(f: impl AsRef<Path>) -> String {
    let mut file = fs::OpenOptions::new()
        .read(true)
        .open(f).unwrap();
    let mut buf = vec![0u8; 2048];
    let mut md5_ctx = md5::Context::new();
    loop {
        let n = file.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        md5_ctx.consume(&buf[..n])
    }
    let digest = md5_ctx.compute();
    format!("{digest:x}")
}

fn check_module_sum(cfg: &CliConfig) -> Option<i32> {
    for module in cfg.0.modules_ref() {
        let m_file = &module.file;
        let md5sum = file_md5(m_file);
        if md5sum != module.md5 {
            eprintln!("the module {m_file} file md5 checksum is not correctly.");
            return Some(128);
        }
    }
    None
}

fn load_wasm_directly(wasmfile: &str) -> Result<CliConfig> {
    Ok(CliConfig::new_with_wasm(wasmfile))
}

/// the cli support 3 type file, 
/// 1. the car file format, all files archive into the car file.
/// 2. the wasm or wasi file format, will run wasm directly.
/// 3. the the config file, format, all files is define in the config file.
fn load_cli_config(file_path: &str) -> Result<CliConfig> {
    let ext = Path::new(file_path).extension();
    let cfg = ext.and_then(|ext| ext.to_str().map(str::to_ascii_lowercase));
    let cli_config = match cfg {
        Some(ext) if ext == "car" => {
            let file = fs::OpenOptions::new()
                .read(true)
                .open(file_path)?;
            Some(load_cli_config_extract_from_car(file))
        },
        Some(ext) if ext == "wasm" || ext == "wasi" || ext == "wat" => {
            Some(load_wasm_directly(file_path))
        },
        _ => None,
    };
    cli_config.unwrap_or_else(|| CliConfig::from_file(file_path))
}

fn v86_runtime(path: &str) -> u8 {
    let file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .unwrap();
    let cfg = load_v86conf_extract_from_car(file).unwrap();
    let v86 = V86Lib::load(&cfg.dynamic_lib_path).unwrap();
    let raw_config_json = &cfg.raw_config.unwrap();
    v86.v86_wasi_run(raw_config_json) as u8
}

fn wasm_runtime(mut cfg: CliConfig, cli_command_opts: CliCommandOpts) -> u8 {
    logger_init_with_config(&cfg);
    let mut std_buffer = String::new();
    if cfg.0.stdin_ref().is_empty() {
        io::stdin().read_line(&mut std_buffer).unwrap();
        cfg.0.stdin(std_buffer);
    }
    let run_time = cfg.0.run_time();
    cli_command_opts.into_config(&mut cfg);
    let rt = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        if let Some(time) = run_time {
            let _ = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(time)).await;
                info!("The wasm execute finish, the exit code: 15");
                std::process::exit(15);
            }).await;
        }
        info!("The wasm app start.");
        std::panic::set_hook(Box::new(|panic_info| {
            error!("{}", panic_info.to_string());
            eprintln!("The wasm app crash, please check the log file for detail messages.");
        }));
        let exit_code = blockless_run(cfg.0).await;
        info!("The wasm execute finish, the exit code: {}", exit_code.code);
        exit_code.code as u8
    })
}

fn main() -> ExitCode {
    let cli_command_opts = CliCommandOpts::parse();
    let path = cli_command_opts.input_ref();
    let code = if cli_command_opts.is_v86() {
        v86_runtime(path) as u8
    } else {
        let cfg = load_cli_config(path).unwrap();
        if let Some(code) = check_module_sum(&cfg) {
            return ExitCode::from(code as u8);
        }
        wasm_runtime(cfg, cli_command_opts) as u8
    };
    ExitCode::from(code)
}

#[cfg(test)]
mod test {
    #![allow(unused)]
    use blockless::ModuleType;
    use rust_car::{
        Ipld,
        header::CarHeader,
        writer::{self as  car_writer, CarWriter}, 
        unixfs::{UnixFs, Link}, 
        codec::Encoder, reader::{self, CarReader}
    };
    use crate::config::load_cli_config_from_car;

    use super::*;

    #[test]
    fn test_load_cli_wasm_config() {
        let wasm_conf = load_cli_config("test.wasm");
        let wasm_conf = wasm_conf.unwrap();
        let entry_ref = wasm_conf.0.entry_ref();
        assert_eq!(entry_ref, "test.wasm");
        let root_path_ref = wasm_conf.0.fs_root_path_ref();
        assert_eq!(root_path_ref, Some("."));
    }

    #[test]
    fn test_load_from_car() {
        let mut buf = Vec::new();
        let mut write_car = || {
            let output = std::io::Cursor::new(&mut buf);
            let mut writer = car_writer::new_v1_default_roots(output).unwrap();
            let data = br#"{
                "fs_root_path": "$ENV_ROOT_PATH", 
                "drivers_root_path": "$ENV_ROOT_PATH/drivers", 
                "runtime_logger": "runtime.log", 
                "limited_fuel": 200000000,
                "limited_memory": 30,
                "debug_info": false,
                "entry": "release",
                "modules": [
                    {
                        "file": "$ROOT/lib.wasm",
                        "name": "lib",
                        "type": "module",
                        "md5": "d41d8cd98f00b204e9800998ecf8427e"
                    },
                    {
                        "file": "$ROOT/release.wasm",
                        "name": "release",
                        "type": "entry",
                        "md5": "d41d8cd98f00b204e9800998ecf8427e"
                    }
                ],
                "permissions": [
                    "http://httpbin.org/anything",
                    "file://a.go"
                ]
            }"#.to_vec();
            let d_len = data.len();
            let f_cid = writer.write_ipld(Ipld::Bytes(data)).unwrap();
            let mut unixfs = UnixFs::new_directory();
            unixfs.add_link(Link::new(f_cid, "config.json".to_string(), d_len as _));
            let root_cid = writer.write_ipld(unixfs.encode().unwrap());
            writer.rewrite_header(CarHeader::new_v1(vec![root_cid.unwrap()])).unwrap();
            writer.flush().unwrap();
        };
        write_car();
        let current_path = std::env::current_dir().unwrap();
        std::env::set_var("ENV_ROOT_PATH", "target");
        let input = std::io::Cursor::new(&mut buf);
        let mut car_reader = reader::new_v1(input).unwrap();
        let root_cid = car_reader.header().roots()[0];
        let cfg = load_cli_config_from_car(&mut car_reader).unwrap();
        assert_eq!(cfg.0.fs_root_path_ref(), Some("target"));
        assert_eq!(cfg.0.drivers_root_path_ref(), Some("target/drivers"));
    }
}