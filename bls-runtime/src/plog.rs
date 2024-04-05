use std::{
    fmt::Arguments, sync::OnceLock
};

use env_logger::Logger;
use log::{
    Level, Log, MetadataBuilder, Record
};

static mut ENV_LOGGER: OnceLock<Logger> = OnceLock::new();

/// The logger should only be set once in log crate by set_logger,
/// if set the console as output, can't be set the file as output.
/// Therefore, we need a logger where the console is set as output
/// before initializing the env_log by the configure file.
pub(crate) fn env_logger() -> &'static Logger {
    unsafe {
        ENV_LOGGER.get_or_init(|| {
            // the log metadata
            let metadata = MetadataBuilder::new().build();
            let mut builder = env_logger::Builder::from_default_env();
            // set the console as output.
            builder.target(Default::default());
            let logger = builder.build();
            logger.enabled(&metadata);
            logger
        })
    }
}

/// log info by level
pub fn plog(level: Level, args: Arguments<'_>) {
    let record = Record::builder().args(args).level(level).build();
    env_logger().log(&record)
}

