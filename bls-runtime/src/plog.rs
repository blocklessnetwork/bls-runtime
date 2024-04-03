use std::{fmt::Arguments, sync::Once};

use env_logger::Logger;
use log::{Log, Level, MetadataBuilder, Record};

static mut ENV_LOGGER: Option<Logger> = None;

/// The logger should only be set once in log crate by set_logger,
/// if set the console as output, can't be set the file as output.
/// Therefore, we need a logger where the console is set as output 
/// before initializing the env_log by the configure file.
pub fn env_logger() -> &'static mut Logger {
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            ENV_LOGGER = Some(Logger::from_default_env())
        });
        ENV_LOGGER.as_mut().unwrap()
    }
}

/// log info by level
pub fn plog(level: Level, args: Arguments<'_>) {
    let error_metadata = MetadataBuilder::new()
        .level(level)
        .build();
    let record = Record::builder()
        .args(args)
        .metadata(error_metadata)
        .build();
    env_logger().log(&record);
}


