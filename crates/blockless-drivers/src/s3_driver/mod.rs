mod bucket;
use std::{collections::HashMap, sync::Once};

use crate::{read_ext::ReadRemain, S3ErrorKind};

pub struct VecResult {
    content: Vec<u8>,
    read_point: usize,
}

impl VecResult {
    fn new(content: Vec<u8>) -> Self {
        VecResult {
            content,
            read_point: 0,
        }
    }
}

impl ReadRemain for VecResult {
    fn as_bytes_ref(&self) -> Option<&[u8]> {
        Some(&self.content)
    }

    fn read_point(&self) -> usize {
        self.read_point
    }

    fn set_read_point(&mut self, point: usize) {
        self.read_point = point;
    }
}

pub enum S3Ctx {
    VecResult(VecResult),
    None,
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

pub async fn bucket_command(cmd: u16, params: &str) -> Result<u32, S3ErrorKind> {
    let content = match cmd {
        1 => {
            let json = bucket::create(params).await?;
            S3Ctx::VecResult(VecResult::new(json.as_bytes().to_vec()))
        }
        2 => {
            let json = bucket::list(params).await?;
            S3Ctx::VecResult(VecResult::new(json.as_bytes().to_vec()))
        }
        3 => {
            let rs = bucket::get_object(params).await?;
            S3Ctx::VecResult(VecResult::new(rs))
        }
        4 => {
            bucket::delete_object(params).await?;
            S3Ctx::None
        }
        _ => return Err(S3ErrorKind::InvalidParameter),
    };

    let fd = increase_fd().unwrap();
    get_ctx().unwrap().insert(fd, content);
    Ok(fd)
}

pub async fn bucket_put_object(cfg: &str, buf: &[u8]) -> Result<(), S3ErrorKind> {
    bucket::put_object(cfg, buf).await
}

pub async fn read(handle: u32, buf: &mut [u8]) -> Result<u32, S3ErrorKind> {
    let ctx = get_ctx().unwrap();
    if buf.len() == 0 {
        return Err(S3ErrorKind::InvalidParameter);
    }
    match ctx.get_mut(&handle) {
        Some(S3Ctx::VecResult(resp)) => Ok(resp.copy_remain(buf) as _),
        _ => return Err(S3ErrorKind::InvalidHandle),
    }
}
