use clap::ValueEnum;

#[derive(thiserror::Error)]
pub enum Error {
    #[error("invalid: {0}")]
    Invalid(String),
}

/// Format error messages for display
pub(crate) fn format_error(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter,
) -> std::fmt::Result {
    write!(f, "{e}")?;

    let mut source = e.source();

    if e.source().is_some() {
        writeln!(f, "\ncaused by:")?;
        let mut i: usize = 0;
        while let Some(inner) = source {
            writeln!(f, "{i: >5}: {inner}")?;
            source = inner.source();
            i += 1;
        }
    }

    Ok(())
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format_error(self, f)
    }
}

#[derive(ValueEnum, Debug, Default, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Error,
    #[default]
    Warn,
    Info,
    Debug,
    Trace,
    Off,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Off => write!(f, "off"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "error" => Ok(LogLevel::Error),
            "Error" => Ok(LogLevel::Error),
            "ERROR" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "Warn" => Ok(LogLevel::Warn),
            "WARN" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "Info" => Ok(LogLevel::Info),
            "INFO" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "Debug" => Ok(LogLevel::Debug),
            "DEBUG" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            "Trace" => Ok(LogLevel::Trace),
            "TRACE" => Ok(LogLevel::Trace),
            "off" => Ok(LogLevel::Off),
            "Off" => Ok(LogLevel::Off),
            "OFF" => Ok(LogLevel::Off),
            e => Err(Self::Err::Invalid(e.to_string())),
        }
    }
}

impl From<LogLevel> for log::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
            LogLevel::Off => log::LevelFilter::Off,
        }
    }
}
