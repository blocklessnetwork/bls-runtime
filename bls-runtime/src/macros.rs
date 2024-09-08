macro_rules! plog {
    ($level: expr, $($args:tt)+) => {{
        crate::plog::plog($level, std::format_args!($($args)+))
    }};
}

/// export the perror macro for log the error
macro_rules! perror {
    ($($args:tt)+) => {{
        plog!(log::Level::Error, $($args)+)
    }};

    () => {{
        use log::Level;
        crate::plog!(Level::Error, "\n")
    }};
}

/// export the pinfo macro for log the info
#[allow(unused_macros)]
macro_rules! pinfo {
    ($($args:tt)+) => {{
        crate::plog!(log::Level::Info, $($args)+)
    }};

    () => {{
        use log::Level;
        crate::plog!(Level::Info, "\n")
    }};
}

macro_rules! stdio_cfg {
    ($p:ident, $stdio:ident, $e: ident) => {
        match $p {
            Some(s) if s == "inherit" => $stdio::Inherit,
            Some(s) if s == "null" => $stdio::Null,
            Some(s) => $stdio::$e(s.to_string()),
            _ => $stdio::Inherit,
        }
    };
}
