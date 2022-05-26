pub use anyhow::{Context, Error};

/// Internal error type for the `wasi-common` crate.
/// Contains variants of the WASI `$errno` type are added according to what is actually used internally by
/// the crate. Not all values are represented presently.
#[derive(Debug)]
pub enum ErrorKind {
    ConnectError,
    EofError,
    MemoryNotExport,
    BadFileDescriptor,
    DriverNotFound,
    Addrnotavail,
    DriverBadOpen,
    DriverBadParams,
    Unkown,
}

impl std::error::Error for ErrorKind {}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Self::ConnectError => write!(f, "Connect Error"),
            &Self::MemoryNotExport => write!(f, "Memoery not export"),
            &Self::DriverNotFound => write!(f, "Driver not found."),
            &Self::DriverBadOpen => write!(f, "Driver bad open"),
            &Self::BadFileDescriptor => write!(f, "Bad file descriptor"),
            &Self::DriverBadParams => write!(f, "Driver bad params"),
            &Self::Addrnotavail => write!(f, "Address is not avail"),
            &Self::Unkown => write!(f, "unkown error"),
            &Self::EofError => write!(f, "end of file error"),
        }
    }
}

impl From<i32> for ErrorKind {
    fn from(i: i32) -> ErrorKind {
        match i { 
            -1 => ErrorKind::EofError,
            -2 => ErrorKind::ConnectError,
           -5 => ErrorKind::Addrnotavail,
           -11 => ErrorKind::DriverBadOpen,
           -12 => ErrorKind::DriverBadParams,
           _ => ErrorKind::Unkown,
        }
    }
}

