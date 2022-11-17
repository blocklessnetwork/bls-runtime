use json::JsonValue;

use crate::CgiErrorKind;


pub struct CgiProcess {
    command: String,
    args: Vec<String>,
    env: Vec<String>,
}

impl CgiProcess {
    pub fn new(cmd_with_params: &str) -> Result<Self, CgiErrorKind> {

        let obj = match json::parse(cmd_with_params) {
            Ok(o) => o,
            Err(_) => return Err(CgiErrorKind::InvalidParameter),
        };
        let command = match obj["command"].as_str() {
            Some(s) => String::from(s),
            None => return Err(CgiErrorKind::InvalidParameter),
        };
        let collect_json = |b: &JsonValue| {
            match b {
                &json::JsonValue::Array(ref args) => {
                    args.iter().map(|arg| {
                        arg.as_str().map(String::from)
                    })
                    .filter(Option::is_some)
                    .map(Option::unwrap)
                    .collect()
                }
                _ => Vec::new(),
            }
        };
        let args: Vec<String> = collect_json(&obj["args"]);
        let env: Vec<String> = collect_json(&obj["env"]);
        Ok(Self {
            command,
            args,
            env,
        })
    }
}