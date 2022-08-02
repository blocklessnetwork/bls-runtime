#![allow(non_upper_case_globals)]
use log::error;
use wasi_common::WasiCtx;
use wiggle::GuestPtr;
use crate::{memory_driver, S3ErrorKind};

wiggle::from_witx!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_memory.witx"],
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
  impl blockless_memory::BlocklessMemory for WasiCtx {
    async fn memory_read<'a>(
      &mut self,
      handle: types::S3Handle,
      buf: &GuestPtr<'a, u8>,
      buf_len: u32,
  ) -> Result<u32, S3ErrorKind> {
      let stdin = self.blockless_config.as_ref().unwrap().stdin_ref();
      let mut dest_buf = vec![0; buf_len as _];
      let rs = memory_driver::read(handle.into(), &mut dest_buf, stdin.to_string()).await?;
      if rs > 0 {
        buf.as_array(rs).copy_from_slice(&dest_buf[0..rs as _]).map_err(|_| S3ErrorKind::RuntimeError)?;
      }
      Ok(rs)
  }
}
