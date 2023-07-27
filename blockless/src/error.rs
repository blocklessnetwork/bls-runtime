pub enum RegisterResult {
    Success,
    MemoryNotFound,
    Fail,
}

impl From<RegisterResult> for u32 {
    fn from(value: RegisterResult) -> Self {
        match value {
            RegisterResult::Success => 0,
            RegisterResult::MemoryNotFound => 1,
            RegisterResult::Fail => 2,
        }
    }
}

pub enum MCallResult {
    Success,
    MemoryNotFound,
    AllocError,
    DeallocError,
    MCallError,
    Fail,
}

impl From<MCallResult> for u32 {
    fn from(value: MCallResult) -> Self {
        match value {
            MCallResult::Success => 0,
            MCallResult::MemoryNotFound => 1,
            MCallResult::AllocError => 2,
            MCallResult::DeallocError => 3,
            MCallResult::MCallError => 4,
            MCallResult::Fail => 5,
        }
    }
}