use blockless::BlocklessConfig;
use anyhow::Result;
use std::fs;
use json;


pub(crate) struct CliConfig(pub(crate)BlocklessConfig);

impl CliConfig {

    pub fn from_file(path: &str) -> Result<Self> {
        let values = fs::read(path)?;
        let file = std::str::from_utf8(&values)?;
        let json_obj = json::parse(file)?;
        let root_path: Option<String> = json_obj["root_path"].as_str().map(|s| s.into());
        let limited_fuel: Option<u64> = json_obj["limited_fuel"].as_u64();
        let limited_memory: Option<u64> = json_obj["limited_memory"].as_u64();
        let entry: &str = json_obj["entry"].as_str().unwrap();
        let mut bc = BlocklessConfig::new(entry);
        bc.root_path(root_path);
        bc.limited_fuel(limited_fuel);
        bc.limited_memory(limited_memory);
        Ok(CliConfig(bc))
    }

}