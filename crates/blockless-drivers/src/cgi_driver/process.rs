use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Once,
};

use crate::CgiErrorKind;
use json::object::Object as JsonObject;
use json::JsonValue;
use log::{debug, error};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
};

use super::db::{ExtensionMeta, DB, ExtensionMetaStatus};

fn get_db(path: impl AsRef<Path>) -> Option<&'static mut DB> {
    static mut DB: Option<DB> = None;
    static DB_ONCE: Once = Once::new();
    DB_ONCE.call_once(|| unsafe {
        let db = DB::new(path)
            .map_err(|e| error!("error open db {}", e))
            .ok();
        DB = db
            .map(|mut db| {
                if db.create_schema().is_ok() {
                    Some(db)
                } else {
                    None
                }
            })
            .flatten();
    });
    unsafe { DB.as_mut() }
}

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

/// get db file name from path.
fn get_db_file_name(path: &str) -> PathBuf {
    const DB_NAME: &str = ".extsdb";
    let path = Path::new(path);
    path.join(DB_NAME)
}

fn list_extensions_from_db(path: &str) -> (Vec<i32>, HashMap<String, ExtensionMeta>) {
    let db_file_name = get_db_file_name(path);
    let db = get_db(db_file_name);
    //load the metas from db
    //the ids in db will use for delete the invalid extension.
    db.map(|db| {
        let exts = db
            .list_extensions()
            .map_err(|e| error!("error list extensions: {}", e))
            .unwrap_or_default();
        let mut exts_metas = HashMap::new();
        let mut ids_in_db = Vec::new();
        for mut ext in exts.into_iter() {
            ids_in_db.push(ext.id);
            ext.status = ExtensionMetaStatus::Invalid;
            exts_metas.insert(ext.file_name.clone(), ext);
        }
        (ids_in_db, exts_metas)
    })
    .unwrap_or_default()
}

async fn file_md5(path: impl AsRef<Path>) -> anyhow::Result<md5::Digest> {
    let mut file = File::open(path).await?;
    let mut buf = [0u8; 4096];
    let mut md5_ctx = md5::Context::new();
    loop {
        let rn = file.read(&mut buf).await?;
        if rn == 0 {
            break;
        }
        md5_ctx.consume(&buf[..rn]);
    }
    Ok(md5_ctx.compute())
}

async fn get_file_meta(file_name: &str) -> anyhow::Result<Option<ExtensionMeta>> {
    let mut command = Command::new(&file_name);
    command.args(&["--ext_verify"]);
    let child = command.output().await?;
    let val = std::str::from_utf8(&child.stdout[..])?;
    //parse output json like {"alias":"xxx", "md5":"xxxx", "desciption":"xxxxx", "is_cgi": true}
    let json = json::parse(val.trim())?;
    let is_cgi = match json["is_cgi"].as_bool() {
        Some(b) => b,
        None => return Ok(None),
    };
    if is_cgi != true {
        return Ok(None);
    }
    let alias = match json["alias"].as_str() {
        Some(b) => b.into(),
        None => return Ok(None),
    };
    let md5: String = match json["md5"].as_str() {
        Some(md5) => md5.into(),
        None => {
            //TODO: next step will embed the md5 value into file.
            let file_md5 = file_md5(file_name).await?;
            format!("{:x}", file_md5)
        },
    };
    let description: String = match json["description"].as_str() {
        Some(d) => d.into(),
        None => return Ok(None),
    };
    
    Ok(Some(ExtensionMeta {
        md5,
        alias,
        description,
        file_name: file_name.into(),
        ..Default::default()
    }))
}

async fn list_cgi_directory(
    path: impl AsRef<Path>, 
    meta_db: &mut HashMap<String, ExtensionMeta>,
) -> anyhow::Result<Vec<ExtensionMeta>> {
    let mut read_dir = tokio::fs::read_dir(path).await?;
    let mut rs = Vec::new();
    loop {
        let entry = read_dir.next_entry().await?;
        let entry = match entry {
            Some(e) => e,
            None => break,
        };
        let file_name = entry.file_name();
        let md5 = file_md5(&file_name).await?;
        let md5 = format!("{:02x}", md5);
        let file_name: String = entry.file_name().to_str().unwrap_or("").into();
        match meta_db.get_mut(&file_name) {
            Some(meta) => {
                if meta.md5 != md5 {
                    meta.md5 = md5;
                }
                meta.status = ExtensionMetaStatus::UPDATE;
            },
            None => {
                
            },
        }
    }
    Ok(rs)
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

#[cfg(test)]
mod test {
    use super::*;
    use tempdir;

    #[test]
    fn test_extensions_file() {
        let temp_dir = tempdir::TempDir::new("drivers-test");
        temp_dir.map(|dir| {});
    }
}
