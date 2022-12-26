mod process;

use std::{collections::HashMap, sync::Once};

use process::CgiProcess;

use crate::CgiErrorKind;

enum CGICtx {
    Process(CgiProcess),
    DirectoryList((String, usize)),
}

fn get_ctx() -> Option<&'static mut HashMap<u32, CGICtx>> {
    static mut CTX: Option<HashMap<u32, CGICtx>> = None;
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

pub async fn cgi_directory_list_exec(path: &str) -> Result<u32, CgiErrorKind> {
    let rs = process::cgi_directory_list_exec(path).await?;
    let handle = increase_handle();

    get_ctx().map(|ctx| {
        ctx.insert(handle, CGICtx::DirectoryList((rs, 0)));
    });
    Ok(handle)
}

pub async fn cgi_directory_list_read(handle: u32, buf: &mut [u8]) -> Result<u32, CgiErrorKind> {
    let ctx = get_ctx().unwrap();
    let (vals, pos) = match ctx.remove(&handle) {
        Some(CGICtx::DirectoryList((s, p))) => (s, p),
        _ => return Err(CgiErrorKind::InvalidHandle),
    };
    let rs = vals.as_bytes();
    let remaining = rs.len() - pos;
    let copyn = remaining.min(buf.len());
    if remaining == 0 {
        ctx.insert(handle, CGICtx::DirectoryList((vals, pos)));
        return Ok(0);
    }
    
    buf[0..copyn].copy_from_slice(&rs[pos..(pos+copyn)]);
    ctx.insert(handle, CGICtx::DirectoryList((vals, pos+copyn)));
    Ok(copyn as u32)
}

pub async fn command_and_exec(root_path: &str, cmd: &str) -> Result<u32, CgiErrorKind> {
    let handle = increase_handle();
    let mut cgi = CgiProcess::new(root_path.into(), cmd)?;
    cgi.exec()?;
    get_ctx().map(|ctx| {
        ctx.insert(handle, CGICtx::Process(cgi));
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
        Some(CGICtx::Process(cgi_process)) => cgi_process,
        _ => return Err(CgiErrorKind::InvalidHandle),
    };
    cgi_process.child_stdin_write(buf).await
}

pub async fn child_stdout_read(handle: u32, buf: &mut [u8]) -> Result<u32, CgiErrorKind> {
    let ctx = get_ctx().unwrap();
    let cgi_process = match ctx.get_mut(&handle) {
        Some(CGICtx::Process(cgi_process)) => cgi_process,
        _ => return Err(CgiErrorKind::InvalidHandle),
    };
    cgi_process.child_stdout_read(buf).await
}

pub async fn child_stderr_read(handle: u32, buf: &mut [u8]) -> Result<u32, CgiErrorKind> {
    let ctx = get_ctx().unwrap();
    let cgi_process = match ctx.get_mut(&handle) {
        Some(CGICtx::Process(cgi_process)) => cgi_process,
        _ => return Err(CgiErrorKind::InvalidHandle),
    };
    cgi_process.child_stderr_read(buf).await
}