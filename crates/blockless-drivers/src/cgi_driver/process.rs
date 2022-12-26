use std::process::Stdio;

use crate::CgiErrorKind;
use json::object::Object as JsonObject;
use json::JsonValue;
use log::debug;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
};

pub struct CgiProcess {
    root_path: String,
    child: Option<Child>,
    command: String,
    args: Vec<String>,
    envs: Vec<(String, String)>,
}

impl CgiProcess {
    
    /// create a CgiProcess with arguments and envriment variables .
    pub fn new(root_path: String, cmd_with_params: &str) -> Result<Self, CgiErrorKind> {
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
            root_path,
            command,
            args,
            envs,
        })
    }

    /// read bytes from the stdout .
    pub async fn child_stdout_read(&mut self, buf: &mut [u8]) -> Result<u32, CgiErrorKind> {
        if self.child.is_some() {
            let child = self.child.as_mut().unwrap();
            if child.stdout.is_some() {
                let stdout = child.stdout.as_mut().unwrap();
                return stdout.read(buf).await.map(|i| i as u32).map_err(|e| {
                    debug!("error read stdout {}", e);
                    CgiErrorKind::RuntimeError
                });
            } else {
                return Ok(0);
            }
        }
        Err(CgiErrorKind::InvalidHandle)
    }

    /// read bytes from the stderr .
    pub async fn child_stderr_read(&mut self, buf: &mut [u8]) -> Result<u32, CgiErrorKind> {
        if self.child.is_some() {
            let child = self.child.as_mut().unwrap();
            if child.stderr.is_some() {
                let stderr = child.stderr.as_mut().unwrap();
                return stderr.read(buf).await.map(|i| i as u32).map_err(|e| {
                    debug!("error read stderr {}", e);
                    CgiErrorKind::RuntimeError
                });
            } else {
                return Ok(0);
            }
        }
        Err(CgiErrorKind::InvalidHandle)
    }

    /// write buf bytes to the stdin .
    pub async fn child_stdin_write(&mut self, buf: &[u8]) -> Result<u32, CgiErrorKind> {
        if self.child.is_some() {
            let child = self.child.as_mut().unwrap();
            if child.stdin.is_some() {
                let stdin = child.stdin.as_mut().unwrap();
                return stdin.write(buf).await.map(|i| i as u32).map_err(|e| {
                    debug!("error write stdin {}", e);
                    CgiErrorKind::RuntimeError
                });
            } else {
                return Ok(0);
            }
        }
        Err(CgiErrorKind::InvalidHandle)
    }

    /// kill the children process .
    #[allow(unused)]
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
        Err(CgiErrorKind::InvalidHandle)
    }

    /// the extern to exec the command with the arguments and envoriment variables .
    pub fn exec(&mut self) -> Result<(), CgiErrorKind> {
        let exec_file = format!("{}/{}", self.root_path, &self.command);
        let mut command = Command::new(&exec_file);
        command.stderr(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stdin(Stdio::piped());
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

pub async fn cgi_directory_list_exec(path: &str) -> Result<String, CgiErrorKind> {
    let mut read_dir = tokio::fs::read_dir(path).await.map_err(|e| {
        debug!("error read dir {}", e);
        CgiErrorKind::RuntimeError
    })?;
    let mut entries: Vec<JsonValue> = Vec::new();
    loop {
        let entry = match read_dir.next_entry().await {
            Ok(Some(e)) => e,
            Ok(None) => break,
            Err(e) => {
                debug!("error read dir next entry {}", e);
                return Err(CgiErrorKind::RuntimeError);
            }
        };
        
        let fname: String = entry.file_name().to_str().unwrap_or("").into();
        
        let is_file = entry.metadata().await.map_or(false, |m| m.is_file());
        if is_file {
            let mut json_obj = JsonObject::new();
            json_obj.insert("fileName", JsonValue::String(fname));
            entries.push(JsonValue::Object(json_obj));
        }
    }
    let vals = JsonValue::Array(entries);
    Ok(json::stringify(vals))
}
