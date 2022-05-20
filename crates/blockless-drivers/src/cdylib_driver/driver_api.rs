use super::{OpenFuncType, ReadFuncType, WriteFuncType};


pub struct DriverApi {
    api_open: OpenFuncType,
    api_read: ReadFuncType,
    api_write: WriteFuncType,
}

impl DriverApi {

    pub fn new(api_open: OpenFuncType, api_read: ReadFuncType, api_write: WriteFuncType) -> Self {
        DriverApi { 
            api_open, 
            api_read, 
            api_write 
        }
    }

    pub fn blockless_open(&self, uri: &str, opts: &str) -> i32 {
        unsafe {
            let uri_len: i32  = uri.len() as _;
            let opts_len: i32  = opts.len() as _;
            (self.api_open)(uri.as_ptr(), uri_len, opts.as_ptr(), opts_len)
        }
    }

    pub fn blockless_read(&self, fd: i32, buf: &mut [u8]) -> i32 {
        unsafe {
            let buf_len: i32  = buf.len() as _;
            (self.api_read)(fd, buf.as_mut_ptr(), buf_len)
        }
    }

    pub fn blockless_write(&self, fd: i32, buf: &[u8]) -> i32 {
        unsafe {
            let buf_len: i32  = buf.len() as _;
            (self.api_write)(fd, buf.as_ptr(), buf_len)
        }
    }
}

impl Clone for DriverApi {
    fn clone(&self) -> DriverApi {
        DriverApi{
            api_open: self.api_open,
            api_read: self.api_read,
            api_write: self.api_write,
        }
    }
}