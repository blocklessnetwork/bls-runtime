use core::ffi;
use std::path::Path;

use dlopen::raw::Library;
use anyhow::Result;

pub(crate) struct V86Lib {
    _lib: Library,
    v86_wasi_run_func: V86WasiRunFuncType,
}

type V86WasiRunFuncType = unsafe extern "C" fn(conf :*const ffi::c_char, len: ffi::c_int) -> ffi::c_int;

impl V86Lib {

    pub fn load<T: AsRef<Path>>(path: &str) -> Result<Self> {
        let lib = Library::open(path)?;
        let v86_wasi_run_func = unsafe {
            lib.symbol("run_v86_wasi")?
        };
        Ok(Self {
            _lib: lib,
            v86_wasi_run_func,
        })
    }

    pub fn v86_wasi_run(&self, path: &str) -> i32 {
        let path_len = path.len();
        let pasth_ptr = path.as_ptr();
        unsafe {
            (self.v86_wasi_run_func)(pasth_ptr as _, path_len as _)
        }
    }

}