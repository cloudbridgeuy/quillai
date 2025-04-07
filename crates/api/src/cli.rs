use clap::{Parser, ValueEnum};

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
    type Err = crate::error::Error;

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
            _ => Err(Self::Err::InvalidLogLevel),
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

#[derive(Debug, Parser)]
#[command(name = "quillai-api")]
#[command(about = "Quillai API")]
pub struct App {
    /// Host to attach the service to.
    #[clap(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port the API should listen.
    #[clap(long, default_value = "2908")]
    pub port: u32,

    /// Database host url
    #[clap(long, default_value = "sqlite://", env = "QUILLAI_API_DB_URL")]
    pub db_url: String,

    /// Database maximum connections
    #[clap(long, default_value = "1", env = "QUILLAI_API_DB_MAX_CONNECTIONS")]
    pub db_max_connections: u32,

    /// Database acquire timeout
    #[clap(long, default_value = "3", env = "QUILLAI_API_DB_ACQUIRE_TIMEOUT")]
    pub db_acquire_timeout: u64,

    /// Log level
    #[clap(
        long,
        value_enum,
        default_value = "warn",
        env = "QUILLAI_API_LOG_LEVEL"
    )]
    pub log_level: LogLevel,
}
