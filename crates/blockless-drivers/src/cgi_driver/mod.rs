mod process;

use std::{collections::HashMap, sync::Once};

use process::CgiProcess;

use crate::CgiErrorKind;

fn get_ctx() -> Option<&'static mut HashMap<u32, CgiProcess>> {
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

pub fn close(handle: u32) -> Result<(), CgiErrorKind> {
    let ctx = get_ctx().unwrap();
    if ctx.remove(&handle).is_none() {
        return Err(CgiErrorKind::InvalidHandle);
    }
    Ok(())
}

pub async fn child_stdin_write(handle: u32, buf: &[u8]) -> Result<u32, CgiErrorKind> {
    let ctx = get_ctx().unwrap();
    let cgi_process = match ctx.get_mut(&handle) {
        Some(cgi_process) => cgi_process,
        None => return Err(CgiErrorKind::InvalidHandle),
    };
    cgi_process.child_stdin_write(buf).await
}

pub async fn child_stdout_read(handle: u32, buf: &mut [u8]) -> Result<u32, CgiErrorKind> {
    let ctx = get_ctx().unwrap();
    let cgi_process = match ctx.get_mut(&handle) {
        Some(cgi_process) => cgi_process,
        None => return Err(CgiErrorKind::InvalidHandle),
    };
    cgi_process.child_stdout_read(buf).await
}

pub async fn child_stderr_read(handle: u32, buf: &mut [u8]) -> Result<u32, CgiErrorKind> {
    let ctx = get_ctx().unwrap();
    let cgi_process = match ctx.get_mut(&handle) {
        Some(cgi_process) => cgi_process,
        None => return Err(CgiErrorKind::InvalidHandle),
    };
    cgi_process.child_stderr_read(buf).await
}