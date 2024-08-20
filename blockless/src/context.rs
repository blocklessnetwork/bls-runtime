use std::sync::{Arc, Mutex};

use wasmtime::StoreLimits;
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi_threads::WasiThreadsCtx;

#[derive(Clone)]
pub(crate) struct BlocklessContext {
    pub(crate) preview1_ctx: Option<wasi_common::WasiCtx>,

    pub(crate) preview2_ctx: Option<Arc<Mutex<WasiP1Ctx>>>,

    pub(crate) wasi_threads: Option<Arc<WasiThreadsCtx<BlocklessContext>>>,

    pub(crate) store_limits: StoreLimits,
}

impl Default for BlocklessContext {
    fn default() -> Self {
        Self {
            preview1_ctx: None,
            preview2_ctx: None,
            wasi_threads: None,
            store_limits: Default::default(),
        }
    }
}

impl BlocklessContext {
    pub(crate) fn preview2_ctx(&mut self) -> &mut WasiP1Ctx {
        let ctx = self.preview2_ctx.as_mut().unwrap();
        Arc::get_mut(ctx)
            .expect("wasmtime_wasi was not compatiable threads")
            .get_mut()
            .unwrap()
    }
}

impl wasmtime_wasi::WasiView for BlocklessContext {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        self.preview2_ctx().table()
    }

    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        self.preview2_ctx().ctx()
    }
}
