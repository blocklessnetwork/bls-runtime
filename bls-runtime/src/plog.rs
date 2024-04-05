use std::{fmt::Arguments, sync::Mutex};
use once_cell::sync::Lazy;
use env_logger::{Builder, Logger};
use log::{Level, Log, MetadataBuilder, Record};

/// The logger should only be set once in log crate by set_logger,
/// if set the console as output, can't be set the file as output.
/// Therefore, we need a logger where the console is set as output
/// before initializing the env_log by the configure file.
static ENV_LOGGER: Lazy<Mutex<Logger>> = Lazy::new(|| {
    // the log metadata
    let metadata = MetadataBuilder::new().build();
    let mut builder = Builder::from_default_env();
    // set the console as output.
    builder.target(env_logger::Target::Stdout);
    let logger = builder.build();
    logger.enabled(&metadata);
    Mutex::new(logger)
});

/// log info by level
pub fn plog(level: Level, args: Arguments<'_>) {
    let logger = ENV_LOGGER.lock().unwrap();
    let record = Record::builder().args(args).level(level).build();
    logger.log(&record);
}
