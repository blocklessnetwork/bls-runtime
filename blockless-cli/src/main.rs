mod config;
use blockless::{
    blockless_run, 
    LoggerLevel
};
use config::CliConfig;
use anyhow::Result;
use std::{
    env, 
    io::{self, Read}, 
    fs::File, 
    path::PathBuf, 
    time::Duration, 
    process::ExitCode
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
use rust_car::{
    reader::{self, CarReader},
    utils::{ipld_write, extract_ipld}
};

fn logger_init(cfg: &CliConfig) {
    let rt_logger = cfg.0.runtime_logger_ref();
    let mut builder = env_logger::Builder::from_default_env();
    let rt_logger_level = cfg.0.runtime_logger_level_ref();
    let filter_level = match *rt_logger_level {
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

fn load_from_car<T>(car_reader: &mut T) -> Result<CliConfig>
where
    T: CarReader
{
    let cid = car_reader.search_file_cid("config.json")?;
    let mut data = Vec::new();
    ipld_write(car_reader, cid, &mut data)?;
    let raw_json = String::from_utf8(data)?;
    let roots = car_reader.header().roots();
    let root_suffix = roots.iter().nth(0).map(|c| c.to_string());
    let mut cli_cfg = CliConfig::from_data(raw_json, root_suffix)?;
    cli_cfg.0.set_is_carfile(true);
    Ok(cli_cfg)
}

fn load_extract_from_car(f: File) -> Result<CliConfig> {
    let mut reader = reader::new_v1(f)?;
    let cfg = load_from_car(&mut reader)?;
    let header = reader.header();
    let rootfs = cfg.0.fs_root_path_ref().expect("root path must be config in car file");
    for rcid in header.roots() {
        let root_path: PathBuf = rootfs.into();
        let root_path = root_path.join(rcid.to_string());
        if !root_path.exists() {
            fs::create_dir(&root_path)?;
        }
        extract_ipld(&mut reader, rcid, Some(root_path))?;
    }
    Ok(cfg)
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
            Some(load_extract_from_car(file))
        },
        Some(ext) if ext == "wasm" || ext == "wasi" => {
            Some(load_wasm_directly(file_path))
        },
        _ => None,
    };
    cli_config.unwrap_or_else(|| CliConfig::from_file(file_path))
}

fn main() -> ExitCode {
    let args = env::args().collect::<Vec<_>>();
    let path = args.iter().nth(1);
    let mut std_buffer = String::new();

    if path.is_none() {
        eprintln!("usage: {} [path]\npath: configure file path", args[0]);
        return ExitCode::from(128);
    }
    let path = path.unwrap();
    let mut cfg = load_cli_config(path).unwrap();
    if let Some(code) = check_module_sum(&cfg) {
        return ExitCode::from(code as u8);
    }
    logger_init(&cfg);
    if cfg.0.stdin_ref().is_empty() {
        io::stdin().read_line(&mut std_buffer).unwrap();
        cfg.0.stdin(std_buffer);
    }
    let run_time = cfg.0.run_time();
    let rt = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
        
    let code = rt.block_on(async {
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
        exit_code.code
    });
    ExitCode::from(code as u8)
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
        codec::Encoder
    };
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
        let cfg = load_from_car(&mut car_reader).unwrap();
        assert_eq!(cfg.0.fs_root_path_ref(), Some("target"));
        assert_eq!(cfg.0.drivers_root_path_ref(), Some("target/drivers"));
    }
}