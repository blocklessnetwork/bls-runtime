#![allow(non_upper_case_globals)]
use crate::{memory_driver, BlocklessMemoryErrorKind};
use std::env;
use wasi_common::WasiCtx;
use wiggle::{GuestMemory, GuestPtr};

wiggle::from_witx!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_memory.witx"],
    errors: { blockless_memory_error => BlocklessMemoryErrorKind },
    async: *,
});

impl types::UserErrorConversion for WasiCtx {
    fn blockless_memory_error_from_blockless_memory_error_kind(
        &mut self,
        e: self::BlocklessMemoryErrorKind,
    ) -> wiggle::anyhow::Result<types::BlocklessMemoryError> {
        e.try_into()
            .map_err(|e| wiggle::anyhow::anyhow!(format!("{:?}", e)))
    }
}

impl From<BlocklessMemoryErrorKind> for types::BlocklessMemoryError {
    fn from(e: BlocklessMemoryErrorKind) -> types::BlocklessMemoryError {
        use types::BlocklessMemoryError;
        match e {
            BlocklessMemoryErrorKind::InvalidHandle => BlocklessMemoryError::InvalidHandle,
            BlocklessMemoryErrorKind::RuntimeError => BlocklessMemoryError::RuntimeError,
            BlocklessMemoryErrorKind::InvalidParameter => BlocklessMemoryError::InvalidParameter,
        }
    }
}

impl wiggle::GuestErrorType for types::BlocklessMemoryError {
    fn success() -> Self {
        Self::Success
    }
}

#[wiggle::async_trait]
impl blockless_memory::BlocklessMemory for WasiCtx {
    async fn memory_read(
        &mut self,
        memory: &mut GuestMemory<'_>,
        buf: GuestPtr<u8>,
        buf_len: u32,
    ) -> Result<u32, BlocklessMemoryErrorKind> {
        let stdin = self.config_stdin_ref().unwrap();
        let mut dest_buf = vec![0; buf_len as _];
        let rs = memory_driver::read(&mut dest_buf, stdin.to_string()).await?;
        if rs > 0 {
            memory
                .copy_from_slice(&dest_buf[0..rs as _], buf.as_array(rs))
                .map_err(|_| BlocklessMemoryErrorKind::RuntimeError)?;
        }
        Ok(rs)
    }

    async fn env_var_read(
        &mut self,
        memory: &mut GuestMemory<'_>,
        buf: GuestPtr<u8>,
        buf_len: u32,
    ) -> Result<u32, BlocklessMemoryErrorKind> {
        // get the list of env_vars to load into the wasi assembly
        // from the BLS_LIST_VARS env var
        let env_var = match env::var_os("BLS_LIST_VARS") {
            Some(v) => v.into_string().unwrap(),
            None => "".to_string(),
        };

        let mut owned_string: String = "{".to_owned();
        for s in env_var.split(";") {
            let env_var = match env::var_os(s) {
                Some(v) => v.into_string().unwrap(),
                None => "".to_string(),
            };
            owned_string.push_str(&format!("\"{}\": \"{}\",", s, env_var));
        }
        owned_string.pop();
        owned_string.push_str(&"}");

        let mut dest_buf = vec![0; buf_len as _];
        let rs = memory_driver::read(&mut dest_buf, owned_string.to_string()).await?;
        if rs > 0 {
            memory
                .copy_from_slice(&dest_buf[0..rs as _], buf.as_array(rs))
                .map_err(|_| BlocklessMemoryErrorKind::RuntimeError)?;
        }
        Ok(rs)
    }
}
