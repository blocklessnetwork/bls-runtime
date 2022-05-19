pub mod error;
pub mod tcp_driver;
pub mod wasi;
use blockless_multiaddr as multiaddr;
pub use error::*;
use lazy_static::*;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tcp_driver::TcpDriver;
use wasi_common::WasiCtx;
use wasi_common::WasiFile;
use log::error;

pub trait Driver {
    fn open(
        &self,
        uri: &str,
        opts: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn WasiFile>, ErrorKind>> + Send>>;
}

pub trait DriverConetxt {
    fn find_driver(&self, uri: &str) -> Option<Arc<dyn Driver + Sync + Send>>;
}

lazy_static! {
    pub static ref DRIVERS: DriverConetxtImpl = DriverConetxtImpl::new();
}

pub struct DriverConetxtImpl {
    drivers: HashMap<String, Arc<dyn Driver + Sync + Send>>,
}

impl DriverConetxtImpl {
    fn new() -> Self {
        let mut ctx = DriverConetxtImpl {
            drivers: HashMap::new(),
        };
        ctx.insert_driver("tcp", TcpDriver {});
        ctx
    }

    fn insert_driver(&mut self, key: &str, driver: impl Driver + Send + Sync + 'static) {
        self.drivers.insert(key.to_lowercase(), Arc::new(driver));
    }

    fn find_driver(&self, uri: &str) -> Option<Arc<dyn Driver + Sync + Send>> {
        let addr = match multiaddr::parse(uri.as_bytes()) {
            Err(e) => {
                error!("error parse:{:?}", e);
                return None;
            }
            Ok(addr) => addr,
        };
        let schema = match addr.schema() {
            Err(e) => {
                error!("get schema error:{:?}", e);
                return None;
            }
            Ok(s) => s.to_lowercase(),
        };
        self.drivers.get(&schema).map(|d| d.clone())
    }
}

impl DriverConetxt for WasiCtx {
    fn find_driver(&self, uri: &str) -> Option<Arc<dyn Driver + Sync + Send>> {
        DRIVERS.find_driver(uri)
    }
}
