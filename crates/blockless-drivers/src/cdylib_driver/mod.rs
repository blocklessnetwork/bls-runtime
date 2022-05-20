mod driver_api;
mod driver_wasi_file;
use driver_api::DriverApi;
use dlopen::raw::Library;
use anyhow::Result;

type OpenFuncType = unsafe extern "C" fn(uri: *const u8, uri_len: i32, opts: *const u8, opts_len: i32) -> i32;
type ReadFuncType = unsafe extern "C" fn(fd: i32, buf: *mut u8, len: i32) -> i32;
type WriteFuncType = unsafe extern "C" fn(fd: i32, buf: *const u8, len: i32) -> i32;

pub struct CdylibDriver {
    name: String,
    path: String,
    lib: Library,
    api: DriverApi,
}

impl CdylibDriver {

    pub fn load(path: &str, name: &str) -> Result<Self> {
        let path = path.into();
        let name = name.into();
        let lib = Library::open(&path)?;
        let api_open: OpenFuncType;
        let api_read: ReadFuncType;
        let api_write: WriteFuncType;
        unsafe {
            api_open = lib.symbol("blockless_open")?;
            api_read = lib.symbol("blockless_read")?;
            api_write = lib.symbol("blockless_write")?;
        }
        Ok(Self {
            name,
            path,
            lib,
            api: DriverApi::new(
                api_open,
                api_read,
                api_write,
            )
        })
    }

    pub fn get_api(&self) -> DriverApi {
        self.api.clone()
    }

}



