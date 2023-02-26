#![allow(non_upper_case_globals)]
use log::error;
use wasi_common::WasiCtx;
use wiggle::GuestPtr;

use crate::cgi_driver::{
    self, child_stderr_read, child_stdin_write, child_stdout_read, command_and_exec,cgi_directory_list_exec, cgi_directory_list_read
};
use crate::CgiErrorKind;

wiggle::from_witx!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_cgi.witx"],
    errors: { cgi_error => CgiErrorKind },
    async: *,
    wasmtime: false,
});

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

impl types::UserErrorConversion for WasiCtx {
    fn cgi_error_from_cgi_error_kind(
        &mut self,
        e: CgiErrorKind,
    ) -> Result<types::CgiError, wiggle::Trap> {
        e.try_into()
            .map_err(|e| wiggle::Trap::String(format!("{:?}", e)))
    }
}

impl wiggle::GuestErrorType for types::CgiError {
    fn success() -> Self {
        Self::Success
    }
}

#[wiggle::async_trait]
impl blockless_cgi::BlocklessCgi for WasiCtx {
    async fn cgi_open<'a>(
        &mut self,
        command_with_args: &GuestPtr<'a, str>,
    ) -> Result<types::CgiHandle, CgiErrorKind> {
        let cmd: &str = &command_with_args.as_str().map_err(|e| {
            error!("command error: {}", e);
            CgiErrorKind::InvalidParameter
        })?;
        let root_path = self.blockless_config
            .as_ref()
            .and_then(|c| c.drivers_root_path_ref())
            .unwrap_or("cgi_drivers_root");
        command_and_exec(root_path, cmd).await.map(|r| r.into())
    }

    async fn cgi_list_exec(
        &mut self
    ) -> Result<types::CgiHandle, CgiErrorKind> {
        let root_path = self.blockless_config
            .as_ref()
            .and_then(|c| c.drivers_root_path_ref())
            .unwrap_or("cgi_drivers_root");
        cgi_directory_list_exec(root_path).await.map(|r| r.into())
    }

    async fn cgi_list_read<'a>(
        &mut self,
        handle: types::CgiHandle,
        buf: &GuestPtr<'a, u8>,
        buf_len: u32,
    ) -> Result<u32, CgiErrorKind> {
        let mut dest_buf = vec![0; buf_len as _];
        let buf = buf.clone();
        let rs = cgi_directory_list_read(handle.into(), &mut dest_buf[..]).await?;
        if rs > 0 {
            buf.as_array(rs)
                .copy_from_slice(&dest_buf[0..rs as _])
                .map_err(|_| CgiErrorKind::RuntimeError)?;
        }
        Ok(rs)
    }

    async fn cgi_stdout_read<'a>(
        &mut self,
        handle: types::CgiHandle,
        buf: &GuestPtr<'a, u8>,
        buf_len: u32,
    ) -> Result<u32, CgiErrorKind> {
        let mut dest_buf = vec![0; buf_len as _];
        let buf = buf.clone();
        let rs = child_stdout_read(handle.into(), &mut dest_buf[..]).await?;
        if rs > 0 {
            buf.as_array(rs)
                .copy_from_slice(&dest_buf[0..rs as _])
                .map_err(|_| CgiErrorKind::RuntimeError)?;
        }
        Ok(rs)
    }

    async fn cgi_stderr_read<'a>(
        &mut self,
        handle: types::CgiHandle,
        buf: &GuestPtr<'a, u8>,
        buf_len: u32,
    ) -> Result<u32, CgiErrorKind> {
        let mut dest_buf = vec![0; buf_len as _];
        let buf = buf.clone();
        let rs = child_stderr_read(handle.into(), &mut dest_buf[..]).await?;
        if rs > 0 {
            buf.as_array(rs)
                .copy_from_slice(&dest_buf[0..rs as _])
                .map_err(|_| CgiErrorKind::RuntimeError)?;
        }
        Ok(rs)
    }

    async fn cgi_stdin_write<'a>(
        &mut self,
        handle: types::CgiHandle,
        buf: &GuestPtr<'a, u8>,
        buf_len: u32,
    ) -> Result<u32, CgiErrorKind> {
        let buf = buf.as_array(buf_len).as_slice().map_err(|e| {
            error!("guest stdin write buf error: {}", e);
            CgiErrorKind::InvalidParameter
        })?;
        let buf = unsafe { std::slice::from_raw_parts(buf.as_ptr(), buf_len as _) };
        child_stdin_write(handle.into(), buf).await
    }

    async fn cgi_close(
        &mut self,
        handle: types::CgiHandle,
    )  -> Result<(), CgiErrorKind>  {
        cgi_driver::close(handle.into())
    }
}
