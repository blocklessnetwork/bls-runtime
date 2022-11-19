mod process;

use std::{collections::HashMap, sync::Once};

use process::CgiProcess;

use crate::CgiErrorKind;

pub fn get_ctx() -> Option<&'static mut HashMap<u32, CgiProcess>> {
    static mut CTX: Option<HashMap<u32, CgiProcess>> = None;
    static CTX_ONCE: Once = Once::new();
    CTX_ONCE.call_once(|| {
        unsafe {
            CTX = Some(HashMap::new());
        };
    });
    unsafe { CTX.as_mut() }
}

fn increase_handle() -> u32 {
    static mut MAX_HANDLE: u32 = 0;
    unsafe {
        MAX_HANDLE += 1;
        MAX_HANDLE
    }
}

pub async fn command_and_exec(cmd: &str) -> Result<u32, CgiErrorKind> {
    let handle = increase_handle();
    let mut cgi = CgiProcess::new(cmd)?;
    cgi.exec()?;
    get_ctx().map(|ctx| {
        ctx.insert(handle, cgi);
    });
    Ok(handle)
}

pub async fn stdin_write(handle: u32, buf: &[u8]) -> Result<u32, CgiErrorKind> {
    get_ctx().map(|ctx| {
        ctx.get_mut(&handle)
    }).flatten();
    Ok(0)
}