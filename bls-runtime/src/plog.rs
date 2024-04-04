use std::{fmt::Arguments, sync::Once};

use env_logger::Logger;
use log::{Log, Level, Record};

static mut ENV_LOGGER: Option<Logger> = None;

/// The logger should only be set once in log crate by set_logger,
/// if set the console as output, can't be set the file as output.
/// Therefore, we need a logger where the console is set as output 
/// before initializing the env_log by the configure file.
pub(crate) fn env_logger() -> Option<&'static mut Logger> {
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            let logger = Logger::from_default_env();
            ENV_LOGGER = Some(logger);
        });
        ENV_LOGGER.as_mut()
    }
}

/// log info by level
pub fn plog(level: Level, args: Arguments<'_>) {
    let record = Record::builder()
        .args(args)
        .level(level)
        .build();
    env_logger().unwrap().log(&record);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_logger() {
        assert_eq!(env_logger().is_some(), true);
    }
}
