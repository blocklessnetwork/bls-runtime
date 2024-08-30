use std::{
    fmt,
    process::{ExitCode, Termination},
};

#[derive(Debug, PartialEq)]
pub enum CliExitCode {
    Success,
    FlueUsedOut,
    CallStackExhausted,
    OutOfBoundsMemoryAccess,
    MisalignedMemoryAccess,
    UndefinedElement,
    UninitializedElement,
    IndirectCallTypeMismatch,
    IntegerOverflow,
    IntegerDivideByZero,
    InvalidConversionToInteger,
    UnreachableInstructionExecuted,
    Interrupt,
    DegenerateComponentAdapterCalled,
    AppTimeout,
    ConfigureError,
    UnknownError(String),
}

impl fmt::Display for CliExitCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CliExitCode::Success => write!(f, "Success"),
            CliExitCode::FlueUsedOut => write!(f, "The flue used out"),
            CliExitCode::CallStackExhausted => write!(f, "Call stack exhausted"),
            CliExitCode::OutOfBoundsMemoryAccess => write!(f, "Out of bounds memory access"),
            CliExitCode::MisalignedMemoryAccess => write!(f, "Misaligned memory access"),
            CliExitCode::UndefinedElement => {
                write!(f, "Undefined element: out of bounds table access")
            }
            CliExitCode::UninitializedElement => write!(f, "Uninitialized element"),
            CliExitCode::IndirectCallTypeMismatch => write!(f, "Indirect call type mismatch"),
            CliExitCode::IntegerOverflow => write!(f, "Integer overflow"),
            CliExitCode::IntegerDivideByZero => write!(f, "Integer divide by zero"),
            CliExitCode::InvalidConversionToInteger => write!(f, "Invalid conversion to integer"),
            CliExitCode::UnreachableInstructionExecuted => {
                write!(f, "wasm 'unreachable' instruction executed")
            }
            CliExitCode::Interrupt => write!(f, "Interrupt"),
            CliExitCode::DegenerateComponentAdapterCalled => {
                write!(f, "Degenerate component adapter called")
            }
            CliExitCode::AppTimeout => write!(f, "The app timeout"),
            CliExitCode::ConfigureError => write!(f, "The configure error"),
            CliExitCode::UnknownError(err_str) => write!(f, "Unknown error: {}", err_str),
        }
    }
}

// derived from README
impl From<i32> for CliExitCode {
    fn from(exitcode: i32) -> Self {
        match exitcode {
            0 => CliExitCode::Success,
            1 => CliExitCode::FlueUsedOut,
            2 => CliExitCode::CallStackExhausted,
            3 => CliExitCode::OutOfBoundsMemoryAccess,
            4 => CliExitCode::MisalignedMemoryAccess,
            5 => CliExitCode::UndefinedElement,
            6 => CliExitCode::UninitializedElement,
            7 => CliExitCode::IndirectCallTypeMismatch,
            8 => CliExitCode::IntegerOverflow,
            9 => CliExitCode::IntegerDivideByZero,
            10 => CliExitCode::InvalidConversionToInteger,
            11 => CliExitCode::UnreachableInstructionExecuted,
            12 => CliExitCode::Interrupt,
            13 => CliExitCode::DegenerateComponentAdapterCalled,
            // NOTE: where is 14?
            15 => CliExitCode::AppTimeout,
            128 => CliExitCode::ConfigureError,
            _ => CliExitCode::UnknownError(format!("exit code: {}", exitcode)),
        }
    }
}

impl From<u8> for CliExitCode {
    fn from(exitcode: u8) -> Self {
        Into::<i32>::into(exitcode).into()
    }
}

impl Into<u8> for CliExitCode {
    fn into(self) -> u8 {
        match self {
            CliExitCode::Success => 0,
            CliExitCode::FlueUsedOut => 1,
            CliExitCode::CallStackExhausted => 2,
            CliExitCode::OutOfBoundsMemoryAccess => 3,
            CliExitCode::MisalignedMemoryAccess => 4,
            CliExitCode::UndefinedElement => 5,
            CliExitCode::UninitializedElement => 6,
            CliExitCode::IndirectCallTypeMismatch => 7,
            CliExitCode::IntegerOverflow => 8,
            CliExitCode::IntegerDivideByZero => 9,
            CliExitCode::InvalidConversionToInteger => 10,
            CliExitCode::UnreachableInstructionExecuted => 11,
            CliExitCode::Interrupt => 12,
            CliExitCode::DegenerateComponentAdapterCalled => 13,
            // NOTE: where is 14?
            CliExitCode::AppTimeout => 15,
            CliExitCode::ConfigureError => 128,
            CliExitCode::UnknownError(_) => 255,
        }
    }
}

impl Into<i32> for CliExitCode {
    fn into(self) -> i32 {
        Into::<u8>::into(self) as i32
    }
}

impl std::error::Error for CliExitCode {}

impl Termination for CliExitCode {
    #[inline]
    fn report(self) -> ExitCode {
        ExitCode::from(Into::<u8>::into(self))
    }
}

#[cfg(test)]
mod tests {
    use super::CliExitCode;

    #[test]
    fn test_cli_exit_code_success() {
        // testing conversion from i32
        let from_i32: CliExitCode = 0.into();
        assert_eq!(from_i32, CliExitCode::Success);

        // testing conversion from u8
        let from_u8: CliExitCode = 0u8.into();
        assert_eq!(from_u8, CliExitCode::Success);

        // testing conversion into u8
        let into_u8: u8 = CliExitCode::Success.into();
        assert_eq!(into_u8, 0u8);

        // testing conversion into i32
        let into_i32: i32 = CliExitCode::Success.into();
        assert_eq!(into_i32, 0i32);
    }
}
