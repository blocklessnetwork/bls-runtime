
use crate::HttpErrorKind;
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
        Err(HttpErrorKind::BufferTooSmall)
    }

    async fn http_close(&mut self, handle: types::HttpHandle) -> Result<(), HttpErrorKind> {
        Ok(())
    }
}
