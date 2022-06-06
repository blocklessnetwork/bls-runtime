#![allow(non_upper_case_globals)]
pub mod guest_ptr;
pub mod http;
use crate::ErrorKind;
use crate::{Driver, DriverConetxt};
pub use guest_ptr::ArrayTuple;
use log::debug;
use std::sync::Arc;
use wasi_common::file::{FileCaps, FileEntry};
use wasi_common::WasiCtx;
use wiggle::GuestPtr;

wiggle::from_witx!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_drivers.witx"],
    errors: { errno => ErrorKind },
    async: *,
    wasmtime: false,
});

impl From<ErrorKind> for types::Errno {
    fn from(e: ErrorKind) -> types::Errno {
        use types::Errno;
        match e {
            ErrorKind::ConnectError => Errno::BadConnect,
            ErrorKind::DriverNotFound => Errno::BadDriver,
            ErrorKind::Addrnotavail => Errno::Addrnotavail,
            ErrorKind::MemoryNotExport => Errno::Acces,
            ErrorKind::DriverBadOpen => Errno::BadOpen,
            ErrorKind::DriverBadParams => Errno::BadParams,
            ErrorKind::BadFileDescriptor => Errno::Badf,
            ErrorKind::EofError => Errno::Eof,
            ErrorKind::Unknown => Errno::Unknown,
        }
    }
}

macro_rules! enum_2_u32 {
    ($($t:tt),+) => {
       $(const $t: u32 = types::Errno::$t as _;)*
    }
}

enum_2_u32!(
    BadConnect,
    BadDriver,
    Addrnotavail,
    Acces,
    BadParams,
    BadOpen,
    Badf,
    Eof,
    Unknown
);

impl From<u32> for ErrorKind {
    fn from(i: u32) -> ErrorKind {
        match i {
            Eof => ErrorKind::EofError,
            BadConnect => ErrorKind::ConnectError,
            Addrnotavail => ErrorKind::Addrnotavail,
            BadOpen => ErrorKind::DriverBadOpen,
            Acces => ErrorKind::MemoryNotExport,
            BadDriver => ErrorKind::DriverNotFound,
            BadParams => ErrorKind::DriverBadParams,
            Unknown => ErrorKind::Unknown,
            Badf => ErrorKind::BadFileDescriptor,
            _ => ErrorKind::Unknown,
        }
    }
}

impl types::UserErrorConversion for WasiCtx {
    fn errno_from_error_kind(&mut self, e: ErrorKind) -> Result<types::Errno, wiggle::Trap> {
        debug!("Error: {:?}", e);
        e.try_into()
            .map_err(|e| wiggle::Trap::String(format!("{:?}", e)))
    }
}

impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

#[wiggle::async_trait]
impl blockless_drivers::BlocklessDrivers for WasiCtx {
    async fn blockless_open<'a>(
        &mut self,
        path: &GuestPtr<'a, str>,
        opts: &GuestPtr<'a, str>,
    ) -> Result<types::Fd, ErrorKind> {
        let path: &str = &path.as_str().unwrap();
        let opts: &str = &opts.as_str().unwrap();
        let drv: Arc<dyn Driver + Sync + Send> = match DriverConetxt::find_driver(path) {
            Some(d) => d,
            None => return Err(ErrorKind::DriverNotFound),
        };
        let caps = FileCaps::FDSTAT_SET_FLAGS
            | FileCaps::FILESTAT_GET
            | FileCaps::READ
            | FileCaps::WRITE
            | FileCaps::POLL_READWRITE;
        match drv
            .open(path, opts)
            .await
            .map(|f| Box::new(FileEntry::new(caps, f)))
        {
            Ok(f) => {
                let fd_num = self.table().push(f).unwrap();
                let fd = types::Fd::from(fd_num);
                Ok(fd)
            }
            Err(e) => Err(e),
        }
    }
}
