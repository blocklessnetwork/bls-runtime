use crate::Permission;
use anyhow::{bail, Ok};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
};
use wasmtime::OptLevel;

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

#[derive(Debug, Clone)]
pub enum Stdin {
    Inherit,
    Fixed(String),
}

#[derive(Debug, Clone)]
pub enum Stdout {
    //no stdout.
    Null,
    //inherit stdout.
    Inherit,
    //stdout redirect to file.
    FileName(String),
}

#[derive(Debug, Clone)]
pub enum Stderr {
    //no stderr.
    Null,
    //inherit stderr.
    Inherit,
    //stderr redirect to file.
    FileName(String),
}

#[derive(Clone)]
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

#[derive(Default, Clone)]
pub struct StoreLimited {
    pub max_memory_size: Option<usize>,
    pub max_table_elements: Option<u32>,
    pub max_instances: Option<usize>,
    pub max_tables: Option<u32>,
    pub max_memories: Option<usize>,
    pub trap_on_grow_failure: Option<bool>,
}

pub trait BlsOptions {
    const OPTIONS: &'static [OptionDesc];
}

pub struct OptionDesc {
    pub opt_name: &'static str,
    pub opt_docs: &'static str,
}

macro_rules! bls_options {
    (
        $(#[$attr:meta])*
        pub struct $opts:ident {
            $(
                $(#[doc = $doc:tt])*
                pub $opt:ident: $container:ident<$payload:ty>,
            )+
        }
    ) => {
        #[derive(Default, Debug)]
        $(#[$attr])*
        pub struct $opts {
            $(
                pub $opt: $container<$payload>,
            )+
        }

        impl $opts {
            pub fn config(&mut self, items: Vec<(String, String)>) -> anyhow::Result<()> {
                for item in items.iter() {
                    match item.0.as_str()  {
                        $(
                        stringify!($opt) => self.$opt = Some(OptionParser::<String>::parse(&item.1)?),
                        )+
                        _ => bail!("there is no optimize argument: {}", item.0),
                    }
                }
                Ok(())
            }

            pub fn is_empty(&self) -> bool {
                *self == Default::default()
            }
        }

        impl BlsOptions for $opts {
            const OPTIONS: &'static [OptionDesc] = &[
                $(
                    OptionDesc {
                        opt_name: stringify!($opt),
                        opt_docs: concat!($($doc, "\n", )*),
                    },
                )+
            ];
        }
    }
}

pub trait OptionParser<T>: Sized {
    fn parse(v: &T) -> anyhow::Result<Self>
    where
        T: Sized;
}

impl OptionParser<String> for u32 {
    fn parse(val: &String) -> anyhow::Result<Self> {
        match val.strip_prefix("0x") {
            Some(hex) => Ok(u32::from_str_radix(hex, 16)?),
            None => Ok(val.parse()?),
        }
    }
}

impl OptionParser<String> for usize {
    fn parse(val: &String) -> anyhow::Result<Self> {
        match val.strip_prefix("0x") {
            Some(hex) => Ok(usize::from_str_radix(hex, 16)?),
            None => Ok(val.parse()?),
        }
    }
}

impl OptionParser<String> for u64 {
    fn parse(val: &String) -> anyhow::Result<Self> {
        match val.strip_prefix("0x") {
            Some(hex) => Ok(u64::from_str_radix(hex, 16)?),
            None => Ok(val.parse()?),
        }
    }
}

impl OptionParser<String> for OptLevel {
    fn parse(v: &String) -> anyhow::Result<Self> {
        match v.as_str() {
            "n" => Ok(OptLevel::None),
            "s" => Ok(OptLevel::Speed),
            "ss" => Ok(OptLevel::SpeedAndSize),
            _ => bail!(
                "unknown optimization level {v}, level must be n:None, s:Speed, ss:SpeedAndSize"
            ),
        }
    }
}

impl OptionParser<String> for bool {
    fn parse(val: &String) -> anyhow::Result<Self> {
        match val.as_str() {
            "y" | "yes" | "true" => Ok(true),
            "n" | "no" | "false" => Ok(false),
            s @ _ => bail!("unknown boolean flag `{s}`, only yes,no,<nothing> accepted"),
        }
    }
}

impl OptionParser<String> for wasmtime::RegallocAlgorithm {
    fn parse(val: &String) -> anyhow::Result<Self> {
        match val.as_str() {
            "backtracking" => Ok(wasmtime::RegallocAlgorithm::Backtracking),
            "single-pass" => Ok(wasmtime::RegallocAlgorithm::SinglePass),
            other => bail!(
                "unknown regalloc algorithm`{}`, only backtracking,single-pass accepted",
                other
            ),
        }
    }
}

bls_options! {
    #[derive(PartialEq, Clone)]
    pub struct OptimizeOpts {
        /// Optimization level of generated code (0-2, s; default: 2)
        pub opt_level: Option<wasmtime::OptLevel>,

        /// Register allocator algorithm choice.
        pub regalloc_algorithm: Option<wasmtime::RegallocAlgorithm>,

        /// Do not allow Wasm linear memories to move in the host process's
        /// address space.
        pub memory_may_move: Option<bool>,

        /// Initial virtual memory allocation size for memories.
        pub memory_reservation: Option<u64>,

        /// Bytes to reserve at the end of linear memory for growth into.
        pub memory_reservation_for_growth: Option<u64>,

        /// Size, in bytes, of guard pages for linear memories.
        pub memory_guard_size: Option<u64>,

        /// Indicates whether an unmapped region of memory is placed before all
        /// linear memories.
        pub guard_before_linear_memory: Option<bool>,

        /// Whether to initialize tables lazily, so that instantiation is
        /// fast but indirect calls are a little slower. If no, tables are
        /// initialized eagerly from any active element segments that apply to
        /// them during instantiation. (default: yes)
        pub table_lazy_init: Option<bool>,

        /// Enable the pooling allocator, in place of the on-demand allocator.
        pub pooling_allocator: Option<bool>,

        /// The number of decommits to do per batch. A batch size of 1
        /// effectively disables decommit batching. (default: 1)
        pub pooling_decommit_batch_size: Option<usize>,

        /// How many bytes to keep resident between instantiations for the
        /// pooling allocator in linear memories.
        pub pooling_memory_keep_resident: Option<usize>,

        /// How many bytes to keep resident between instantiations for the
        /// pooling allocator in tables.
        pub pooling_table_keep_resident: Option<usize>,

        /// Enable memory protection keys for the pooling allocator; this can
        /// optimize the size of memory slots.
        pub pooling_memory_protection_keys: Option<bool>,

        /// Sets an upper limit on how many memory protection keys (MPK) Wasmtime
        /// will use. (default: 16)
        pub pooling_max_memory_protection_keys: Option<usize>,

        /// Configure attempting to initialize linear memory via a
        /// copy-on-write mapping (default: yes)
        pub memory_init_cow: Option<bool>,

        /// The maximum number of WebAssembly instances which can be created
        /// with the pooling allocator.
        pub pooling_total_core_instances: Option<u32>,

        /// The maximum number of WebAssembly components which can be created
        /// with the pooling allocator.
        pub pooling_total_component_instances: Option<u32>,

        /// The maximum number of WebAssembly memories which can be created with
        /// the pooling allocator.
        pub pooling_total_memories: Option<u32>,

        /// The maximum number of WebAssembly tables which can be created with
        /// the pooling allocator.
        pub pooling_total_tables: Option<u32>,

        /// The maximum number of WebAssembly stacks which can be created with
        /// the pooling allocator.
        pub pooling_total_stacks: Option<u32>,

        /// The maximum runtime size of each linear memory in the pooling
        /// allocator, in bytes.
        pub pooling_max_memory_size: Option<usize>,

        /// The maximum table elements for any table defined in a module when
        /// using the pooling allocator.
        pub pooling_table_elements: Option<usize>,

        /// The maximum size, in bytes, allocated for a core instance's metadata
        /// when using the pooling allocator.
        pub pooling_max_core_instance_size: Option<usize>,

        /// Configures the maximum number of "unused warm slots" to retain in the
        /// pooling allocator. (default: 100)
        pub pooling_max_unused_warm_slots: Option<u32>,

        /// Configures whether or not stacks used for async futures are reset to
        /// zero after usage. (default: false)
        pub pooling_async_stack_zeroing: Option<bool>,

        /// How much memory, in bytes, to keep resident for async stacks allocated
        /// with the pooling allocator. (default: 0)
        pub pooling_async_stack_keep_resident: Option<usize>,

        /// The maximum size, in bytes, allocated for a component instance's
        /// `VMComponentContext` metadata. (default: 1MiB)
        pub pooling_max_component_instance_size: Option<usize>,

        /// The maximum number of core instances a single component may contain
        /// (default is unlimited).
        pub pooling_max_core_instances_per_component: Option<u32>,

        /// The maximum number of Wasm linear memories that a single component may
        /// transitively contain (default is unlimited).
        pub pooling_max_memories_per_component: Option<u32>,

        /// The maximum number of tables that a single component may transitively
        /// contain (default is unlimited).
        pub pooling_max_tables_per_component: Option<u32>,

        /// The maximum number of defined tables for a core module. (default: 1)
        pub pooling_max_tables_per_module: Option<u32>,

        /// The maximum number of defined linear memories for a module. (default: 1)
        pub pooling_max_memories_per_module: Option<u32>,

        /// The maximum number of concurrent GC heaps supported. (default: 1000)
        pub pooling_total_gc_heaps: Option<u32>,

        /// Enable or disable the use of host signal handlers for traps.
        pub signals_based_traps: Option<bool>,

        /// DEPRECATED: Use `-Cmemory-guard-size=N` instead.
        pub dynamic_memory_guard_size: Option<u64>,

        /// DEPRECATED: Use `-Cmemory-guard-size=N` instead.
        pub static_memory_guard_size: Option<u64>,

        /// DEPRECATED: Use `-Cmemory-may-move` instead.
        pub static_memory_forced: Option<bool>,

        /// DEPRECATED: Use `-Cmemory-reservation=N` instead.
        pub static_memory_maximum_size: Option<u64>,

        /// DEPRECATED: Use `-Cmemory-reservation-for-growth=N` instead.
        pub dynamic_memory_reserved_for_growth: Option<u64>,
    }
}

#[derive(Clone)]
pub struct Stdio {
    pub stdin: Stdin,
    pub stdout: Stdout,
    pub stderr: Stderr,
}

impl Default for Stdio {
    fn default() -> Self {
        Stdio {
            stdin: Stdin::Fixed(String::new()),
            stdout: Stdout::Inherit,
            stderr: Stderr::Inherit,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlsNnGraph {
    pub format: String,
    pub dir: String,
}

#[derive(Clone)]
pub struct BlocklessConfig {
    pub entry: String,
    pub nn: bool,
    pub stdio: Stdio,
    pub debug_info: bool,
    pub is_carfile: bool,
    pub opts: OptimizeOpts,
    pub feature_thread: bool,
    pub run_time: Option<u64>,
    pub nn_graph: Vec<BlsNnGraph>,
    pub stdin_args: Vec<String>,
    pub coredump: Option<String>,
    pub limited_fuel: Option<u64>,
    pub limited_time: Option<u64>,
    pub drivers: Vec<DriverConfig>,
    pub unknown_imports_trap: bool,
    pub store_limited: StoreLimited,
    pub envs: Vec<(String, String)>,
    pub tcp_listens: Vec<(SocketAddr, Option<u32>)>,
    pub permisions: Vec<Permission>,
    pub dirs: Vec<(String, String)>,
    pub fs_root_path: Option<String>,
    pub modules: Vec<BlocklessModule>,
    pub runtime_logger: Option<String>,
    pub extensions_path: Option<String>,
    // the config version
    pub version: BlocklessConfigVersion,
    pub drivers_root_path: Option<String>,
    pub runtime_logger_level: LoggerLevel,
    pub cli_exit_with_code: bool,
    pub network_error_code: bool,
    pub group_permisions: HashMap<String, Vec<Permission>>,
}

impl BlocklessConfig {
    pub fn new(entry: &str) -> BlocklessConfig {
        Self {
            nn: false,
            run_time: None,
            coredump: None,
            envs: Vec::new(),
            debug_info: false,
            dirs: Vec::new(),
            is_carfile: false,
            fs_root_path: None,
            drivers: Vec::new(),
            modules: Vec::new(),
            stdio: Default::default(),
            runtime_logger: None,
            feature_thread: false,
            //vm instruction limit.
            limited_fuel: None,
            limited_time: None,
            // define the base fd
            tcp_listens: Vec::new(),
            stdin_args: Vec::new(),
            cli_exit_with_code: false,
            network_error_code: false,
            //memory limit, 1 page = 64k.
            store_limited: Default::default(),
            extensions_path: None,
            drivers_root_path: None,
            unknown_imports_trap: false,
            nn_graph: Vec::new(),
            entry: String::from(entry),
            permisions: Default::default(),
            group_permisions: HashMap::new(),
            opts: Default::default(),
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
    pub fn set_map_dirs(&mut self, dirs: Vec<(String, String)>) {
        self.dirs = dirs;
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
                .find(|m| matches!(m.module_type, ModuleType::Entry))
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
        self.stdio.stdout = stdout
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
    pub fn fixed_stdin(&mut self, stdin: String) {
        self.stdio.stdin = Stdin::Fixed(stdin);
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
    pub fn is_fixed_stdin(&self) -> bool {
        match self.stdio.stdin {
            Stdin::Fixed(_) => true,
            _ => false,
        }
    }

    #[inline(always)]
    pub fn stdout_ref(&self) -> &Stdout {
        &self.stdio.stdout
    }

    #[inline(always)]
    pub fn stderr_ref(&self) -> &Stderr {
        &self.stdio.stderr
    }

    #[inline(always)]
    pub fn fix_stdin_ref(&self) -> Option<&str> {
        match self.stdio.stdin {
            Stdin::Fixed(ref s) => Some(s.as_str()),
            _ => None,
        }
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
        self.store_limited.max_memories = m.map(|s| s as _);
    }

    #[inline(always)]
    pub fn get_limited_memory(&self) -> Option<u64> {
        self.store_limited.max_memories.map(|m| m as u64)
    }

    pub fn resource_permission(&self, url: &str) -> bool {
        self.permisions.iter().any(|p| p.is_permision(url))
    }

    #[inline(always)]
    pub fn store_limited(&self) -> &StoreLimited {
        &self.store_limited
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
