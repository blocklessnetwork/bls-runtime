#![allow(non_upper_case_globals)]
use crate::s3_driver;
use crate::S3ErrorKind;
use log::error;
use wasi_common::WasiCtx;
use wiggle::GuestPtr;

wiggle::from_witx!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_s3.witx"],
    errors: { s3_error => S3ErrorKind },
    async: *,
    wasmtime: false,
});


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

impl types::UserErrorConversion for WasiCtx {
    fn s3_error_from_s3_error_kind(
        &mut self,
        e: S3ErrorKind,
    ) -> Result<types::S3Error, wiggle::Trap> {
        e.try_into()
            .map_err(|e| wiggle::Trap::String(format!("{:?}", e)))
    }
}

impl wiggle::GuestErrorType for types::S3Error {
    fn success() -> Self {
        Self::Success
    }
}


#[wiggle::async_trait]
impl blockless_s3::BlocklessS3 for WasiCtx {
    async fn bucket_create<'a>(
        &mut self,
        param: &GuestPtr<'a, str>,
        buf: &GuestPtr<'a, u8>,
        buf_len: u32,
    ) -> Result<(), S3ErrorKind> {
        
        Ok(())
    }
}