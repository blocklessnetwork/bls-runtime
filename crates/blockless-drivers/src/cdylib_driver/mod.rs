mod driver_api;
mod driver_wasi_file;
use crate::{multiaddr, Driver, ErrorKind};
use anyhow::Result;
use dlopen::raw::Library;
use driver_api::DriverApi;
use driver_wasi_file::DriverWasiFile;
use log::error;
use wasi_common::WasiFile;

type OpenFuncType = unsafe extern "C" fn(
    uri: *const u8,
    uri_len: i32,
    opts: *const u8,
    opts_len: i32,
    fd: *mut i32,
) -> i32;
type ReadFuncType = unsafe extern "C" fn(fd: i32, buf: *mut u8, len: i32, n: *mut i32) -> i32;
type WriteFuncType = unsafe extern "C" fn(fd: i32, buf: *const u8, len: i32, n: *mut i32) -> i32;
type CloseFuncType = unsafe extern "C" fn(fd: i32) -> i32;

pub struct CdylibDriver {
    name: String,
    _path: String,
    _lib: Library,
    api: DriverApi,
}

impl CdylibDriver {
    pub fn load(path: &str, name: &str) -> Result<Self> {
        let path = path.into();
        let name = name.to_lowercase();
        let lib = Library::open(&path)?;
        let api_open: OpenFuncType;
        let api_read: ReadFuncType;
        let api_write: WriteFuncType;
        let api_close: CloseFuncType;
        unsafe {
            api_open = lib.symbol("blockless_open")?;
            api_read = lib.symbol("blockless_read")?;
            api_write = lib.symbol("blockless_write")?;
            api_close = lib.symbol("blockless_close")?;
        }
        Ok(Self {
            name,
            _path: path,
            _lib: lib,
            api: DriverApi::new(api_open, api_read, api_write, api_close),
        })
    }

    pub fn get_api(&self) -> DriverApi {
        self.api.clone()
    }
}

impl Driver for CdylibDriver {
    fn name(&self) -> &str {
        &self.name
    }

    fn open(
        &self,
        uri: &str,
        opts: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Box<dyn WasiFile>, crate::ErrorKind>> + Send>,
    > {
        let api = self.api.clone();
        let uri: String = uri.into();
        let opts: String = opts.into();
        return Box::pin(async move {
            let addr = match multiaddr::parse(uri.as_bytes()) {
                Err(e) => {
                    error!("error parse:{:?}", e);
                    return Err(ErrorKind::DriverBadParams);
                }
                Ok(addr) => addr,
            };
            let addr = addr
                .to_url_string()
                .map_err(|_| ErrorKind::DriverBadParams)?;
            let mut fd = -1;
            let rs = api.blockless_open(&addr, &opts, &mut fd);
            if rs != 0 {
                return Err(rs.into());
            }
            let file: DriverWasiFile = DriverWasiFile::new(api, fd)?;
            let file: Box<dyn WasiFile> = Box::new(file);
            Ok(file)
        });
    }
}
