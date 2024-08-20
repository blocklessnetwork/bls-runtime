mod cli_clap;
mod config;
mod error;
#[macro_use]
mod macros;
mod plog;
mod v86;
mod v86config;
use blockless::{blockless_run, LoggerLevel};
use clap::Parser;
use cli_clap::{CliCommandOpts, RuntimeType};
use config::load_cli_config_extract_from_car;
#[allow(unused_imports)]
use config::CliConfig;
use env_logger::Target;
use error::CliExitCode;
use log::{error, info, LevelFilter};
use std::fs;
use std::path::Path;
use std::{io::Read, path::PathBuf, time::Duration};
use v86::V86Lib;
use v86config::load_v86conf_extract_from_car;

const ENV_ROOT_PATH_NAME: &str = "ENV_ROOT_PATH";

/// set the logger output and filter level.
fn logger_init_with_config(cfg: &CliConfig) -> Result<(), CliExitCode> {
    let rt_logger = cfg.0.runtime_logger_path();
    let rt_logger_level = cfg.0.get_runtime_logger_level();
    logger_init(rt_logger, rt_logger_level)?;
    Ok(())
}

/// init the logger with the configure file.
/// the default output is the console.
/// you can set the output as file in the configure.
fn logger_init(
    rt_logger: Option<PathBuf>,
    rt_logger_level: LoggerLevel,
) -> Result<(), CliExitCode> {
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
                .map_err(|_e| {
                    CliExitCode::UnknownError(
                        "the runtime logger file does not exist or is unreadable.".into(),
                    )
                })?;
            Target::Pipe(Box::new(file))
        }
    };

    builder.target(target);
    builder.init();
    Ok(())
}

fn file_md5(f: impl AsRef<Path>) -> Result<String, CliExitCode> {
    let mut file = fs::OpenOptions::new().read(true).open(f).map_err(|_e| {
        CliExitCode::UnknownError(
            "the module file either does not exist or is inaccessible.".into(),
        )
    })?;
    let mut buf = vec![0u8; 2048];
    let mut md5_ctx = md5::Context::new();
    loop {
        let n = file.read(&mut buf).map_err(|_e| {
            CliExitCode::UnknownError(
                "the module file either does not exist or is inaccessible.".into(),
            )
        })?;
        if n == 0 {
            break;
        }
        md5_ctx.consume(&buf[..n])
    }
    let digest = md5_ctx.compute();
    Ok(format!("{digest:x}"))
}

fn check_module_sum(cfg: &CliConfig) -> Result<(), CliExitCode> {
    for module in cfg.0.modules_ref() {
        let m_file = &module.file;
        let md5sum = file_md5(m_file)?;
        if md5sum != module.md5 {
            perror!("the module {m_file} file md5 checksum is not correctly.");
            return Err(CliExitCode::ConfigureError);
        }
    }
    Ok(())
}

/// the cli support 3 type file,
/// 1. the car file format, all files archive into the car file.
/// 2. the wasm or wasi file format, will run wasm directly.
/// 3. the the config file, format, all files is define in the config file.
fn load_cli_config(file_path: &str) -> Result<CliConfig, CliExitCode> {
    let ext = Path::new(file_path).extension();
    let cfg = ext.and_then(|ext| ext.to_str().map(str::to_ascii_lowercase));
    let cli_config = match cfg {
        Some(ext) if ext == "car" => {
            let file = fs::OpenOptions::new()
                .read(true)
                .open(file_path)
                .map_err(|_e| {
                    CliExitCode::UnknownError(
                        "the car file does not exist or is unreadable.".into(),
                    )
                })?;
            Some(load_cli_config_extract_from_car(file))
        }
        Some(ext) if ext == "wasm" || ext == "wasi" || ext == "wat" => {
            Some(Ok(CliConfig::new_with_wasm(file_path)))
        }
        _ => None,
    };
    cli_config
        .unwrap_or_else(|| CliConfig::from_file(file_path))
        .map_err(|e| CliExitCode::UnknownError(e.to_string()))
}

fn v86_runtime(path: &str) -> Result<i32, CliExitCode> {
    let file = fs::OpenOptions::new().read(true).open(path).map_err(|e| {
        CliExitCode::UnknownError(format!(
            "the v86 car file does not exist or is unreadable: {}",
            e
        ))
    })?;

    let cfg = load_v86conf_extract_from_car(file)?;
    let v86 = V86Lib::load(&cfg.dynamic_lib_path)?;

    let raw_config_json = &cfg.raw_config.ok_or_else(|| {
        CliExitCode::UnknownError(
            "the v86 config file does not exist or is unreadable.".to_string(),
        )
    })?;
    Ok(v86.v86_wasi_run(raw_config_json))
}

