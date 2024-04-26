use core::ffi;
use dlopen::raw::Library;
use std::path::Path;

use crate::error::CLIExitCode;

pub(crate) struct V86Lib {
    _lib: Library,
    v86_wasi_run_func: V86WasiRunFuncType,
}

type V86WasiRunFuncType =
    unsafe extern "C" fn(conf: *const ffi::c_char, len: ffi::c_int) -> ffi::c_int;

impl V86Lib {
    pub fn load<T: AsRef<Path>>(path: T) -> Result<V86Lib, CLIExitCode> {
        let lib = Library::open(path.as_ref()).map_err(|_| CLIExitCode::ConfigureError)?;
        let v86_wasi_run_func = unsafe {
            lib.symbol("run_v86_wasi")
                .map_err(|_| CLIExitCode::UnknownError("failed to load v86 wasi".to_string()))?
        };
        Ok(Self {
            _lib: lib,
            v86_wasi_run_func,
        })
    }

    pub fn v86_wasi_run(&self, path: &str) -> i32 {
        let path_len = path.len();
        let path_ptr = path.as_ptr();
        unsafe { (self.v86_wasi_run_func)(path_ptr as _, path_len as _) }
    }
}
