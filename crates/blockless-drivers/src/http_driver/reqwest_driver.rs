use std::{collections::HashMap, sync::Once, time::Duration};

use log::error;
use reqwest::Response;

use crate::HttpErrorKind;

pub(crate) enum HttpCtx {
    Response(Response),
}

/// get the http context
pub(crate) fn get_ctx() -> Option<&'static mut HashMap<u32, HttpCtx>> {
    static mut CTX: Option<HashMap<u32, HttpCtx>> = None;
    static CTX_ONCE: Once = Once::new();
    CTX_ONCE.call_once(||{
        unsafe {
            CTX = Some(HashMap::new());
        }
    });
    unsafe {
        CTX.as_mut()
    }
}

fn increase_fd() -> Option<u32> {
    static mut MAX_HANDLE: u32 = 0;
    unsafe {
        MAX_HANDLE += 1;
        Some(MAX_HANDLE)
    }
}

pub async fn http_req(url: &str, opts: &str) -> Result<(u32, i32), HttpErrorKind> {
    let json = match json::parse(opts) {
        Ok(o) => o,
        Err(_) => return Err(HttpErrorKind::RequestError),
    };
    let method = match json["method"].as_str() {
        Some(s) => String::from(s),
        None => return Err(HttpErrorKind::RequestError),
    };
    let connect_timeout = json["connectTimeout"]
        .as_u64()
        .map(|s| Duration::from_secs(s));
    let read_timeout = json["readTimeout"]
        .as_u64()
        .map(|s| Duration::from_secs(s));
    
    let mut client_builder = reqwest::ClientBuilder::new();
    if connect_timeout.is_some() {
        client_builder = client_builder.connect_timeout(connect_timeout.unwrap());
    }
    if read_timeout.is_some() {
        client_builder = client_builder.timeout(read_timeout.unwrap());
    }
    let mut client = client_builder.build().unwrap();
    let req_method = method.to_lowercase();
    let req_builder = match req_method.as_str() {
        "get" => client.get(url),
        "post" => client.post(url),
        _ => return Err(HttpErrorKind::RequestError),
    };
    let resp = req_builder
        .send()
        .await
        .map_err(|e| {
            error!("request send error, {}", e);
            HttpErrorKind::RuntimeError
        })?;
    let status = resp.status().as_u16() as i32;
    let fd = increase_fd().unwrap();
    let ctx = get_ctx().unwrap();
    ctx.insert(fd, HttpCtx::Response(resp));
    Ok((fd, status))
}