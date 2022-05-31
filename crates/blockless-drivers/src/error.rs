pub use anyhow::{Context, Error};

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

#[derive(Debug)]
pub enum HttpErrorKind {
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
    TooManySessions,
}

impl std::error::Error for HttpErrorKind {}

impl std::fmt::Display for HttpErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Self::InvalidHandle => write!(f, "Invalid Error"),
            &Self::MemoryAccessError => write!(f, "Memoery Access Error"),
            &Self::BufferTooSmall => write!(f, "Buffer too small"),
            &Self::HeaderNotFound => write!(f, "Header not found"),
            &Self::Utf8Error => write!(f, "Utf8 error"),
            &Self::DestinationNotAllowed => write!(f, "Destination not allowed"),
            &Self::InvalidMethod => write!(f, "Invalid method"),
            &Self::InvalidEncoding => write!(f, "Invalid encoding"),
            &Self::InvalidUrl => write!(f, "Invalid url"),
            &Self::RequestError => write!(f, "Request url"),
            &Self::RuntimeError => write!(f, "Runtime error"),
            &Self::TooManySessions => write!(f, "Too many sessions"),
        }
    }
}
