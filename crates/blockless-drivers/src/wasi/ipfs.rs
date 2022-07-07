#![allow(non_upper_case_globals)]
use crate::IpfsErrorKind;
use log::error;
use wasi_common::WasiCtx;
use wiggle::GuestPtr;
use crate::ipfs_driver;

wiggle::from_witx!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_ipfs.witx"],
    errors: { ipfs_error => IpfsErrorKind },
    async: *,
    wasmtime: false,
});

impl From<IpfsErrorKind> for types::IpfsError {
    fn from(e: IpfsErrorKind) -> types::IpfsError {
        use types::IpfsError;
        match e {
            IpfsErrorKind::InvalidHandle => IpfsError::InvalidHandle,
            IpfsErrorKind::Utf8Error => IpfsError::Utf8Error,
            IpfsErrorKind::InvalidParameter => IpfsError::InvalidParameter,
            IpfsErrorKind::InvalidMethod => IpfsError::InvalidMethod,
            IpfsErrorKind::InvalidEncoding => IpfsError::InvalidEncoding,
            IpfsErrorKind::RequestError => IpfsError::RequestError,
            IpfsErrorKind::RuntimeError => IpfsError::RuntimeError,
            IpfsErrorKind::TooManySessions => IpfsError::TooManySessions,
            IpfsErrorKind::PermissionDeny => IpfsError::PermissionDeny,
        }
    }
}


impl types::UserErrorConversion for WasiCtx {
    fn ipfs_error_from_ipfs_error_kind(
        &mut self,
        e: IpfsErrorKind,
    ) -> Result<types::IpfsError, wiggle::Trap> {
        e.try_into()
            .map_err(|e| wiggle::Trap::String(format!("{:?}", e)))
    }
}

impl wiggle::GuestErrorType for types::IpfsError {
    fn success() -> Self {
        Self::Success
    }
}

#[wiggle::async_trait]
impl blockless_ipfs::BlocklessIpfs for WasiCtx {
    async fn ipfs_command<'a>(
        &mut self,
        params: &GuestPtr<'a, str>,
    ) -> Result<(types::IpfsHandle, types::StatusCode), IpfsErrorKind> {
        let params: &str = &params.as_str().map_err(|e| {
            error!("guest url error: {}", e);
            IpfsErrorKind::Utf8Error
        })?;
        let (status, fd) = ipfs_driver::command(params).await?;
        Ok((types::IpfsHandle::from(fd), types::StatusCode::from(status)))
    }

    async fn ipfs_read_body<'a>(
        &mut self, 
        handle: types::IpfsHandle,
        buf: &GuestPtr<'a, u8>,
        buf_len: u32,
    ) -> Result<u32, IpfsErrorKind> {
        let mut dest_buf = vec![0; buf_len as _];
        let buf = buf.clone();
        let rs = ipfs_driver::read_body(handle.into(), &mut dest_buf[..]).await?;
        if rs > 0 {
            buf.as_array(rs)
                .copy_from_slice(&dest_buf[0..rs as _])
                .map_err(|_| IpfsErrorKind::RuntimeError)?;
        }
        Ok(rs)
    }

    async fn ipfs_close(
        &mut self, 
        handle: types::IpfsHandle
    ) -> Result<(), IpfsErrorKind> {
        ipfs_driver::close(handle.into()).await?;
        Ok(())
    }

    async fn ipfs_write<'a>(
        &mut self, 
        handle: types::IpfsHandle,
        buf: &GuestPtr<'a, u8>,
        buf_len: u32,
    ) -> Result<u32, IpfsErrorKind> {
        let params = buf.as_array(buf_len).as_slice().map_err(|e| {
            error!("guest url error: {}", e);
            IpfsErrorKind::InvalidParameter
        })?;
        let buf = unsafe {
            std::slice::from_raw_parts(params.as_ptr(), buf_len as _)
        };
        let rs = ipfs_driver::write_body(handle.into(), buf).await?;
        Ok(rs)
    }
}
