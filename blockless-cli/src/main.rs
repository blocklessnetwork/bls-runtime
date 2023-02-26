mod config;
use blockless::{blockless_run, LoggerLevel};
use config::CliConfig;
use anyhow::Result;
use std::{env, io};
use env_logger::Target;
use tokio::runtime::Builder;
use log::{error, info, LevelFilter};
use std::fs;
use std::path::Path;
use rust_car::{
    reader::{self, CarReader},
    utils::ipld_write
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

fn load_cli_config(conf_path: &str) -> Result<CliConfig> {
    let ext = Path::new(conf_path).extension();
    let cfg = ext.and_then(|ext| ext.to_str().map(str::to_ascii_lowercase));
    let cli_config = match cfg {
        Some(ref f) if f == "car" => {
            let file = fs::OpenOptions::new()
                .read(true)
                .open(f)?;
            let mut car_reader = reader::new_v1(file)?;
            let cid = car_reader.search_file_cid("config.json").ok();
            match cid {
                Some(c) => {
                    let mut data = Vec::new();
                    ipld_write(&mut car_reader, c, &mut data)?;
                    Some(CliConfig::from_data(data))
                }
                None => None,
            }
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
