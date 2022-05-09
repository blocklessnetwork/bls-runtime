fn main() {
    let cwd = std::env::current_dir().unwrap();
    let wasi = cwd.join("../blockless-drivers");
    // this will be available to dependent crates via the DEP_WASI_COMMON_19_WASI env var:
    println!("cargo:rustc-env=BLOCKLESS_DRIVERS_ROOT={}", wasi.display());
}
