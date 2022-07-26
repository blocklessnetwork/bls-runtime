mod bucket;
use std::{collections::HashMap, sync::Once};

use crate::{read_ext::ReadRemain, S3ErrorKind};

pub struct JsonResult {
    content: String,
    read_point: usize,
}

impl JsonResult {
    fn new(content: String) -> Self {
        JsonResult {
            content,
            read_point: 0,
        }
    }
}

impl ReadRemain for JsonResult {
    fn as_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.content.as_bytes())
    }

    fn read_point(&self) -> usize {
        self.read_point
    }

    fn set_read_point(&mut self, point: usize) {
        self.read_point = point;
    }
}

pub enum S3Ctx {
    JsonStringfy(JsonResult),
}

pub fn get_ctx() -> Option<&'static mut HashMap<u32, S3Ctx>> {
    static mut CTX: Option<HashMap<u32, S3Ctx>> = None;
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

pub async fn close(handle: u32) -> Result<(), S3ErrorKind> {
    let ctx = get_ctx().unwrap();
    ctx.remove(&handle);
    Ok(())
}

pub async fn bucket_create(params: &str) -> Result<u32, S3ErrorKind> {
    let json = bucket::create(params).await?;
    let fd = increase_fd().unwrap();
    get_ctx()
        .unwrap()
        .insert(fd, S3Ctx::JsonStringfy(JsonResult::new(json)));
    Ok(fd)
}

pub async fn bucket_list(params: &str) -> Result<u32, S3ErrorKind> {
    let json = bucket::list(params).await?;
    let fd = increase_fd().unwrap();
    get_ctx()
        .unwrap()
        .insert(fd, S3Ctx::JsonStringfy(JsonResult::new(json)));
    Ok(fd)
}

pub async fn read(handle: u32, buf: &mut [u8]) -> Result<u32, S3ErrorKind> {
    let ctx = get_ctx().unwrap();
    if buf.len() == 0 {
        return Err(S3ErrorKind::InvalidParameter);
    }
    match ctx.get_mut(&handle) {
        Some(S3Ctx::JsonStringfy(resp)) => Ok(resp.copy_remain(buf) as _),
        _ => return Err(S3ErrorKind::InvalidHandle),
    }
}
