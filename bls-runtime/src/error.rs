use std::{fmt, process::{ExitCode, Termination}};

#[derive(Debug)]
pub enum CLIExitCode {
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

impl fmt::Display for CLIExitCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CLIExitCode::FlueUsedOut => write!(f, "The flue used out"),
            CLIExitCode::CallStackExhausted => write!(f, "Call stack exhausted"),
            CLIExitCode::OutOfBoundsMemoryAccess => write!(f, "Out of bounds memory access"),
            CLIExitCode::MisalignedMemoryAccess => write!(f, "Misaligned memory access"),
            CLIExitCode::UndefinedElement => write!(f, "Undefined element: out of bounds table access"),
            CLIExitCode::UninitializedElement => write!(f, "Uninitialized element"),
            CLIExitCode::IndirectCallTypeMismatch => write!(f, "Indirect call type mismatch"),
            CLIExitCode::IntegerOverflow => write!(f, "Integer overflow"),
            CLIExitCode::IntegerDivideByZero => write!(f, "Integer divide by zero"),
            CLIExitCode::InvalidConversionToInteger => write!(f, "Invalid conversion to integer"),
            CLIExitCode::UnreachableInstructionExecuted => write!(f, "wasm 'unreachable' instruction executed"),
            CLIExitCode::Interrupt => write!(f, "Interrupt"),
            CLIExitCode::DegenerateComponentAdapterCalled => write!(f, "Degenerate component adapter called"),
            CLIExitCode::AppTimeout => write!(f, "The app timeout"),
            CLIExitCode::ConfigureError => write!(f, "The configure error"),
            CLIExitCode::UnknownError(err_str) => write!(f, "Unknown error: {}", err_str),
        }
    }
}

// derived from README
impl From<i32> for CLIExitCode {
  fn from(exitcode: i32) -> Self {
    match exitcode {
      1 => CLIExitCode::FlueUsedOut,
      2 => CLIExitCode::CallStackExhausted,
      3 => CLIExitCode::OutOfBoundsMemoryAccess,
      4 => CLIExitCode::MisalignedMemoryAccess,
      5 => CLIExitCode::UndefinedElement,
      6 => CLIExitCode::UninitializedElement,
      7 => CLIExitCode::IndirectCallTypeMismatch,
      8 => CLIExitCode::IntegerOverflow,
      9 => CLIExitCode::IntegerDivideByZero,
      10 => CLIExitCode::InvalidConversionToInteger,
      11 => CLIExitCode::UnreachableInstructionExecuted,
      12 => CLIExitCode::Interrupt,
      13 => CLIExitCode::DegenerateComponentAdapterCalled,
      // NOTE: where is 14?
      15 => CLIExitCode::AppTimeout,
      128 => CLIExitCode::ConfigureError,
      _ => CLIExitCode::UnknownError(format!("exit code: {}", exitcode)),
    }
  }
}

impl From<u8> for CLIExitCode {
  fn from(exitcode: u8) -> Self {
    Into::<i32>::into(exitcode).into()
  }
}

impl Into<u8> for CLIExitCode {
  fn into(self) -> u8 {
    match self {
      CLIExitCode::FlueUsedOut => 1,
      CLIExitCode::CallStackExhausted => 2,
      CLIExitCode::OutOfBoundsMemoryAccess => 3,
      CLIExitCode::MisalignedMemoryAccess => 4,
      CLIExitCode::UndefinedElement => 5,
      CLIExitCode::UninitializedElement => 6,
      CLIExitCode::IndirectCallTypeMismatch => 7,
      CLIExitCode::IntegerOverflow => 8,
      CLIExitCode::IntegerDivideByZero => 9,
      CLIExitCode::InvalidConversionToInteger => 10,
      CLIExitCode::UnreachableInstructionExecuted => 11,
      CLIExitCode::Interrupt => 12,
      CLIExitCode::DegenerateComponentAdapterCalled => 13,
      // NOTE: where is 14?
      CLIExitCode::AppTimeout => 15,
      CLIExitCode::ConfigureError => 128,
      CLIExitCode::UnknownError(_) => 255,
    }
  }
}

impl Into<i32> for CLIExitCode {
  fn into(self) -> i32 {
    Into::<u8>::into(self) as i32
  }
}

impl std::error::Error for CLIExitCode {}

impl Termination for CLIExitCode {
    #[inline]
    fn report(self) -> ExitCode {
        ExitCode::from(Into::<u8>::into(self))
    }
}
