use crate::Permission;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

const ENTRY: &str = "_start";

#[derive(Clone, Debug)]
pub enum LoggerLevel {
    INFO,
    WARN,
    DEBUG,
    ERROR,
    TRACE,
}

impl From<&str> for LoggerLevel {
    fn from(value: &str) -> Self {
        match value {
            "debug" | "DEBUG" => LoggerLevel::DEBUG,
            "info" | "INFO" => LoggerLevel::INFO,
            "warn" | "WARN" => LoggerLevel::WARN,
            "trace" | "TRACE" => LoggerLevel::TRACE,
            "error" | "ERROR" => LoggerLevel::ERROR,
            _ => LoggerLevel::INFO,
        }
    }
}

pub enum Stdout {
    //no stdout.
    Null,
    //inherit stdout.
    Inherit,
    //stdout redirect to file.
    FileName(String),
}

pub enum Stderr {
    //no stderr.
    Null,
    //inherit stderr.
    Inherit,
    //stderr redirect to file.
    FileName(String),
}

pub struct DriverConfig {
    schema: String,
    path: String,
}

impl DriverConfig {
    pub fn new(schema: String, path: String) -> DriverConfig {
        DriverConfig { schema, path }
    }

    pub fn schema(&self) -> &str {
        &self.schema
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ModuleType {
    Module,
    Entry,
}

impl PartialEq for ModuleType {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (ModuleType::Module, ModuleType::Module) => true,
            (ModuleType::Module, ModuleType::Entry) => false,
            (ModuleType::Entry, ModuleType::Module) => false,
            (ModuleType::Entry, ModuleType::Entry) => true,
        }
    }
}

impl PartialOrd for ModuleType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (*self, *other) {
            (ModuleType::Module, ModuleType::Module) => Some(std::cmp::Ordering::Equal),
            (ModuleType::Module, ModuleType::Entry) => Some(std::cmp::Ordering::Less),
            (ModuleType::Entry, ModuleType::Module) => Some(std::cmp::Ordering::Greater),
            (ModuleType::Entry, ModuleType::Entry) => Some(std::cmp::Ordering::Equal),
        }
    }
}

