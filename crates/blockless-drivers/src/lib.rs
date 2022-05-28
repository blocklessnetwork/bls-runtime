mod cdylib_driver;
pub mod error;
pub mod tcp_driver;
pub mod wasi;
use blockless_multiaddr as multiaddr;
pub use cdylib_driver::CdylibDriver;
pub use error::*;
use lazy_static::*;
use log::error;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use tcp_driver::TcpDriver;
use wasi_common::WasiCtx;
use wasi_common::WasiFile;

pub trait Driver {
    fn name(&self) -> &str;

    fn open(
        &self,
        uri: &str,
        opts: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn WasiFile>, ErrorKind>> + Send>>;
}

pub trait DriverConetxt {
    fn find_driver(&self, uri: &str) -> Option<Arc<dyn Driver + Sync + Send>>;
    fn insert_driver<T: Driver + Sync + Send + 'static>(&self, driver: T);
}

lazy_static! {
    pub static ref DRIVERS: Mutex<DriverConetxtImpl> = Mutex::new(DriverConetxtImpl::new());
}

pub struct DriverConetxtImpl {
    drivers: HashMap<String, Arc<dyn Driver + Sync + Send>>,
}

impl DriverConetxtImpl {
    fn new() -> Self {
        let mut ctx = DriverConetxtImpl {
            drivers: HashMap::new(),
        };
        ctx.insert_driver(TcpDriver {});
        ctx
    }

    fn insert_driver<T>(&mut self, driver: T)
    where
        T: Driver + Send + Sync + 'static,
    {
        let key = driver.name().to_lowercase();
        self.drivers.insert(key, Arc::new(driver));
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
        let drv = DRIVERS.lock().unwrap();
        drv.find_driver(uri)
    }

    fn insert_driver<T: Driver + Sync + Send + 'static>(&self, driver: T) {
        let mut drv = DRIVERS.lock().unwrap();
        drv.insert_driver(driver);
    }
}
