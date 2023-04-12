#![allow(unused)]
use blockless::BlocklessConfig;
use clap::{Arg, ArgMatches, Command};

pub(crate) fn cli_command() -> Command {
    Command::new("blockless_cli")
        .arg(
            Arg::new("input")
                .help("the input file is wasm file, configure file, or car file")
                .required(true)
        )
        .arg(
            Arg::new("debug_info")
                .long("debug_info")
                .help("the debug info for the runtime.")
                .required(false),
        )
        .arg(
            Arg::new("fs_root_path")
                .long("fs_root_path")
                .help("the root directory for the runtime.")
                .required(false),
        )
        .arg(
            Arg::new("runtime_logger")
                .long("runtime_logger")
                .help("the logger file for the runtime.")
                .required(false),
        )
        .arg(
            Arg::new("limited_memory")
                .long("limited_memory")
                .help("the limited memory for the runtime, default is infine.")
                .required(false),
        )
        .arg(
            Arg::new("run_time")
                .long("run_time")
                .help("the run time for the runtime, default is infine.")
                .required(false),
        )
        .arg(
            Arg::new("entry")
                .long("entry")
                .help("the entry for wasm, default is _start")
                .required(false),
        )
        .arg(
            Arg::new("limited_fuel")
                .long("limited_fuel")
                .help("the limited fuel for runtime, default is infine")
                .required(false),
        )
        .allow_missing_positional(true)
}

#[rustfmt::skip]
pub(crate) fn apply_config(cfg: &mut BlocklessConfig, matches: &ArgMatches) {
    matches.get_one::<bool>("debug_info").map(|d| cfg.debug_info(*d));
    matches.get_one::<String>("fs_root_path").map(|f| cfg.fs_root_path(Some(f.clone())));
    matches.get_one::<String>("runtime_logger").map(|f| cfg.runtime_logger(Some(f.clone())));
    matches.get_one::<String>("entry").map(|f| cfg.entry(f.clone()));
    matches.get_one::<u64>("limited_memory").map(|f| cfg.limited_memory(Some(*f)));
    matches.get_one::<u64>("run_time").map(|f| cfg.set_run_time(Some(*f)));
    matches.get_one::<u64>("limited_fuel").map(|f| cfg.limited_fuel(Some(*f)));
}

#[cfg(test)]
mod test {

    #[allow(unused)]
    use super::*;

    #[test]
    fn test_cli_command_input() {
        let cli_cmd = cli_command();
        let command_line = r#"blockless_cli test.wasm"#;
        let command_line = command_line
            .split(" ")
            .map(str::to_string)
            .collect::<Vec<String>>();
        let matches = cli_cmd.try_get_matches_from(command_line).unwrap();
        let input = matches.get_one::<String>("input");
        let pat = match input {
            Some(p) => p,
            None => unreachable!("can't reach here!"),
        };
        assert_eq!(pat, &"test.wasm");
    }

    #[test]
    fn test_cli_command_runtime_log() {
        let cli_cmd = cli_command();
        let command_line = r#"blockless_cli test.wasm --runtime_logger runtime.log"#;
        let command_line = command_line
            .split(" ")
            .map(str::to_string)
            .collect::<Vec<String>>();
        let matches = cli_cmd.try_get_matches_from(command_line).unwrap();
        let input = matches.get_one::<String>("runtime_logger");
        let pat = match input {
            Some(p) => p,
            None => unreachable!("can't reach here!"),
        };
        assert_eq!(pat, &"runtime.log");
    }

    #[test]
    fn test_cli_command_fs_root_path() {
        let cli_cmd = cli_command();
        let command_line = r#"blockless_cli test.wasm --fs_root_path /"#;
        let command_line = command_line
            .split(" ")
            .map(str::to_string)
            .collect::<Vec<String>>();
        let matches = cli_cmd.try_get_matches_from(command_line).unwrap();
        let input = matches.get_one::<String>("fs_root_path");
        let pat = match input {
            Some(p) => p,
            None => unreachable!("can't reach here!"),
        };
        assert_eq!(pat, &"/");
    }
    
}
