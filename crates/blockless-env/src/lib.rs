use blockless_drivers_macro::linker_integration;

use wasi_common::WasiCtx;
use wasmtime::Linker;

linker_integration!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_drivers.witx"],
    target: blockless_drivers::wasi,
    link_method: "add_drivers_to_linker",
});

linker_integration!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_http.witx"],
    target: blockless_drivers::wasi::http,
    link_method: "add_http_to_linker",
});
