mod config;
use config::CliConfig;
use blockless::{blockless_run};
use tokio::runtime::Builder;
use std::env;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let path  = args.iter().nth(1);
    if path.is_none() {
        eprintln!("usage: {} [path]\npath: configure file path", args[0]);
        return;
    }
    let  cfg = CliConfig::from_file(path.unwrap()).unwrap();
    let rt = Builder::new_current_thread().enable_io().build().unwrap();
    rt.block_on(async {
        let env = env_logger::Env::default();
        env_logger::init_from_env(env);
        blockless_run(cfg.0).await;
    });
}
