use crate::CgiErrorKind;
use log::debug;
use tokio::process::{Child, Command};

pub struct CgiProcess {
    child: Option<Child>,
    command: String,
    args: Vec<String>,
    envs: Vec<(String, String)>,
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
        let args = match obj["args"] {
            json::JsonValue::Array(ref args) => args
                .iter()
                .map(|arg| arg.as_str().map(String::from))
                .filter(Option::is_some)
                .map(Option::unwrap)
                .collect(),
            _ => Vec::new(),
        };
        let envs = match obj["envs"] {
            json::JsonValue::Array(ref args) => args
                .iter()
                .map(|arg| {
                    arg["env_name"]
                        .as_str()
                        .map(String::from)
                        .zip(arg["env_val"].as_str().map(String::from))
                })
                .filter(Option::is_some)
                .map(Option::unwrap)
                .collect(),
            _ => Vec::new(),
        };
        Ok(Self {
            child: None,
            command,
            args,
            envs,
        })
    }

    pub async fn kill(&mut self) -> Result<(), CgiErrorKind> {
        if self.child.is_some() {
            return self
                .child
                .as_mut()
                .unwrap()
                .kill()
                .await
                .map_err(|_| CgiErrorKind::RuntimeError);
        }
        Ok(())
    }

    pub fn exec(&mut self) -> Result<(), CgiErrorKind> {
        let mut command = Command::new(&self.command);
        command.kill_on_drop(true);
        command.args(self.args.iter());
        command.envs(self.envs.clone());
        self.child = match command.spawn() {
            Ok(o) => Some(o),
            Err(e) => {
                debug!("error exec command: {}", e);
                return Err(CgiErrorKind::RuntimeError);
            }
        };
        Ok(())
    }
}
