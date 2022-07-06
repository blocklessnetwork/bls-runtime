mod api;
mod file;
mod http_raw;
mod util;
use std::{collections::HashMap, sync::Once};
pub use util::gen_boundary;
use http_raw::HttpRaw;
use api::*;

#[cfg(feature = "runtime")]
use tokio::runtime::{Builder, Runtime};

use crate::IpfsErrorKind;

const HOST: &str = "127.0.0.1";
const PORT: u16 = 5001;

#[cfg(feature = "runtime")]
pub fn get_runtime() -> Option<&'static Runtime> {
    static mut RUNTIME: Option<Runtime> = None;
    static RUNTIME_ONCE: Once = Once::new();
    RUNTIME_ONCE.call_once(|| {
        let rt = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        unsafe {
            RUNTIME = Some(rt);
        };
    });
    unsafe { RUNTIME.as_ref() }
}

pub enum ApiCtx {
    Response(Response),
    HttpRaw(HttpRaw),
}


pub fn get_ctx() -> Option<&'static mut HashMap<u32, ApiCtx>> {
    static mut CTX: Option<HashMap<u32, ApiCtx>> = None;
    static CTX_ONCE: Once = Once::new();
    CTX_ONCE.call_once(|| {
        unsafe {
            CTX = Some(HashMap::new());
        };
    });
    unsafe { CTX.as_mut() }
}

pub fn increase_fd() -> Option<u32> {
    static mut MAX_HANDLE: u32 = 0;
    unsafe { 
        MAX_HANDLE += 1;
        Some(MAX_HANDLE)
    }
}

pub async fn command(cmd: &str) -> Result<(u16, u32), IpfsErrorKind> {
    let rs = inner_command(cmd).await?;
    let fd = increase_fd().unwrap();
    match rs {
        ApiCtx::Response(rs) => {
            let status = rs.status;
            get_ctx().unwrap().insert(fd, ApiCtx::Response(rs));
            Ok((status, fd))
        }
        ApiCtx::HttpRaw(raw) => {
            get_ctx().unwrap().insert(fd, ApiCtx::HttpRaw(raw));
            Ok((0, fd))
        }
    }
}

pub async fn read_body(handle: u32, buf: &mut [u8]) -> Result<u32, IpfsErrorKind> {
    let ctx = get_ctx().unwrap();
    if buf.len() == 0 {
        return Err(IpfsErrorKind::InvalidEncoding);
    }
    match ctx.get_mut(&handle) {
        Some(ApiCtx::Response(resp)) => {
            Ok(resp.copy_body_remain(buf) as _)
        }
        Some(ApiCtx::HttpRaw(raw)) if raw.is_connect() => {
            Ok(0)
        }
        _ => return Err(IpfsErrorKind::RuntimeError),
    }
}

async fn inner_command(cmd: &str) -> Result<ApiCtx, IpfsErrorKind> {
    let json = match json::parse(cmd) {
        Ok(o) => o,
        Err(_) => return Err(IpfsErrorKind::InvalidParameter),
    };
    let api = match json["api"].as_str() {
        Some(s) => String::from(s),
        None => return Err(IpfsErrorKind::InvalidParameter),
    };
    let args = match json["args"] {
        json::JsonValue::Array(ref arr) => {
            let mut kv = Vec::with_capacity(arr.len());
            for v in arr.iter() {
                let name = match v["name"] {
                    json::JsonValue::String(ref s) => String::from(s),
                    json::JsonValue::Short(b) => b.into(),
                    _ => return Err(IpfsErrorKind::InvalidParameter),
                };
                let value = match v["value"] {
                    json::JsonValue::String(ref s) => String::from(s),
                    json::JsonValue::Boolean(b) => format!("{}", b),
                    json::JsonValue::Number(b) => format!("{}", b),
                    json::JsonValue::Short(b) => b.into(),
                    _ => return Err(IpfsErrorKind::InvalidParameter),
                };
                kv.push((name, value));
            }
            match serde_urlencoded::to_string(&kv) {
                Ok(o) => Some(o),
                Err(_) => return Err(IpfsErrorKind::InvalidParameter),
            }
        },
        _ => None,
    };
    match api.as_str() {
        "files/ls" => Api::new(HOST, PORT).file_api().ls(args).await.map(ApiCtx::Response),
        "files/mkdir" => Api::new(HOST, PORT).file_api().mkdir(args).await.map(ApiCtx::Response),
        "files/rm" => Api::new(HOST, PORT).file_api().rm(args).await.map(ApiCtx::Response),
        "files/write" => Api::new(HOST, PORT).file_api().write(args).await.map(ApiCtx::HttpRaw),
        _ => return Err(IpfsErrorKind::InvalidMethod),
    }
}
