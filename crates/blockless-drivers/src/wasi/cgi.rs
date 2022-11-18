#![allow(non_upper_case_globals)]
use wasi_common::WasiCtx;
use wiggle::GuestPtr;

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
        param: &GuestPtr<'a, str>,
    ) -> Result<types::CgiHandle, CgiErrorKind> {
        Ok(0.into())
    }

    async fn cgi_read<'a>(
        &mut self,
        buf: &GuestPtr<'a, u8>,
        buf_len: u32,
    ) -> Result<u32, CgiErrorKind> {
        Ok(0)
    }

    async fn cgi_write<'a>(
        &mut self,
        buf: &GuestPtr<'a, u8>,
        buf_len: u32,
    ) -> Result<u32, CgiErrorKind> {
        Ok(0)
    }
}
