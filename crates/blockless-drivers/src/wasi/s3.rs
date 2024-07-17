#![allow(non_upper_case_globals)]
use crate::{s3_driver, S3ErrorKind};
use log::error;
use wasi_common::WasiCtx;
use wiggle::{GuestMemory, GuestPtr};

wiggle::from_witx!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_s3.witx"],
    errors: { s3_error => S3ErrorKind },
    async: *,
    wasmtime: false,
});

impl types::UserErrorConversion for WasiCtx {
    fn s3_error_from_s3_error_kind(
        &mut self,
        e: self::S3ErrorKind,
    ) -> wiggle::anyhow::Result<types::S3Error> {
        e.try_into()
            .map_err(|e| wiggle::anyhow::anyhow!(format!("{:?}", e)))
    }
}

impl From<S3ErrorKind> for types::S3Error {
    fn from(e: S3ErrorKind) -> types::S3Error {
        use types::S3Error;
        match e {
            S3ErrorKind::InvalidHandle => S3Error::InvalidHandle,
            S3ErrorKind::Utf8Error => S3Error::Utf8Error,
            S3ErrorKind::InvalidParameter => S3Error::InvalidParameter,
            S3ErrorKind::InvalidMethod => S3Error::InvalidMethod,
            S3ErrorKind::InvalidEncoding => S3Error::InvalidEncoding,
            S3ErrorKind::CredentialsError => S3Error::CredentialsError,
            S3ErrorKind::RegionError => S3Error::RegionError,
            S3ErrorKind::RequestError => S3Error::RequestError,
            S3ErrorKind::RuntimeError => S3Error::RuntimeError,
            S3ErrorKind::TooManySessions => S3Error::TooManySessions,
            S3ErrorKind::PermissionDeny => S3Error::PermissionDeny,
        }
    }
}

impl wiggle::GuestErrorType for types::S3Error {
    fn success() -> Self {
        Self::Success
    }
}

#[wiggle::async_trait]
impl blockless_s3::BlocklessS3 for WasiCtx {
    async fn bucket_command(
        &mut self,
        memory: &mut GuestMemory<'_>,
        cmd: u16,
        param: GuestPtr<str>,
    ) -> Result<types::S3Handle, S3ErrorKind> {
        let params = memory.as_str(param)
            .map_err(|e| {
                error!("guest url error: {}", e);
                S3ErrorKind::Utf8Error
            })?
            .unwrap();
        let rs = s3_driver::bucket_command(cmd, &params).await?;
        Ok(rs.into())
    }

    async fn bucket_put_object(
        &mut self,
        memory: &mut GuestMemory<'_>,
        cfg: GuestPtr<str>,
        buf: GuestPtr<u8>,
        buf_len: u32,
    ) -> Result<(), S3ErrorKind> {
        let cfg = memory.as_str(cfg)
            .map_err(|e| {
                error!("guest url error: {}", e);
                S3ErrorKind::Utf8Error
            })?
            .unwrap();
        
        let params = memory.as_slice(buf.as_array(buf_len))
            .map_err(|e| {
                error!("guest url error: {}", e);
                S3ErrorKind::InvalidParameter
            })?
            .unwrap();
        s3_driver::bucket_put_object(&cfg, &params).await
    }

    async fn s3_read(
        &mut self,
        memory: &mut GuestMemory<'_>,
        handle: types::S3Handle,
        buf: GuestPtr<u8>,
        buf_len: u32,
    ) -> Result<u32, S3ErrorKind> {
        let mut dest_buf = vec![0; buf_len as _];
        let rs = s3_driver::read(handle.into(), &mut dest_buf).await?;
        if rs > 0 {
            memory.copy_from_slice(&dest_buf[0..rs as _], buf.as_array(rs))
                .map_err(|_| S3ErrorKind::RuntimeError)?;
        }
        Ok(rs)
    }

    async fn s3_close(
        &mut self, 
        memory: &mut GuestMemory<'_>,
        handle: types::S3Handle
    ) -> Result<(), S3ErrorKind> {
        s3_driver::close(handle.into()).await
    }
}
