mod config;
use blockless::{blockless_run, LoggerLevel};
use config::CliConfig;
use std::{env, io};
use env_logger::Target;
use tokio::runtime::Builder;
use log::{error, info, LevelFilter};
use std::fs;



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

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let path = args.iter().nth(1);
    let mut std_buffer = String::new();

    if path.is_none() {
        eprintln!("usage: {} [path]\npath: configure file path", args[0]);
        return;
    }

    let mut cfg = CliConfig::from_file(path.unwrap()).unwrap();
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
        println!("The wasm execute finish, the exit code: {}", exit_code.code);
        info!("The wasm execute finish, the exit code: {}", exit_code.code);
    });
}
