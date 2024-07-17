#![allow(non_upper_case_globals)]
use log::error;
use wasi_common::WasiCtx;
use wiggle::{GuestMemory, GuestPtr};

use crate::cgi_driver::{
    self, cgi_directory_list_exec, cgi_directory_list_read, child_stderr_read, child_stdin_write,
    child_stdout_read, command_and_exec,
};
use crate::CgiErrorKind;

wiggle::from_witx!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_cgi.witx"],
    errors: { cgi_error => CgiErrorKind },
    async: *,
    wasmtime: false,
});

impl types::UserErrorConversion for WasiCtx {
    fn cgi_error_from_cgi_error_kind(
        &mut self,
        e: self::CgiErrorKind,
    ) -> wiggle::anyhow::Result<types::CgiError> {
        e.try_into()
            .map_err(|e| wiggle::anyhow::anyhow!(format!("{:?}", e)))
    }
}

impl From<CgiErrorKind> for types::CgiError {
    fn from(c: CgiErrorKind) -> Self {
        use types::CgiError;
        match c {
            CgiErrorKind::InvalidHandle => CgiError::InvalidHandle,
            CgiErrorKind::InvalidParameter => CgiError::InvalidParameter,
            CgiErrorKind::RuntimeError => CgiError::RuntimeError,
            CgiErrorKind::InvalidExtension => CgiError::InvalidExtension,
        }
    }
}

impl wiggle::GuestErrorType for types::CgiError {
    fn success() -> Self {
        Self::Success
    }
}

#[wiggle::async_trait]
impl blockless_cgi::BlocklessCgi for WasiCtx {
    async fn cgi_open(
        &mut self,
        memory: &mut GuestMemory<'_>,
        command_with_args: GuestPtr<str>,
    ) -> Result<types::CgiHandle, CgiErrorKind> {
        let cmd: &str = memory.as_str(command_with_args)
            .map_err(|e| {
                error!("command error: {}", e);
                CgiErrorKind::InvalidParameter
            })?
            .unwrap();
        let root_path = self.config_drivers_root_path_ref().unwrap();
        command_and_exec(&root_path, cmd).await.map(|r| r.into())
    }

    async fn cgi_list_exec(&mut self, _memory: &mut GuestMemory<'_>) -> Result<types::CgiHandle, CgiErrorKind> {
        let root_path = self.config_drivers_root_path_ref().unwrap();
        cgi_directory_list_exec(&root_path).await.map(|r| r.into())
    }

    async fn cgi_list_read(
        &mut self,
        memory: &mut GuestMemory<'_>,
        handle: types::CgiHandle,
        buf: GuestPtr<u8>,
        buf_len: u32,
    ) -> Result<u32, CgiErrorKind> {
        let mut dest_buf = vec![0; buf_len as _];
        let buf = buf.clone();
        let rs = cgi_directory_list_read(handle.into(), &mut dest_buf[..]).await?;
        if rs > 0 {
            memory.copy_from_slice(&dest_buf[0..rs as _], buf.as_array(rs))
                .map_err(|_| CgiErrorKind::RuntimeError)?;
        }
        Ok(rs)
    }

    async fn cgi_stdout_read(
        &mut self,
        memory: &mut GuestMemory<'_>,
        handle: types::CgiHandle,
        buf: GuestPtr<u8>,
        buf_len: u32,
    ) -> Result<u32, CgiErrorKind> {
        let mut dest_buf = vec![0; buf_len as _];
        let buf = buf.clone();
        let rs = child_stdout_read(handle.into(), &mut dest_buf[..]).await?;
        if rs > 0 {
            memory.copy_from_slice(&dest_buf[0..rs as _], buf.as_array(rs))
                .map_err(|_| CgiErrorKind::RuntimeError)?;
        }
        Ok(rs)
    }

    async fn cgi_stderr_read(
        &mut self,
        memory: &mut GuestMemory<'_>,
        handle: types::CgiHandle,
        buf: GuestPtr<u8>,
        buf_len: u32,
    ) -> Result<u32, CgiErrorKind> {
        let mut dest_buf = vec![0; buf_len as _];
        let buf = buf.clone();
        let rs = child_stderr_read(handle.into(), &mut dest_buf[..]).await?;
        if rs > 0 {
            memory.copy_from_slice(&dest_buf[0..rs as _], buf.as_array(rs))
                .map_err(|_| CgiErrorKind::RuntimeError)?;
        }
        Ok(rs)
    }

    async fn cgi_stdin_write(
        &mut self,
        memory: &mut GuestMemory<'_>,
        handle: types::CgiHandle,
        buf: GuestPtr<u8>,
        buf_len: u32,
    ) -> Result<u32, CgiErrorKind> {
        let buf = memory.as_slice(buf.as_array(buf_len))
            .map_err(|e| {
                error!("guest stdin write buf error: {}", e);
                CgiErrorKind::InvalidParameter
            })?
            .unwrap();
        let buf = unsafe { std::slice::from_raw_parts(buf.as_ptr(), buf_len as _) };
        child_stdin_write(handle.into(), buf).await
    }

    async fn cgi_close(&mut self, _memory: &mut GuestMemory<'_>, handle: types::CgiHandle) -> Result<(), CgiErrorKind> {
        cgi_driver::close(handle.into())
    }
}
