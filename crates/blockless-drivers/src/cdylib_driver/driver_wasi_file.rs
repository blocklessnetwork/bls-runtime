use wasi_common::{WasiFile, file::FileType};
use std::any::Any;
use anyhow::Result;
use std::io;
use wasi_common::Error;
use super::driver_api::DriverApi;

struct DriverWasiFile {
    api: DriverApi,
    fd: i32,
}

#[async_trait::async_trait]
impl WasiFile for DriverWasiFile {

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn get_filetype(&mut self) -> Result<FileType> {
        Ok(FileType::BlockDevice)
    }

    async fn read_vectored<'a>(
        &mut self,
        slices: &mut [io::IoSliceMut<'a>],
    ) -> Result<u64, Error> {
        let buf = slices.iter_mut().find(|b| !b.is_empty()).map_or(&mut [][..], |b| &mut **b);
        let n = self.api.blockless_read(self.fd, buf);
        Ok(n.try_into()?)
    }

    async fn write_vectored<'a>(&mut self, slices: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        let buf = slices.iter().find(|b| !b.is_empty()).map_or(&[][..], |b| &**b);
        let n = self.api.blockless_write(self.fd, buf);
        Ok(n.try_into()?)
    }


}