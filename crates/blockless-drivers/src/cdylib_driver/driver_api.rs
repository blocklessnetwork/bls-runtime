use super::{CloseFuncType, OpenFuncType, ReadFuncType, WriteFuncType};

pub struct DriverApi {
    api_open: OpenFuncType,
    api_read: ReadFuncType,
    api_write: WriteFuncType,
    api_close: CloseFuncType,
}

impl DriverApi {
    pub fn new(
        api_open: OpenFuncType,
        api_read: ReadFuncType,
        api_write: WriteFuncType,
        api_close: CloseFuncType,
    ) -> Self {
        DriverApi {
            api_open,
            api_read,
            api_write,
            api_close,
        }
    }

    pub fn blockless_open(&self, uri: &str, opts: &str, fd: &mut u32) -> u32 {
        unsafe {
            let uri_len: i32 = uri.len() as _;
            let opts_len: i32 = opts.len() as _;
            (self.api_open)(
                uri.as_ptr(),
                uri_len,
                opts.as_ptr(),
                opts_len,
                fd as *mut u32,
            )
        }
    }

    pub fn blockless_read(&self, fd: u32, buf: &mut [u8], rn: &mut i32) -> u32 {
        unsafe {
            let buf_len: i32 = buf.len() as _;
            (self.api_read)(fd, buf.as_mut_ptr(), buf_len, rn as *mut i32)
        }
    }

    pub fn blockless_write(&self, fd: u32, buf: &[u8], wn: &mut i32) -> u32 {
        unsafe {
            let buf_len: i32 = buf.len() as _;
            (self.api_write)(fd, buf.as_ptr(), buf_len, wn as *mut i32)
        }
    }

    pub fn blockless_close(&self, fd: u32) -> u32 {
        unsafe { (self.api_close)(fd) }
    }
}

impl Clone for DriverApi {
    fn clone(&self) -> DriverApi {
        DriverApi {
            api_open: self.api_open,
            api_read: self.api_read,
            api_write: self.api_write,
            api_close: self.api_close,
        }
    }
}