async fn wasm_runtime(mut cfg: CliConfig, cli_command_opts: CliCommandOpts) -> CliExitCode {
    if let Err(err) = logger_init_with_config(&cfg) {
        perror!("failed to init logger: {}", err);
        return err;
    }

    if cfg.0.stdin_ref().is_empty() {
        if let Some(stdin_buffer) = non_blocking_read(std::io::stdin()).await {
            cfg.0.stdin(stdin_buffer);
        }
    }
    let run_time = cfg.0.run_time();
    cli_command_opts.into_config(&mut cfg);

    if let Some(time) = run_time {
        let _ = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(time)).await;
            info!("The wasm execute finish, the exit code: 15");
            std::process::exit(CliExitCode::AppTimeout.into());
        })
        .await;
    }

    info!("The wasm app started.");
    std::panic::set_hook(Box::new(|panic_info| {
        error!("{}", panic_info);
        perror!("WASM app crashed, please check the runtime.log file");
    }));

    let exit_status = blockless_run(cfg.0).await.unwrap();
    info!(
        "The wasm execute finish, the exit code: {}",
        exit_status.code
    );
    exit_status.code.into()
}

fn set_root_path_env_var(cli_command_opts: &CliCommandOpts) {
    cli_command_opts
        .fs_root_path()
        .map(|s| std::env::set_var(ENV_ROOT_PATH_NAME, s.as_str()));
}

async fn non_blocking_read<R: Read + Send + 'static>(mut reader: R) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();

    // spawn thread to read from the reader asynchronously
    std::thread::spawn(move || {
        let mut buffer = String::new();
        if reader.read_to_string(&mut buffer).is_ok() && !buffer.is_empty() {
            // blocks
            let _ = tx.send(buffer);
        }
    });

    // wait for either a message from the thread or timeout
    rx.recv_timeout(std::time::Duration::from_millis(1000)).ok()
}

#[tokio::main]
async fn main() -> CliExitCode {
    let cli_command_opts = CliCommandOpts::parse();
    set_root_path_env_var(&cli_command_opts);
    let path = cli_command_opts.input_ref();

    match cli_command_opts.runtime_type() {
        RuntimeType::V86 => match v86_runtime(&path) {
            Ok(exit_code_err) => return exit_code_err.into(),
            Err(e) => {
                perror!("{}", e);
                return e;
            }
        },
        RuntimeType::Wasm => {
            let cfg = match load_cli_config(&path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    perror!("failed to load CLI config: {}", e);
                    return e;
                }
            };
            if let Err(code) = check_module_sum(&cfg) {
                perror!("{}", code);
                return code;
            }
            return wasm_runtime(cfg, cli_command_opts).await;
        }
    };
}

#[cfg(test)]
mod test {
    #![allow(unused)]
    use crate::config::load_cli_config_from_car;
    use blockless::ModuleType;
    use rust_car::{
        codec::Encoder,
        header::CarHeader,
        reader::{self, CarReader},
        unixfs::{Link, UnixFs},
        writer::{self as car_writer, CarWriter},
        Ipld,
    };

    use super::*;

    #[test]
    fn test_set_root_path_env_var() {
        let cli_opts =
            CliCommandOpts::try_parse_from(["cli", "test", "--fs-root-path=./test"]).unwrap();
        set_root_path_env_var(&cli_opts);
        assert_eq!(
            std::env::var(ENV_ROOT_PATH_NAME).unwrap(),
            *cli_opts.fs_root_path().unwrap()
        );
    }

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
            }"#
            .to_vec();
            let d_len = data.len();
            let f_cid = writer.write_ipld(Ipld::Bytes(data)).unwrap();
            let mut unixfs = UnixFs::new_directory();
            unixfs.add_link(Link::new(f_cid, "config.json".to_string(), d_len as _));
            let root_cid = writer.write_ipld(unixfs.encode().unwrap());
            writer
                .rewrite_header(CarHeader::new_v1(vec![root_cid.unwrap()]))
                .unwrap();
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

    #[tokio::test]
    async fn test_no_input_non_blocking_read() {
        let result = non_blocking_read(std::io::stdin()).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_empty_input_non_blocking_read() {
        let cursor = std::io::Cursor::new("");
        let result = non_blocking_read(cursor).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_input_non_blocking_read() {
        let cursor = std::io::Cursor::new("test input");
        let result = non_blocking_read(cursor).await;
        assert_eq!(result, Some("test input".to_string()));
    }
}
