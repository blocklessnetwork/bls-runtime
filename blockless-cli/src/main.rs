mod config;
use blockless::blockless_run;
use config::CliConfig;
use std::{env, io};
use tokio::runtime::Builder;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let path = args.iter().nth(1);
    let mut std_buffer = String::new();

    if path.is_none() {
        eprintln!("usage: {} [path]\npath: configure file path", args[0]);
        return;
    }

    let mut cfg = CliConfig::from_file(path.unwrap()).unwrap();

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
        let env = env_logger::Env::default();
        env_logger::init_from_env(env);
        blockless_run(cfg.0).await;
    });
}
