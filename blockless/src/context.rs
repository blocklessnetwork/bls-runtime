use std::sync::{Arc, Mutex};

use wasmtime_wasi_threads::WasiThreadsCtx;

#[derive(Clone)]
pub(crate) struct BlocklessContext {
    pub(crate) preview1_ctx: Option<wasi_common::WasiCtx>,

    pub(crate) preview2_ctx: Option<Arc<Mutex<wasmtime_wasi::WasiCtx>>>,

    pub(crate) preview2_table: Arc<Mutex<wasmtime::component::ResourceTable>>,

    pub(crate) preview2_adapter: Arc<wasmtime_wasi::preview1::WasiPreview1Adapter>,

    pub(crate) wasi_threads: Option<Arc<WasiThreadsCtx<BlocklessContext>>>,
}

impl Default for BlocklessContext {
    fn default() -> Self {
        Self {
            preview1_ctx: None,
            preview2_ctx: None,
            wasi_threads: None,
            preview2_adapter: Default::default(),
            preview2_table: Arc::new(Mutex::new(wasmtime::component::ResourceTable::new())),
        }
    }
}

impl wasmtime_wasi::WasiView for BlocklessContext {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        Arc::get_mut(&mut self.preview2_table)
            .expect("wasmtime_wasi was not compatiable threads")
            .get_mut()
            .unwrap()
    }

    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        let ctx = self.preview2_ctx.as_mut().unwrap();
        Arc::get_mut(ctx)
            .expect("wasmtime_wasi was not compatiable threads")
            .get_mut()
            .unwrap()
    }
}

impl wasmtime_wasi::preview1::WasiPreview1View for BlocklessContext {
    fn adapter(&self) -> &wasmtime_wasi::preview1::WasiPreview1Adapter {
        &self.preview2_adapter
    }

    fn adapter_mut(&mut self) -> &mut wasmtime_wasi::preview1::WasiPreview1Adapter {
        Arc::get_mut(&mut self.preview2_adapter).expect("preview2 is not compatible with threads")
    }
}
