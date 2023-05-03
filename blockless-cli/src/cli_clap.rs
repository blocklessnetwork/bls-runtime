#![allow(unused)]
use std::str::FromStr;

use anyhow::{bail, Result};
use blockless::{BlocklessConfig, BlocklessModule, Permission, ModuleType};
use clap::{Arg, ArgMatches, Command, Parser};
use url::Url;

use crate::config::CliConfig;

const INPUT_HELP: &str  = 
    "the input file is wasm file, configure file, or car file";

const DEBUG_INFO_HELP: &str  = 
    "the debug info for the runtime.";

const APP_ARGS_HELP: &str = 
    "the app args will pass into the app";

const FS_ROOT_PATH_HELP: &str = 
    "the root directory for the runtime.";

const RUNTIME_LOGGER_HELP: &str = 
    "the logger file for the runtime.";

const LIMITED_MEMORY_HELP: &str = 
    "the limited memory for the runtime, default is infine.";

const RUN_TIME_HELP: &str = 
    "the run time for the runtime, default is infine.";

const ENTRY_HELP: &str = 
    "the entry for wasm, default is _start";

const LIMITED_FUEL_HELP: &str = 
    "the limited fuel for runtime, default is infine";

const ENVS_HELP: &str = 
    "the app envs will pass into the app";

const PERMISSION_HELP: &str = 
    "the permissions for app";

const MODULES_HELP: &str = 
    "the modules used by app";

