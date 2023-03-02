use crate::Permission;
use std::{
    collections::HashMap,
    path::{Path, PathBuf}, 
};

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
            "debug"|"DEBUG" => LoggerLevel::DEBUG,
            "info"|"INFO" => LoggerLevel::INFO,
            "warn"|"WARN" => LoggerLevel::WARN,
            "trace"|"TRACE" => LoggerLevel::TRACE,
            "error"|"ERROR" => LoggerLevel::ERROR,
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

impl ModuleType {

    pub fn from_str(s: &str) -> Self {
        match s {
            "entry" | "ENTRY" => Self::Entry,
            _ => Self::Module,
        }
    }

}

#[derive(Debug)]
pub struct BlocklessModule {
    pub module_type: ModuleType,
    pub name: String,
    pub file: String,
}

pub struct BlocklessConfig {
    stdin: String,
    stdout: Stdout,
    debug_info: bool,
    is_carfile: bool,
    wasm_file: String,
    limited_fuel: Option<u64>,
    limited_time: Option<u64>,
    limited_memory: Option<u64>,
    drivers: Vec<DriverConfig>,
    permisions: Vec<Permission>,
    fs_root_path: Option<String>,
    modules: Vec<BlocklessModule>,
    runtime_logger: Option<String>,
    runtime_logger_level: LoggerLevel,
    extensions_path: Option<String>,
    drivers_root_path: Option<String>,
    entry_module_index: Option<usize>,
    group_permisions: HashMap<String, Vec<Permission>>,
}

impl BlocklessConfig {

    pub fn get_debug_info(&self) -> bool {
        self.debug_info
    }

    pub fn debug_info(&mut self, b: bool) {
        self.debug_info = b
    }

    pub fn wasm_file_ref(&self) -> &str {
        &self.wasm_file
    }

    pub fn runtime_logger_level_ref(&self) -> &LoggerLevel {
        &self.runtime_logger_level
    }

    pub fn runtime_logger_level(&mut self, level: LoggerLevel) {
        self.runtime_logger_level = level;
    }

    pub fn fs_root_path(&mut self, r: Option<String>) {
        self.fs_root_path = r;
    }

    pub fn permisions_ref(&self) -> &Vec<Permission> {
        &self.permisions
    }

    pub fn runtime_logger(&mut self, l: Option<String>) {
        self.runtime_logger = l;
    }
    
    pub fn permisions(&mut self, perms: Vec<Permission>) {
        let mut g_perms: HashMap<String, Vec<_>> = HashMap::new();
        perms.iter().for_each(|p| {
            g_perms
                .entry(p.schema.clone())
                .or_insert_with(|| Vec::new())
                .push(p.clone());
        });
        self.permisions = perms;
        self.group_permisions = g_perms;
    }

    pub fn fs_root_path_ref(&self) -> Option<&str> {
        self.fs_root_path.as_ref().map(String::as_ref)
    }

    pub fn drivers_root_path_ref(&self) -> Option<&str> {
        self.drivers_root_path.as_ref().map(String::as_ref)
    }

    pub fn drivers_root_path(&mut self, r: Option<String>) {
        self.drivers_root_path = r;
    }

    pub fn set_is_carfile(&mut self, is_carfile: bool) {
        self.is_carfile = is_carfile;
    }

    pub fn is_carfile(&self) -> bool {
        self.is_carfile
    }

    pub fn new(wasm_file: &str) -> BlocklessConfig {
        Self {
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
            //memory limit, 1 page = 64k.
            limited_memory: None,
            extensions_path: None,
            drivers_root_path: None,
            stdout: Stdout::Inherit,
            entry_module_index: None,
            permisions: Default::default(),
            group_permisions: HashMap::new(),
            wasm_file: String::from(wasm_file),
            runtime_logger_level: LoggerLevel::INFO,
        }
    }

    pub fn add_driver(&mut self, d_conf: DriverConfig) {
        self.drivers.push(d_conf)
    }

    pub fn drivers_ref(&self) -> &[DriverConfig] {
        &self.drivers
    }

    /// stdout file must be work in sandbox root_path,
    /// if root_path is not setting, the stdout file will use Inherit
    pub fn stdout(&mut self, stdout: Stdout) {
        self.stdout = stdout
    }

    /// the runtime log file name, if the value is None
    /// the runtime log will ouput to Stdout.
    /// the file is in fs_root_path
    pub fn runtime_logger_ref(&self) -> Option<PathBuf> {
        self.fs_root_path
            .as_ref()
            .zip(self.runtime_logger.as_ref())
            .map(|f| Path::new(f.0).join(f.1))
    }

    pub fn modules_ref(&self) -> Vec<&BlocklessModule> {
        self.modules.iter().map(|s| s).collect()
    }

    pub fn add_module(&mut self, module: BlocklessModule) {
        self.modules.push(module);
    }

    pub fn set_modules(&mut self, modules: Vec<BlocklessModule>) {
        for (i, e) in modules.iter().enumerate() {
            if e.name == self.wasm_file {
                self.entry_module_index = Some(i);
            }
        }
        self.modules = modules;
    }

    pub fn wasm_file_module(&self) -> Option<&BlocklessModule> {
        self.entry_module_index.map(|i| &self.modules[i])
    }

    pub fn stdin(&mut self, stdin: String) {
        self.stdin = stdin
    }

    pub fn extensions_path(&mut self, extensions_path: Option<String>) {
        self.extensions_path = extensions_path;
    }

    pub fn drivers(&mut self, drvs: Vec<DriverConfig>) {
        self.drivers = drvs;
    }

    pub fn stdout_ref(&self) -> &Stdout {
        &self.stdout
    }

    pub fn stdin_ref(&self) -> &String {
        &self.stdin
    }

    pub fn limited_time(&mut self, time: Option<u64>) {
        self.limited_time = time
    }

    pub fn get_limited_time(&self) -> Option<u64> {
        self.limited_time
    }

    pub fn limited_fuel(&mut self, fuel: Option<u64>) {
        self.limited_fuel = fuel
    }

    pub fn get_limited_fuel(&self) -> Option<u64> {
        self.limited_fuel
    }

    pub fn limited_memory(&mut self, m: Option<u64>) {
        self.limited_memory = m
    }

    pub fn get_limited_memory(&self) -> Option<u64> {
        self.limited_memory
    }
}
