use blockless_drivers_macro::linker_integration;

linker_integration!({
    witx: ["$BLOCKLESS_DRIVERS_ROOT/witx/blockless_drivers.witx"],
    target: blockless_drivers::wasi,
});
