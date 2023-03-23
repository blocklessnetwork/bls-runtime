use anyhow::Result;
use blockless::{self, LoggerLevel, BlocklessModule, ModuleType};
use blockless::{BlocklessConfig, DriverConfig, MultiAddr, Permission};
use json::{self, JsonValue};
use std::fs;
use std::path::{PathBuf, Path};

pub(crate) struct CliConfig(pub(crate) BlocklessConfig);

struct EnvVar {
    name: String,
    value: String,
}

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
                    let md5 = c["md5"].as_str().map(String::from).unwrap_or_default();
                    let module_type = c["type"].as_str().map(|s| {
                        ModuleType::parse_from_str(s)
                    })
                    .unwrap_or(ModuleType::Module);
                    BlocklessModule { 
                        module_type, 
                        name, 
                        file,
                        md5,
                    }
                })
                .collect()
            }
            _ => Vec::new(),
        }
    }

    fn from_json_string(json_string: String) -> Result<Self> {
        let json_obj = json::parse(&json_string)?;
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
        let run_time: Option<u64> = json_obj["run_time"].as_u64();
        
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
        bc.set_run_time(run_time);

        if stdin.is_some() {
            bc.stdin(stdin.unwrap().to_string());
        }
        Ok(CliConfig(bc))
    }

    fn env_variables(cid: Option<String>) -> Result<Vec<EnvVar>> {
        let mut vars = Vec::new();
        match std::env::var("ENV_ROOT_PATH") {
            Ok(s) => {
                let env_root = s.clone();
                let path: PathBuf = s.into();
                let root_path = path.join(cid.unwrap_or_default());
                let path: String = root_path.to_str().unwrap_or_default().into();
                vars.push(EnvVar { 
                    name: "$ROOT".to_string(), 
                    value: path,
                });
                vars.push(EnvVar {
                    name: "$ENV_ROOT_PATH".to_string(),
                    value: env_root,
                });
            },
            Err(e) => return Err(e.into()),
        }
        Ok(vars)
    }

    fn replace_vars(json_str: String,  root_suffix: Option<String>) -> Result<String> {
        let vars = Self::env_variables(root_suffix)?;
        let mut raw_json = json_str;
        for var in vars {
            raw_json = raw_json.replace(&var.name, &var.value);
        }
        Ok(raw_json)
    }

    pub fn from_data(data: String, root_suffix: Option<String>) -> Result<Self> {
        let data = Self::replace_vars(data, root_suffix)?;
        Self::from_json_string(data)
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let values = fs::read_to_string(path)?;
        let json_string = Self::replace_vars(values, None)?;
        Self::from_json_string(json_string)
    }
}
