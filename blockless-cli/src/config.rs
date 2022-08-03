use anyhow::Result;
use blockless;
use blockless::{BlocklessConfig, DriverConfig, MultiAddr, Permission};
use json::{self, JsonValue};
use std::fs;

pub(crate) struct CliConfig(pub(crate) BlocklessConfig);

impl CliConfig {
    pub fn from_file(path: &str) -> Result<Self> {
        let values = fs::read(path)?;
        let file = std::str::from_utf8(&values)?;
        let json_obj = json::parse(file)?;
        let fs_root_path: Option<String> = json_obj["fs_root_path"].as_str().map(|s| s.into());
        let drivers_root_path: Option<String> =
            json_obj["drivers_root_path"].as_str().map(|s| s.into());
        let limited_fuel: Option<u64> = json_obj["limited_fuel"].as_u64();
        let limited_memory: Option<u64> = json_obj["limited_memory"].as_u64();
        let stdin: Option<&str> = Some(json_obj["stdin"].as_str()).unwrap_or(None);

        let drvs = match json_obj["drivers"] {
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
            _ => Vec::<DriverConfig>::new(),
        };

        let perms: Vec<Permission> = match json_obj["permissions"] {
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
        };

        let entry: &str = json_obj["entry"].as_str().unwrap();
        let mut bc = BlocklessConfig::new(entry);
        bc.fs_root_path(fs_root_path);
        bc.drivers(drvs);
        bc.permisions(perms);
        bc.drivers_root_path(drivers_root_path);
        bc.limited_fuel(limited_fuel);
        bc.limited_memory(limited_memory);
        
        if stdin.is_some() {
            bc.stdin(stdin.unwrap().to_string());
        }

        Ok(CliConfig(bc))
    }
}
