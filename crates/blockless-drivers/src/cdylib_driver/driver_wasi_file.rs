use crate::error::*;

use super::driver_api::DriverApi;
use std::any::Any;
use wasi_common::Error;
use wasi_common::{file::FileType, WasiFile};

pub(crate) struct DriverWasiFile {
    api: DriverApi,
    fd: u32,
}

impl DriverWasiFile {
    pub(crate) fn new(api: DriverApi, fd: u32) -> Result<Self, ErrorKind> {
        Ok(DriverWasiFile { api, fd })
    }
}

impl Drop for DriverWasiFile {
    fn drop(&mut self) {
        if self.fd > 0 {
            self.api.blockless_close(self.fd);
        }
    }
}

#[async_trait::async_trait]
impl WasiFile for DriverWasiFile {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn get_filetype(&self) -> Result<FileType, Error> {
        Ok(FileType::BlockDevice)
    }

    async fn read_vectored<'a>(&self, _bufs: &mut [std::io::IoSliceMut<'a>]) -> Result<u64, Error> {
        let buf = _bufs
            .iter_mut()
            .find(|b| !b.is_empty())
            .map_or(&mut [][..], |b| &mut **b);
        let mut n = 0;
        let rs = self.api.blockless_read(self.fd, buf, &mut n);
        if rs != 0 {
            if rs == 0 {
                return Ok(n as _);
            }
            return Err(std::io::Error::from_raw_os_error(rs as _).into());
        } else {
            return Ok(n as _);
        }
    }

    async fn write_vectored<'a>(&self, _bufs: &[std::io::IoSlice<'a>]) -> Result<u64, Error> {
        let buf = _bufs
            .iter()
            .find(|b| !b.is_empty())
            .map_or(&[][..], |b| &**b);
        let mut n = 0;
        let rs = self.api.blockless_write(self.fd, buf, &mut n);
        if rs != 0 {
            return Err(std::io::Error::from_raw_os_error(rs as _).into());
        }
        match n.try_into() {
            Ok(o) => Ok(o),
            Err(_) => Err(std::io::Error::from_raw_os_error(rs as _).into()),
        }
    }
}
