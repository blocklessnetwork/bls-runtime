use std::fmt::Display;

#[non_exhaustive]
#[derive(Debug)]
pub enum McallError {
    None,
    MemoryNotFound,
    AllocError,
    DeallocError,
    MCallMemoryNotFound,
    MCallError,
    Fail,
}

impl From<McallError> for u32 {
    fn from(value: McallError) -> Self {
        match value {
            McallError::None => 0,
            McallError::MemoryNotFound => 1,
            McallError::AllocError => 2,
            McallError::DeallocError => 3,
            McallError::MCallError => 4,
            McallError::Fail => 5,
            McallError::MCallMemoryNotFound => 6,
        }
    }
}

impl std::error::Error for McallError {}

impl Display for McallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            McallError::None => write!(f, "No Error"),
            McallError::MemoryNotFound => write!(f, "Memory not export"),
            McallError::AllocError => write!(f, "Alloc error"),
            McallError::DeallocError => write!(f, "Dealloc error"),
            McallError::MCallError => write!(f, "MCall error"),
            McallError::Fail => write!(f, "Call faill"),
            McallError::MCallMemoryNotFound => write!(f, "mcall memory not found"),
        }
    }
}
