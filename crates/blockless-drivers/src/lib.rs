pub mod error;
pub mod tcp_driver;
pub mod wasi;
pub use error::*;
use lazy_static::*;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use wasi_common::WasiCtx;
use wasi_common::WasiFile;

pub trait Driver {
    fn open(
        &mut self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn WasiFile>, ErrorKind>> + Send>>;
}

pub trait DriverConetxt {
    fn find_driver(&self, uri: &str) -> Option<Box<dyn Driver + Sync + Send>>;
}

lazy_static! {
    pub static ref DRIVERS: DriverConetxtImpl = DriverConetxtImpl {
        drivers: HashMap::new()
    };
}

pub struct DriverConetxtImpl {
    drivers: HashMap<String, Box<dyn Driver + Sync + Send>>,
}

impl DriverConetxtImpl {
    fn find_driver(&self, uri: &str) -> Option<Box<dyn Driver + Sync + Send>> {
        Some(Box::new(tcp_driver::TcpDriver {}))
    }
}

impl DriverConetxt for WasiCtx {
    fn find_driver(&self, uri: &str) -> Option<Box<dyn Driver + Sync + Send>> {
        DRIVERS.find_driver(uri)
    }
}
