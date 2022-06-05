#![allow(non_upper_case_globals)]
use crate::{HttpErrorKind, http::get_http_driver};
use log::error;
use wasi_common::WasiCtx;
use wiggle::GuestPtr;

wiggle::from_witx!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_http.witx"],
    errors: { http_error => HttpErrorKind },
    async: *,
    wasmtime: false,
});

impl From<HttpErrorKind> for types::HttpError {
    fn from(e: HttpErrorKind) -> types::HttpError {
        use types::HttpError;
        match e {
            HttpErrorKind::InvalidHandle => HttpError::InvalidHandle,
            HttpErrorKind::MemoryAccessError => HttpError::MemoryAccessError,
            HttpErrorKind::BufferTooSmall => HttpError::BufferTooSmall,
            HttpErrorKind::HeaderNotFound => HttpError::HeaderNotFound,
            HttpErrorKind::Utf8Error => HttpError::Utf8Error,
            HttpErrorKind::DestinationNotAllowed => HttpError::DestinationNotAllowed,
            HttpErrorKind::InvalidMethod => HttpError::InvalidMethod,
            HttpErrorKind::InvalidEncoding => HttpError::InvalidEncoding,
            HttpErrorKind::InvalidUrl => HttpError::InvalidUrl,
            HttpErrorKind::RequestError => HttpError::RequestError,
            HttpErrorKind::RuntimeError => HttpError::RuntimeError,
            HttpErrorKind::TooManySessions => HttpError::TooManySessions,
            HttpErrorKind::InvalidDriver => HttpError::RuntimeError,
        }
    }
}

macro_rules! enum_2_u32 {
    ($($t:tt),+) => {
       $(const $t: u32 = types::HttpError::$t as _;)*
    }
}

enum_2_u32!(
    InvalidHandle,
    MemoryAccessError,
    BufferTooSmall,
    HeaderNotFound,
    Utf8Error,
    DestinationNotAllowed,
    InvalidMethod,
    InvalidEncoding,
    InvalidUrl,
    RequestError,
    RuntimeError,
    TooManySessions
);

impl From<u32> for HttpErrorKind {
    fn from(i: u32) -> HttpErrorKind {
        match i {
            InvalidHandle => HttpErrorKind::InvalidHandle,
            MemoryAccessError => HttpErrorKind::MemoryAccessError,
            BufferTooSmall => HttpErrorKind::BufferTooSmall,
            HeaderNotFound => HttpErrorKind::HeaderNotFound,
            Utf8Error => HttpErrorKind::Utf8Error,
            DestinationNotAllowed => HttpErrorKind::DestinationNotAllowed,
            InvalidMethod => HttpErrorKind::InvalidMethod,
            InvalidEncoding => HttpErrorKind::InvalidEncoding,
            InvalidUrl => HttpErrorKind::InvalidUrl,
            RuntimeError => HttpErrorKind::RuntimeError,
            RequestError => HttpErrorKind::RequestError,
            TooManySessions => HttpErrorKind::TooManySessions,
            _ => HttpErrorKind::RuntimeError,
        }
    }
}

impl types::UserErrorConversion for WasiCtx {
    fn http_error_from_http_error_kind(
        &mut self,
        e: HttpErrorKind,
    ) -> Result<types::HttpError, wiggle::Trap> {
        e.try_into()
            .map_err(|e| wiggle::Trap::String(format!("{:?}", e)))
    }
}

impl wiggle::GuestErrorType for types::HttpError {
    fn success() -> Self {
        Self::Success
    }
}

#[wiggle::async_trait]
impl blockless_http::BlocklessHttp for WasiCtx {
    async fn http_req<'a>(&mut self,
        url: &GuestPtr<'a, str>,
        opts: &GuestPtr<'a, str>
    ) -> Result<types::HttpHandle, HttpErrorKind> {
        let driver = get_http_driver().ok_or(HttpErrorKind::InvalidDriver)?;
        let url: &str = &url.as_str().map_err(|e| {
            error!("guest url error: {}", e);
            HttpErrorKind::InvalidEncoding
        })?;
        let opts: &str = &opts.as_str().map_err(|e| {
            error!("guest options error: {}", e);
            HttpErrorKind::InvalidEncoding
        })?;
        let fd = driver.http_req(url, opts)?;
        Ok(types::HttpHandle::from(fd))
    }

    async fn http_close(&mut self, handle: types::HttpHandle) -> Result<(), HttpErrorKind> {
        let driver = get_http_driver().ok_or(HttpErrorKind::InvalidDriver)?;
        driver.http_close(handle.into())?;
        Ok(())
    }
}
