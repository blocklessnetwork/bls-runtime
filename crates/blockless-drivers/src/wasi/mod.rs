use crate::ErrorKind;
use crate::{Driver, DriverConetxt};
use std::sync::Arc;
use log::debug;
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
            ErrorKind::MemoryNotExport => Errno::Addrnotavail,
            ErrorKind::DriverBadOpen => Errno::BadOpen,
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
        let drv: Arc<dyn Driver + Sync + Send> = match self.find_driver(path) {
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
