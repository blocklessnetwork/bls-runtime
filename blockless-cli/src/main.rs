mod config;
use blockless::{blockless_run, LoggerLevel};
use config::CliConfig;
use anyhow::Result;
use std::{env, io, fs::File, path::PathBuf};
use env_logger::Target;
use tokio::runtime::Builder;
use log::{error, info, LevelFilter};
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

struct EnvVar {
    pub name: String,
    pub value: String,
}

fn env_variables(cid: String) -> Result<Vec<EnvVar>> {
    let mut vars = Vec::new();
    match std::env::var("ENV_ROOT_PATH") {
        Ok(s) => {
            let env_root = s.clone();
            let path: PathBuf = s.into();
            let root_path = path.join(cid);
            let path: String = root_path.to_str().unwrap_or_default().into();
            vars.push(EnvVar { 
                name: "$ROOT".to_string(), 
                value: path,
            });
            vars.push(EnvVar {
                name: "$ENV_ROOT_PATH".to_string(),
                value: env_root,
            });
        },
        Err(e) => return Err(e.into()),
    }
    Ok(vars)
}

fn load_from_car<T>(car_reader: &mut T) -> Result<CliConfig>
where
    T: CarReader
{
    let cid = car_reader.search_file_cid("config.json")?;
    let mut data = Vec::new();
    ipld_write(car_reader, cid, &mut data)?;
    let mut raw_json = String::from_utf8(data)?;
    let roots = car_reader.header().roots();
    let vars = env_variables(roots[0].to_string())?;
    for var in vars {
        raw_json = raw_json.replace(&var.name, &var.value);
    }
    let mut cli_cfg = CliConfig::from_data(raw_json.into())?;
    cli_cfg.0.set_is_carfile(true);
    Ok(cli_cfg)
}

fn load_extract_from_car(f: File) -> Result<CliConfig> {
    let mut reader = reader::new_v1(f)?;
    let cfg = load_from_car(&mut reader)?;
    let header = reader.header();
    let rootfs = cfg.0.fs_root_path_ref().expect("root path must be config in car file");
    for rcid in header.roots() {
        extract_ipld(&mut reader, rcid, Some(rootfs))?;
    }
    Ok(cfg)
}

fn load_cli_config(conf_path: &str) -> Result<CliConfig> {
    let ext = Path::new(conf_path).extension();
    let cfg = ext.and_then(|ext| ext.to_str().map(str::to_ascii_lowercase));
    let cli_config = match cfg {
        Some(ref f) if f == "car" => {
            let file = fs::OpenOptions::new()
                .read(true)
                .open(f)?;
            Some(load_extract_from_car(file))
        },
        _ => None,
    };
    cli_config.unwrap_or_else(|| CliConfig::from_file(conf_path))
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let path = args.iter().nth(1);
    let mut std_buffer = String::new();

    if path.is_none() {
        eprintln!("usage: {} [path]\npath: configure file path", args[0]);
        return;
    }
    let path = path.unwrap();
    let mut cfg = load_cli_config(path).unwrap();
    logger_init(&cfg);
    if cfg.0.stdin_ref().is_empty() {
        io::stdin().read_line(&mut std_buffer).unwrap();
        cfg.0.stdin(std_buffer);
    }

    let rt = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        info!("The wasm app start.");
        std::panic::set_hook(Box::new(|panic_info| {
            error!("{}", panic_info.to_string());
            eprintln!("The wasm app crash, please check the log file for detail messages.");
        }));
        let exit_code = blockless_run(cfg.0).await;
        info!("The wasm execute finish, the exit code: {}", exit_code.code);
    });
}


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
        assert!(matches!(cfg.0.wasm_file_module(), Some(_)));
        if let Some(c) = cfg.0.wasm_file_module() {
            assert!(matches!(c.module_type, ModuleType::Entry));
            assert_eq!(c.file, format!("target/{}/release.wasm", root_cid.to_string()));
        }
    }
}