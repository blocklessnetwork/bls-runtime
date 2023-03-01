use anyhow::Result;
use blockless::{self, LoggerLevel, BlocklessModule, ModuleType};
use blockless::{BlocklessConfig, DriverConfig, MultiAddr, Permission};
use json::{self, JsonValue};
use std::fs;

pub(crate) struct CliConfig(pub(crate) BlocklessConfig);

impl CliConfig {

    fn permissions(permission_json: &JsonValue) -> Vec<Permission> {
        match *permission_json {
            JsonValue::Array(ref perms) => perms
                .iter()
                .filter_map(|p| {
                    let p = p.as_str();
                    match p {
                        Some(p) => {
                            let bs = p.as_bytes();
                            let addr = MultiAddr::parse(bs);
                            let addr = if addr.is_ok() {
                                addr.unwrap()
                            } else {
                                return None;
                            };
                            let schema = addr.schema();
                            let schema = if schema.is_ok() {
                                schema.unwrap()
                            } else {
                                return None;
                            };
                            Some(Permission {
                                schema: schema.into(),
                                url: p.into(),
                            })
                        }
                        None => None,
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn drivers(driver_json: &JsonValue) -> Vec<DriverConfig> {
        match *driver_json {
            JsonValue::Array(ref drvs_cfg) => {
                let cfgs: Vec<DriverConfig> = drvs_cfg
                    .iter()
                    .map(|c| {
                        let schema = c["schema"].as_str().map(String::from);
                        let path = c["path"].as_str().map(String::from);
                        (schema, path)
                    })
                    .filter(|(s, p)| s.is_some() && p.is_some())
                    .map(|(s, p)| DriverConfig::new(s.unwrap(), p.unwrap()))
                    .collect::<Vec<_>>();
                cfgs
            }
            _ => Vec::new(),
        }
    }

    fn modules(modules: &JsonValue) -> Vec<BlocklessModule> {
        match *modules {
            JsonValue::Array(ref modules_cfg) => {
                modules_cfg.iter().map(|c| {
                    let name = c["name"].as_str().map(String::from).unwrap_or_default();
                    let file = c["file"].as_str().map(String::from).unwrap_or_default();
                    let module_type = c["module_type"].as_str().map(|s| {
                        ModuleType::from_str(s)
                    })
                    .unwrap_or(ModuleType::Module);
                    BlocklessModule { 
                        module_type, 
                        name, 
                        file
                    }
                })
                .collect()
            }
            _ => Vec::new(),
        }
    }

    fn from_json_string(json_string: &str) -> Result<Self> {
        let json_obj = json::parse(json_string)?;
        let fs_root_path: Option<String> = json_obj["fs_root_path"].as_str().map(String::from);
        let drivers_root_path: Option<String> =
            json_obj["drivers_root_path"].as_str().map(String::from);
        let limited_fuel: Option<u64> = json_obj["limited_fuel"].as_u64();
        let runtime_logger = json_obj["runtime_logger"].as_str().map(String::from);
        let runtime_logger_level = json_obj["runtime_logger_level"].as_str().map(LoggerLevel::from);
        let limited_memory: Option<u64> = json_obj["limited_memory"].as_u64();
        let extensions_path: Option<String> = json_obj["extensions_path"].as_str().map(String::from);
        let stdin: Option<&str> = Some(json_obj["stdin"].as_str()).unwrap_or(None);
        let stdout: Option<String> = json_obj["stdout"].as_str().map(String::from);
        let debug_info: Option<bool> = json_obj["debug_info"].as_bool();
        
        let drvs = Self::drivers(&json_obj["drivers"]);
        let modules = Self::modules(&json_obj["modules"]);
        let perms: Vec<Permission> = Self::permissions(&json_obj["permissions"]);
        let entry: &str = json_obj["entry"].as_str().unwrap();
        let mut bc = BlocklessConfig::new(entry);
        bc.set_modules(modules);
        bc.extensions_path(extensions_path);
        bc.fs_root_path(fs_root_path);
        bc.drivers(drvs);
        stdout.map(|filename: String| {
            bc.stdout(blockless::Stdout::FileName(filename));
        });
        // the set debug mode
        debug_info.map(|b| bc.debug_info(b));
        runtime_logger_level.map(|l| bc.runtime_logger_level(l));
        bc.permisions(perms);
        bc.runtime_logger(runtime_logger);
        bc.drivers_root_path(drivers_root_path);
        bc.limited_fuel(limited_fuel);
        bc.limited_memory(limited_memory);

        if stdin.is_some() {
            bc.stdin(stdin.unwrap().to_string());
        }
        Ok(CliConfig(bc))
    }

    pub fn from_data(data: Vec<u8>) -> Result<Self> {
        let json_string = std::str::from_utf8(&data[..])?;
        Self::from_json_string(json_string)
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let values = fs::read(path)?;
        let json_string = std::str::from_utf8(&values)?;
        Self::from_json_string(json_string)
    }
}
