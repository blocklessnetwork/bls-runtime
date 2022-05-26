use crate::error::*;

use super::driver_api::DriverApi;
use anyhow::Result;
use std::any::Any;
use std::io;
use anyhow::Error;
use wasi_common::{file::FileType, WasiFile};

pub(crate) struct DriverWasiFile {
    api: DriverApi,
    fd: i32,
}

impl DriverWasiFile {
    pub(crate) fn new(api: DriverApi, fd: i32) -> Result<Self, ErrorKind> {
        if fd < 0 {
            let e = ErrorKind::from(fd);
            return Err(e);
        }
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

    async fn get_filetype(&mut self) -> Result<FileType> {
        Ok(FileType::BlockDevice)
    }

    async fn read_vectored<'a>(&mut self, slices: &mut [io::IoSliceMut<'a>]) -> Result<u64, Error> {
        let buf = slices
            .iter_mut()
            .find(|b| !b.is_empty())
            .map_or(&mut [][..], |b| &mut **b);
        let mut n = 0;
        let rs = self.api.blockless_read(self.fd, buf, &mut n);
        if rs != 0 {
            if rs == -1 {
                return Ok(n as _);
            }
            return Err(ErrorKind::from(rs).into());
        } else {
            return Ok(n as _);
        }
    }

    async fn write_vectored<'a>(&mut self, slices: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        let buf = slices
            .iter()
            .find(|b| !b.is_empty())
            .map_or(&[][..], |b| &**b);
        let mut n = 0;
        let rs = self.api.blockless_write(self.fd, buf, &mut n);
        if  rs != 0 {
            return Err(ErrorKind::from(rs).into());
        }
        Ok(n.try_into()?)
    }
}
