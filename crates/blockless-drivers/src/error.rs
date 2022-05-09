pub use anyhow::{Context, Error};

/// Internal error type for the `wasi-common` crate.
/// Contains variants of the WASI `$errno` type are added according to what is actually used internally by
/// the crate. Not all values are represented presently.
#[derive(Debug)]
pub enum ErrorKind {
    ConnectError,
    MemoryNotExport,
    DriverNotFound,
}

impl std::error::Error for ErrorKind {}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Self::ConnectError => write!(f, "Connect Error"),
            &Self::MemoryNotExport => write!(f, "Memoery not export"),
            &Self::DriverNotFound => write!(f, "Driver not found."),
        }
    }
}