impl ModuleType {
    pub fn parse_from_str(s: &str) -> Self {
        match s {
            "entry" | "ENTRY" => Self::Entry,
            _ => Self::Module,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlocklessModule {
    pub module_type: ModuleType,
    pub name: String,
    pub file: String,
    pub md5: String,
}

#[derive(Clone, Copy, Debug)]
pub enum BlocklessConfigVersion {
    Version0,
    Version1,
}

impl From<usize> for BlocklessConfigVersion {
    fn from(value: usize) -> Self {
        match value {
            1 => BlocklessConfigVersion::Version1,
            0 => BlocklessConfigVersion::Version0,
            _ => unreachable!("unknown configure version: {value}."),
        }
    }
}

pub struct BlocklessConfig {
    entry: String,
    stdin: String,
    stdout: Stdout,
    stderr: Stderr,
    debug_info: bool,
    is_carfile: bool,
    support_thread: bool,
    run_time: Option<u64>,
    stdin_args: Vec<String>,
    limited_fuel: Option<u64>,
    limited_time: Option<u64>,
    drivers: Vec<DriverConfig>,
    limited_memory: Option<u64>,
    envs: Vec<(String, String)>,
    permisions: Vec<Permission>,
    fs_root_path: Option<String>,
    modules: Vec<BlocklessModule>,
    runtime_logger: Option<String>,
    extensions_path: Option<String>,
    // the config version
    version: BlocklessConfigVersion,
    drivers_root_path: Option<String>,
    runtime_logger_level: LoggerLevel,
    group_permisions: HashMap<String, Vec<Permission>>,
}

impl BlocklessConfig {
    pub fn new(entry: &str) -> BlocklessConfig {
        Self {
            support_thread: false,
            run_time: None,
            envs: Vec::new(),
            debug_info: false,
            is_carfile: false,
            fs_root_path: None,
            drivers: Vec::new(),
            modules: Vec::new(),
            stdin: String::new(),
            runtime_logger: None,
            //vm instruction limit.
            limited_fuel: None,
            limited_time: None,
            stdin_args: Vec::new(),
            //memory limit, 1 page = 64k.
            limited_memory: None,
            extensions_path: None,
            stderr: Stderr::Inherit,
            drivers_root_path: None,
            stdout: Stdout::Inherit,
            entry: String::from(entry),
            permisions: Default::default(),
            group_permisions: HashMap::new(),
            runtime_logger_level: LoggerLevel::WARN,
            version: BlocklessConfigVersion::Version0,
        }
    }

    #[inline(always)]
    pub fn version(&self) -> BlocklessConfigVersion {
        self.version
    }


    #[inline(always)]
    pub fn feature_thread(&self) -> bool {
        self.feature_thread
    }

    #[inline(always)]
    pub fn set_feature_thread(&mut self, t: bool) {
        self.feature_thread = t;
    }

    #[inline(always)]
    pub fn envs_ref(&self) -> &Vec<(String, String)> {
        self.envs.as_ref()
    }

    #[inline(always)]
    pub fn stdin_args_ref(&self) -> &Vec<String> {
        self.stdin_args.as_ref()
    }

    #[inline(always)]
    pub fn set_envs(&mut self, envs: Vec<(String, String)>) {
        self.envs = envs;
    }

    #[inline(always)]
    pub fn set_stdin_args(&mut self, args: Vec<String>) {
        self.stdin_args = args;
    }

    #[inline(always)]
    pub fn set_entry(&mut self, entry: String) {
        self.entry = entry;
    }

    pub fn entry_module(&self) -> Option<String> {
        let entry_module = match self.version {
            BlocklessConfigVersion::Version0 => Some(self.entry.as_str()),
            BlocklessConfigVersion::Version1 => self
                .modules
                .iter()
                .find(|m| {
                    matches!(m.module_type, ModuleType::Entry)
                })
                .map(|s| s.file.as_str()),
        };
        entry_module.and_then(|s| {
            #[allow(clippy::map_flatten)]
            PathBuf::from_str(s).ok().and_then(|p| {
                p.file_name()
                    .map(|name| name.to_str().map(|s| s.to_string()))
                    .flatten()
            })
        })
    }

    #[inline(always)]
    pub fn set_version(&mut self, version: BlocklessConfigVersion) {
        self.version = version;
    }

    #[inline(always)]
    pub fn run_time(&self) -> Option<u64> {
        self.run_time
    }

    #[inline(always)]
    pub fn set_run_time(&mut self, run_time: Option<u64>) {
        self.run_time = run_time;
    }

    #[inline(always)]
    pub fn get_debug_info(&self) -> bool {
        self.debug_info
    }

    #[inline(always)]
    pub fn set_debug_info(&mut self, b: bool) {
        self.debug_info = b
    }

    #[inline(always)]
    pub fn entry_ref(&self) -> &str {
        &self.entry
    }

    #[inline(always)]
    pub fn get_runtime_logger_level(&self) -> LoggerLevel {
        self.runtime_logger_level.clone()
    }

    #[inline(always)]
    pub fn set_runtime_logger_level(&mut self, level: LoggerLevel) {
        self.runtime_logger_level = level;
    }

    #[inline(always)]
    pub fn set_fs_root_path(&mut self, r: Option<String>) {
        self.fs_root_path = r;
    }

    #[inline(always)]
    pub fn permisions_ref(&self) -> &Vec<Permission> {
        &self.permisions
    }

    #[inline(always)]
    pub fn set_runtime_logger(&mut self, l: Option<String>) {
        self.runtime_logger = l;
    }

    pub fn set_permisions(&mut self, perms: Vec<Permission>) {
        let mut g_perms: HashMap<String, Vec<_>> = HashMap::new();
        perms.iter().for_each(|p| {
            g_perms
                .entry(p.schema.clone())
                .or_insert_with(Vec::new)
                .push(p.clone());
        });
        self.permisions = perms;
        self.group_permisions = g_perms;
    }

    #[inline(always)]
    pub fn fs_root_path_ref(&self) -> Option<&str> {
        self.fs_root_path.as_ref().map(String::as_ref)
    }

    #[inline(always)]
    pub fn drivers_root_path_ref(&self) -> Option<&str> {
        self.drivers_root_path.as_ref().map(String::as_ref)
    }

    #[inline(always)]
    pub fn set_drivers_root_path(&mut self, r: Option<String>) {
        self.drivers_root_path = r;
    }

    #[inline(always)]
    pub fn set_is_carfile(&mut self, is_carfile: bool) {
        self.is_carfile = is_carfile;
    }

    #[inline(always)]
    pub fn get_is_carfile(&self) -> bool {
        self.is_carfile
    }

    #[inline(always)]
    pub fn add_driver(&mut self, d_conf: DriverConfig) {
        self.drivers.push(d_conf)
    }

    #[inline(always)]
    pub fn drivers_ref(&self) -> &[DriverConfig] {
        &self.drivers
    }

    /// stdout file must be work in sandbox root_path,
    /// if root_path is not setting, the stdout file will use Inherit
    #[inline(always)]
    pub fn stdout(&mut self, stdout: Stdout) {
        self.stdout = stdout
    }

    /// the runtime log file name, if the value is None
    /// the runtime log will ouput to Stdout.
    /// the file is in fs_root_path
    #[inline(always)]
    pub fn runtime_logger_path(&self) -> Option<PathBuf> {
        self.fs_root_path
            .as_ref()
            .zip(self.runtime_logger.as_ref())
            .map(|f| Path::new(f.0).join(f.1))
    }

    #[inline(always)]
    pub fn modules_ref(&self) -> Vec<&BlocklessModule> {
        self.modules.iter().collect()
    }

    #[inline(always)]
    pub fn add_module(&mut self, module: BlocklessModule) {
        self.modules.push(module);
    }

    #[inline(always)]
    pub fn reset_modules_model_entry(&mut self) -> &str {
        self.entry = ENTRY.to_string();
        &self.entry
    }

    #[inline(always)]
    pub fn set_modules(&mut self, modules: Vec<BlocklessModule>) {
        self.modules = modules;
    }

    #[inline(always)]
    pub fn stdin(&mut self, stdin: String) {
        self.stdin = stdin
    }

    #[inline(always)]
    pub fn extensions_path(&mut self, extensions_path: Option<String>) {
        self.extensions_path = extensions_path;
    }

    #[inline(always)]
    pub fn drivers(&mut self, drvs: Vec<DriverConfig>) {
        self.drivers = drvs;
    }

    #[inline(always)]
    pub fn stdout_ref(&self) -> &Stdout {
        &self.stdout
    }

    #[inline(always)]
    pub fn stderr_ref(&self) -> &Stderr {
        &self.stderr
    }

    #[inline(always)]
    pub fn stdin_ref(&self) -> &String {
        &self.stdin
    }

    #[inline(always)]
    pub fn limited_time(&mut self, time: Option<u64>) {
        self.limited_time = time
    }

    #[inline(always)]
    pub fn get_limited_time(&self) -> Option<u64> {
        self.limited_time
    }

    #[inline(always)]
    pub fn limited_fuel(&mut self, fuel: Option<u64>) {
        self.limited_fuel = fuel
    }

    #[inline(always)]
    pub fn get_limited_fuel(&self) -> Option<u64> {
        self.limited_fuel
    }

    #[inline(always)]
    pub fn limited_memory(&mut self, m: Option<u64>) {
        self.limited_memory = m
    }

    #[inline(always)]
    pub fn get_limited_memory(&self) -> Option<u64> {
        self.limited_memory
    }

    pub fn resource_permission(&self, url: &str) -> bool {
        self.permisions
            .iter()
            .any(|p| p.is_permision(url))
    }
}

#[cfg(test)]
mod test {

    #![allow(unused_imports)]
    use super::*;

    #[test]
    fn test_config() {
        let mut config = BlocklessConfig::new("test");
        assert!(matches!(config.version(), BlocklessConfigVersion::Version0));
        let permisions = vec![
            Permission {
                url: "/test1".to_string(),
                schema: "http".to_string(),
            },
            Permission {
                url: "/test2".to_string(),
                schema: "http".to_string(),
            },
        ];
        config.set_permisions(permisions);
        let grps = config.group_permisions.get("http");
        if let Some(grps) = grps {
            assert_eq!(grps.len(), 2);
        } else {
            unreachable!("should not reach.");
        }
        let root = Some("/root".into());
        config.set_fs_root_path(root);
        let test = Some("test.log".into());
        config.set_runtime_logger(test);
        let result = PathBuf::new().join("/root").join("test.log");
        assert_eq!(config.runtime_logger_path().unwrap(), result);

        assert_eq!(config.entry_ref(), "test");
        config.set_entry("_start".into());
        assert_eq!(config.entry_ref(), "_start");
    }

    #[test]
    fn test_version_convert() {
        let _version0: BlocklessConfigVersion = 0.into();
        let matched = matches!(BlocklessConfigVersion::Version0, _version0);
        assert!(matched);

        let _version1: BlocklessConfigVersion = 1.into();
        let matched = matches!(BlocklessConfigVersion::Version1, _version1);
        assert!(matched);
    }

    #[test]
    fn test_logger_level_convert() {
        let ty = "debug".into();
        assert!(matches!(ty, LoggerLevel::DEBUG));

        let ty = "info".into();
        assert!(matches!(ty, LoggerLevel::INFO));

        let ty = "error".into();
        assert!(matches!(ty, LoggerLevel::ERROR));

        let ty = "warn".into();
        assert!(matches!(ty, LoggerLevel::WARN));

        let ty = "trace".into();
        assert!(matches!(ty, LoggerLevel::TRACE));
    }
}
