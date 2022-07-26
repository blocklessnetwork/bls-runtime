mod cdylib_driver;
pub mod error;
pub mod http_driver;
pub mod ipfs_driver;
pub mod s3_driver;
pub mod tcp_driver;
pub mod wasi;
pub mod read_ext;
use blockless_multiaddr as multiaddr;
pub use cdylib_driver::CdylibDriver;
pub use error::*;
use http_driver::{get_http_driver, init_http_driver};
use lazy_static::*;
use log::error;
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use tcp_driver::TcpDriver;
use wasi_common::WasiFile;


pub trait Driver {
    fn name(&self) -> &str;

    fn open(
        &self,
        uri: &str,
        opts: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn WasiFile>, ErrorKind>> + Send>>;
}

lazy_static! {
    pub static ref DRIVERS: Mutex<DriverConetxtImpl> = Mutex::new(DriverConetxtImpl::new());
}

pub struct DriverConetxtImpl {
    drivers: HashMap<String, Arc<dyn Driver + Sync + Send>>,
}

impl DriverConetxtImpl {
    fn new() -> Self {
        let ctx = DriverConetxtImpl {
            drivers: HashMap::new(),
        };
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

pub struct DriverConetxt;

impl DriverConetxt {
    pub fn find_driver(uri: &str) -> Option<Arc<dyn Driver + Sync + Send>> {
        let drv = DRIVERS.lock().unwrap();
        drv.find_driver(uri)
    }

    pub fn insert_driver<T: Driver + Sync + Send + 'static>(driver: T) {
        let mut drv = DRIVERS.lock().unwrap();
        drv.insert_driver(driver);
    }

    pub fn init_built_in_drivers(path: impl AsRef<Path>) {
        let tcp_driver_path = path.as_ref().join("http_driver.so");
        if tcp_driver_path.exists() {
            init_http_driver(tcp_driver_path.as_os_str()).unwrap();
        }

        Self::insert_driver(TcpDriver {});
    }
}
