pub enum Stdout {
    //no stdout.
    Null,
    //inherit stdout.
    Inherit,
    //stdout redirect to file.
    FileName(String),
}

pub struct BlocklessConfig {
    wasm_file: String,
    root_path: Option<String>,
    stdout: Stdout,
    limited_fuel: Option<u64>,
    limited_time: Option<u64>,
    limited_memory: Option<u64>,
}

impl BlocklessConfig {
    pub fn wasm_file_ref(&self) -> &str {
        &self.wasm_file
    }

    pub fn root_path(&mut self, r: &str) {
        self.root_path = Some(r.into());
    }

    pub fn root_path_ref(&self) -> Option<&str> {
        self.root_path.as_ref().map(|x| x.as_str())
    }

    pub fn new(wasm_file: &str) -> BlocklessConfig {
        Self {
            wasm_file: String::from(wasm_file),
            root_path: None,
            stdout: Stdout::Inherit,
            limited_fuel: None,
            limited_time: None,
            limited_memory: None,
        }
    }

    /// stdout file must be work in sandbox root_path,
    /// if root_path is not setting, the stdout file will use Inherit
    pub fn stdout(&mut self, stdout: Stdout) {
        self.stdout = stdout
    }

    pub fn stdout_ref(&self) -> &Stdout {
        &self.stdout
    }

    pub fn limited_time(&mut self, time: Option<u64>)  {
        self.limited_time = time
    }

    pub fn get_limited_time(&self) -> Option<u64>  {
        self.limited_time
    }

    pub fn limited_fuel(&mut self, fuel: Option<u64>)  {
        self.limited_fuel = fuel
    }

    pub fn get_limited_fuel(&self) -> Option<u64> {
        self.limited_fuel
    }

    pub fn limited_memory(&mut self, m: Option<u64>)  {
        self.limited_memory = m
    }

    pub fn get_limited_memory(&self) -> Option<u64>  {
        self.limited_memory
    }

}
