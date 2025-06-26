//! QuillAI Logging - A structured logging library built on tracing
//!
//! This crate provides a convenient interface for setting up structured logging
//! using the `tracing` ecosystem, with support for both pretty-printed and JSON output.
//!
//! # Examples
//!
//! Basic usage:
//! ```rust
//! use quillai_log::{LogLevel, LogConfig, init_logger};
//!
//! // Initialize with default configuration
//! init_logger(LogLevel::Info, &LogConfig::default()).unwrap();
//!
//! // Use tracing macros
//! tracing::info!("Hello, world!");
//! tracing::debug!(user_id = 42, "User logged in");
//! ```

use clap::ValueEnum;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// Re-export tracing macros for convenience
pub use tracing::{debug, error, info, trace, warn};
pub use tracing::{debug_span, error_span, info_span, trace_span, warn_span};
pub use tracing::{event, span, Instrument, Level, Span};

#[derive(thiserror::Error)]
pub enum Error {
    #[error("invalid log level: {0}")]
    InvalidLogLevel(String),
    #[error("logger initialization failed: {0}")]
    InitializationFailed(String),
    #[error("tracing subscriber error: {0}")]
    TracingSubscriber(#[from] tracing_subscriber::util::TryInitError),
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

/// Log level configuration
#[derive(ValueEnum, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Only show error messages
    Error,
    /// Show warnings and errors
    #[default]
    Warn,
    /// Show info, warnings, and errors
    Info,
    /// Show debug info and above
    Debug,
    /// Show all messages including trace
    Trace,
    /// Disable all logging
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
        match s.to_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            "off" => Ok(LogLevel::Off),
            _ => Err(Self::Err::InvalidLogLevel(s.to_string())),
        }
    }
}

/// Convert LogLevel to tracing Level
impl From<LogLevel> for Option<Level> {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Some(Level::ERROR),
            LogLevel::Warn => Some(Level::WARN),
            LogLevel::Info => Some(Level::INFO),
            LogLevel::Debug => Some(Level::DEBUG),
            LogLevel::Trace => Some(Level::TRACE),
            LogLevel::Off => None,
        }
    }
}

/// Convert LogLevel to EnvFilter directive
impl From<LogLevel> for String {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => "error".to_string(),
            LogLevel::Warn => "warn".to_string(),
            LogLevel::Info => "info".to_string(),
            LogLevel::Debug => "debug".to_string(),
            LogLevel::Trace => "trace".to_string(),
            LogLevel::Off => "off".to_string(),
        }
    }
}

/// Logging configuration options
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Output format
    pub format: LogFormat,
    /// Whether to include timestamps
    pub with_timestamp: bool,
    /// Whether to include target module information
    pub with_target: bool,
    /// Whether to include thread information
    pub with_thread_names: bool,
    /// Whether to include line numbers
    pub with_line_number: bool,
    /// Custom environment filter (overrides log level if set)
    pub env_filter: Option<String>,
}

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Pretty-printed output for human reading
    Pretty,
    /// Compact pretty-printed output
    Compact,
    /// JSON output for structured logging
    Json,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Pretty,
            with_timestamp: true,
            with_target: false,
            with_thread_names: false,
            with_line_number: false,
            env_filter: None,
        }
    }
}

/// Initialize the global tracing subscriber
pub fn init_logger(level: LogLevel, config: &LogConfig) -> Result<(), Error> {
    let env_filter = if let Some(ref filter) = config.env_filter {
        EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(filter))
            .map_err(|e| Error::InitializationFailed(e.to_string()))?
    } else {
        EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(String::from(level)))
            .map_err(|e| Error::InitializationFailed(e.to_string()))?
    };

    match config.format {
        LogFormat::Pretty => {
            let fmt_layer = fmt::layer()
                .with_target(config.with_target)
                .with_thread_names(config.with_thread_names)
                .with_line_number(config.with_line_number);

            let fmt_layer = if config.with_timestamp {
                fmt_layer.boxed()
            } else {
                fmt_layer.without_time().boxed()
            };

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()?;
        }
        LogFormat::Compact => {
            let fmt_layer = fmt::layer()
                .compact()
                .with_target(config.with_target)
                .with_thread_names(config.with_thread_names)
                .with_line_number(config.with_line_number);

            let fmt_layer = if config.with_timestamp {
                fmt_layer.boxed()
            } else {
                fmt_layer.without_time().boxed()
            };

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()?;
        }
        LogFormat::Json => {
            #[cfg(feature = "json")]
            {
                let fmt_layer = fmt::layer()
                    .json()
                    .with_target(config.with_target)
                    .with_thread_names(config.with_thread_names)
                    .with_line_number(config.with_line_number);

                let fmt_layer = if config.with_timestamp {
                    fmt_layer.boxed()
                } else {
                    fmt_layer.without_time().boxed()
                };

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .try_init()?;
            }
            #[cfg(not(feature = "json"))]
            {
                return Err(Error::InitializationFailed(
                    "JSON format requires 'json' feature to be enabled".to_string(),
                ));
            }
        }
    }

    Ok(())
}

/// Initialize logger with simple configuration (for backward compatibility)
pub fn init_simple_logger(level: LogLevel) -> Result<(), Error> {
    init_logger(level, &LogConfig::default())
}

/// Create a span with the given name and level
pub fn create_span(level: Level, name: &'static str) -> Span {
    span!(level, name)
}

/// Create an info span (most common case)
pub fn info_span(name: &'static str) -> Span {
    info_span!(name)
}

/// Add fields to the current span
pub fn add_field<T: std::fmt::Debug>(key: &str, value: T) {
    Span::current().record(key, &tracing::field::debug(&value));
}

/// Testing utilities for applications using this logging library
#[cfg(test)]
pub mod testing {
    use super::*;
    
    /// Initialize a test logger that captures output for testing
    pub fn init_test_logger() -> Result<(), Error> {
        init_simple_logger(LogLevel::Off)
    }
    
    /// Initialize a test logger with specific level for testing
    pub fn init_test_logger_with_level(level: LogLevel) -> Result<(), Error> {
        init_simple_logger(level)
    }
}

// For backward compatibility with existing code using log crate
#[deprecated(since = "0.1.0", note = "Use tracing::Level instead")]
pub mod log_compat {
    use super::LogLevel;
    
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
}