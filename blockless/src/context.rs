use std::sync::Arc;

use wasmtime_wasi::preview2;

#[derive(Clone)]
pub(crate) struct BlocklessContext {
    pub(crate) preview1_ctx: Option<wasmtime_wasi::WasiCtx>,
    
    pub(crate) preview2_ctx: Option<Arc<preview2::WasiCtx>>,

    pub(crate) preview2_table: Arc<preview2::Table>,
    
    pub(crate) preview2_adapter: Arc<preview2::preview1::WasiPreview1Adapter>,
}

impl Default for BlocklessContext {
    fn default() -> Self {
        Self {
            preview1_ctx: None, 
            preview2_ctx: None, 
            preview2_adapter: Default::default(),
            preview2_table: Arc::new(preview2::Table::new())
        }
    }
}

impl preview2::WasiView for BlocklessContext {
    fn table(&self) -> &preview2::Table {
        &self.preview2_table
    }

    fn table_mut(&mut self) -> &mut preview2::Table {
        Arc::get_mut(&mut self.preview2_table).expect("preview2 is not compatible with threads")
    }

    fn ctx(&self) -> &preview2::WasiCtx {
        self.preview2_ctx.as_ref().unwrap()
    }

    fn ctx_mut(&mut self) -> &mut preview2::WasiCtx {
        let ctx = self.preview2_ctx.as_mut().unwrap();
        Arc::get_mut(ctx).expect("preview2 is not compatible with threads")
    }
}

impl preview2::preview1::WasiPreview1View for BlocklessContext {
    fn adapter(&self) -> &preview2::preview1::WasiPreview1Adapter {
        &self.preview2_adapter
    }

    fn adapter_mut(&mut self) -> &mut preview2::preview1::WasiPreview1Adapter {
        Arc::get_mut(&mut self.preview2_adapter).expect("preview2 is not compatible with threads")
    }
}

