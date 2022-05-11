pub struct BlocklessConfig {
    wasm_file: String,
    root_path: Option<String>,
}

impl BlocklessConfig {
    pub fn wasm_file_ref(&self) -> &str {
        &self.wasm_file
    }

    pub fn root_path_ref(&self) -> Option<&str> {
        self.root_path.as_ref().map(|x| x.as_str())
    }

    pub fn new(wasm_file: &str) -> BlocklessConfig {
        Self {
            wasm_file: String::from(wasm_file),
            root_path: None,
        }
    }
}
