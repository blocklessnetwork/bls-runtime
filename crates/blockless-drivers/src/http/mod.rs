use dlopen::raw::Library;
use log::error;

use crate::HttpErrorKind;

type ReqFuncType = unsafe extern "C" fn(url: *const u8, url_len: u32, opts: *const u8, opts_len: u32, fd: *mut u32) -> u32;
type ReadBodyFuncType = unsafe extern "C" fn(fd: u32, buf: *mut u8, buf_len: u32, num: *mut u32) -> u32;
type ReadHeadFuncType = unsafe extern "C" fn(fd: u32, name: *const u8, name_len: u32, head_buf: *mut u8, head_buf_len: u32, num: *mut u32) -> u32;
type CloseFuncType = unsafe extern "C" fn(fd: u32) -> u32;

pub struct HttpDriver {
    api_req: ReqFuncType,
    api_read_body: ReadBodyFuncType,
    api_read_head: ReadHeadFuncType,
    api_close: CloseFuncType,
    _lib: Library,
}

impl HttpDriver {
    pub fn http_req(&self, url: &str, opts: &str) -> Result<u32, HttpErrorKind> {
        unsafe {
            let url_len = url.len() as _;
            let opts_len = opts.len() as _;
            let mut fd = 0;
            let rs = (self.api_req)(url.as_ptr(), url_len, opts.as_ptr(), opts_len, &mut fd as _);
            if rs != 0 {
                return Err(HttpErrorKind::from(rs));
            }
            Ok(fd)
        }
    }

    pub fn http_read_head(&self, fd: u32, head: &str) -> Result<String, HttpErrorKind> {
        let mut buf: Vec<u8> = Vec::with_capacity(1024*9);
        let mut num: u32 = 0;
        unsafe {
            let head_len = head.len() as _;
            let buf_len = buf.len() as _;
            let rs = (self.api_read_head)(fd, head.as_ptr(), head_len, buf.as_mut_ptr(), buf_len, &mut num as _);
            if rs != 0 {
                error!("error read header {}", rs);
                return Err(HttpErrorKind::HeaderNotFound)
            }
            Ok(String::from_utf8_unchecked(buf))
        }
    }

    pub fn http_read_body(&self, fd: u32, buf: &mut [u8]) -> Result<u32, HttpErrorKind> {
        unsafe {
            let buf_len = buf.len() as _;
            let mut num: u32 = 0;
            let rs = (self.api_read_body)(fd, buf.as_mut_ptr(), buf_len, &mut num as _);
            if rs != 0 {
                error!("error read header {}", rs);
                return Err(HttpErrorKind::RequestError)
            }
            Ok(rs)
        }
    }

    pub fn http_close(&self, fd: u32) ->  Result<(), HttpErrorKind> {
        unsafe {
            let rs = (self.api_close)(fd);
            if rs != 0 {
                return Err(HttpErrorKind::from(rs));
            }
        }
        Ok(())
    }
}

static mut HTTPDRIVER: Option<HttpDriver> = None;

pub fn init_http_driver(path: &str) -> anyhow::Result<()> {
    let lib = Library::open(path)?;

    unsafe {
        let api_req = lib.symbol("http_req")?;
        let api_read_body = lib.symbol("http_read_body")?;
        let api_read_head = lib.symbol("http_read_head")?;
        let api_close = lib.symbol("http_close")?;
        HTTPDRIVER.replace(HttpDriver{
            api_req,
            api_read_body,
            api_read_head,
            api_close,
            _lib: lib,
        });
    }
    Ok(())
}

pub fn get_http_driver() -> Option<&'static HttpDriver> {
    unsafe {HTTPDRIVER.as_ref()}
}

