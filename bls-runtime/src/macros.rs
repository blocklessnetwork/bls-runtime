#[macro_export]
macro_rules! plog {
    ($level: expr, $($args:tt)+) => {{
        crate::plog::plog($level, std::format_args!($($args)+))
    }};
}

/// export the perror macro for log the error
#[macro_export]
macro_rules! perror {
    ($($args:tt)+) => {{
        crate::plog!(log::Level::Error, $($args)+)
    }};

    () => {{
        use log::Level;
        crate::plog!(Level::Error, "\n")
    }};
}

/// export the pinfo macro for log the info
#[macro_export]
macro_rules! pinfo {
    ($($args:tt)+) => {{
        crate::plog!(log::Level::Info, $($args)+)
    }};

    () => {{
        use log::Level;
        crate::plog!(Level::Info, "\n")
    }};
}