fn parse_envs(envs: &str) -> Result<(String, String)> {
    let parts: Vec<_> = envs.splitn(2, "=").collect();
    if parts.len() != 2 {
        bail!("must be of the form `key=value`")
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn parse_permission(permsion: &str) -> Result<Permission> {
    let url = Url::from_str(permsion)?;
    Ok(Permission { 
        schema: url.scheme().into(), 
        url: permsion.into() 
    })
}

fn parse_module(module: &str) -> Result<BlocklessModule> {
    let mods: Vec<_> = module.splitn(2, "=").collect();
    Ok(BlocklessModule {
        module_type: ModuleType::Module,
        name: mods[0].into(),
        file: mods[1].into(),
        md5: String::new(),//didn't need check.
    })
}

#[derive(Parser, Debug)]
pub(crate) struct CliCommandOpts {
    #[clap(value_name = "INPUT", required = true, help = INPUT_HELP )]
    input: String,

    #[clap(long = "debug-info", value_name = "DEBUG-INFO", help = DEBUG_INFO_HELP)]
    debug_info: bool,

    #[clap(long = "fs-root-path", value_name = "FS-ROOT-PATH", help = FS_ROOT_PATH_HELP)]
    fs_root_path: Option<String>,

    #[clap(long = "runtime-logger", value_name = "RUNTIME-LOGGER", help = RUNTIME_LOGGER_HELP)]
    runtime_logger: Option<String>,

    #[clap(long = "limited-memory", value_name = "LIMITED-MEMORY", help = LIMITED_MEMORY_HELP)]
    limited_memory: Option<u64>,

    #[clap(long = "run-time", value_name = "RUN-TIME", help = RUN_TIME_HELP)]
    run_time: Option<u64>,

    #[clap(long = "entry", value_name = "ENTERY", help = ENTRY_HELP)]
    entry: Option<String>,

    #[clap(long = "limited-fuel", value_name = "LIMITED-FUEL", help = LIMITED_FUEL_HELP)]
    limited_fuel: Option<u64>,

    #[clap(long = "env", value_name = "ENV=VAL", help = ENVS_HELP, number_of_values = 1, value_parser = parse_envs)]
    envs: Vec<(String, String)>,

    #[clap(long = "permission", value_name = "PERMISSION", help = PERMISSION_HELP, value_parser = parse_permission)]
    permissions: Vec<Permission>,

    #[clap(long = "module", value_name = "MODULE-NAME=MODULE-PATH", help = MODULES_HELP, value_parser = parse_module)]
    modules: Vec<BlocklessModule>,

    #[clap(value_name = "ARGS", help = APP_ARGS_HELP)]
    args: Vec<String>,
}


impl CliCommandOpts {
    pub fn input_ref(&self) -> &str {
        &self.input
    }

    pub fn into_config(self, conf: &mut CliConfig) {
        conf.0.debug_info(self.debug_info);
        conf.0.entry(self.entry.unwrap_or("_start".into()));
        conf.0.fs_root_path(self.fs_root_path);
        conf.0.runtime_logger(self.runtime_logger);
        conf.0.limited_memory(self.limited_memory);
        conf.0.limited_fuel(self.limited_fuel);
        conf.0.set_run_time(self.run_time);
        conf.0.set_stdin_args(self.args);
        conf.0.permisions(self.permissions);
        conf.0.set_envs(self.envs);
        let mut modules = self.modules;
        if modules.len() > 0 {
            modules.push(BlocklessModule { 
                module_type: ModuleType::Entry, 
                name: String::new(), 
                file: self.input, 
                md5: String::new() 
            });
            conf.0.set_modules(modules);
            conf.0.set_version(blockless::BlocklessConfigVersion::Version1);
        }
    }
}

#[cfg(test)]
mod test {

    #[allow(unused)]
    use super::*;

    #[test]
    fn test_cli_command() {
        let cli = CliCommandOpts::try_parse_from(["cli", "test", "--", "--test=10"]).unwrap();
        assert_eq!(cli.input.as_str(), "test");
        assert_eq!(cli.args.len(), 1);
        assert_eq!(cli.args[0], "--test=10");
    }

    #[test]
    fn test_cli_command_env() {
        let cli = CliCommandOpts::try_parse_from(["cli", "test", "--env", "a=1", "--env", "b=2"]).unwrap();
        assert_eq!(cli.input.as_str(), "test");
        assert_eq!(cli.envs.len(), 2);
        assert_eq!(cli.envs[0], ("a".to_string(), "1".to_string()));
        assert_eq!(cli.envs[1], ("b".to_string(), "2".to_string()));
    }

    #[test]
    fn test_cli_command_permisson() {
        let cli = CliCommandOpts::try_parse_from(["cli", "test", "--permission", "http://www.google.com"]).unwrap();
        assert_eq!(cli.input.as_str(), "test");
        assert_eq!(cli.permissions.len(), 1);
        let perm = Permission {
            schema: "http".to_string(), 
            url: "http://www.google.com".to_string(),
        };
        assert_eq!(cli.permissions[0], perm);
    }

    #[test]
    fn test_cli_command_input() {
        let command_line = r#"blockless_cli test.wasm"#;
        let command_line = command_line
            .split(" ")
            .map(str::to_string)
            .collect::<Vec<String>>();
        let matches = CliCommandOpts::try_parse_from(command_line).unwrap();
        let pat = matches.input.as_str();
        assert_eq!(pat, "test.wasm");
    }

    #[test]
    fn test_cli_command_runtime_log() {
        let command_line = r#"blockless_cli test.wasm --runtime-logger runtime.log"#;
        let command_line = command_line
            .split(" ")
            .map(str::to_string)
            .collect::<Vec<String>>();
        let matches = CliCommandOpts::try_parse_from(command_line).unwrap();
        let pat = matches.runtime_logger.as_ref().unwrap().as_str();
        assert_eq!(pat, "runtime.log");
    }

    #[test]
    fn test_cli_command_fs_root_path() {
        let command_line = r#"blockless_cli test.wasm --fs-root-path /"#;
        let command_line = command_line
            .split(" ")
            .map(str::to_string)
            .collect::<Vec<String>>();
        let matches = CliCommandOpts::try_parse_from(command_line).unwrap();
        let pat = matches.fs_root_path.as_ref().unwrap().as_str();
        assert_eq!(pat, "/");
    }
    
}