use quillai_log::{LogLevel, LogConfig, LogFormat, init_logger, init_simple_logger};
use quillai_log::{info, debug, warn, error};
use quillai_log::{info_span, create_span, Level};

#[test]
fn test_log_level_from_str() {
    assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
    assert_eq!("INFO".parse::<LogLevel>().unwrap(), LogLevel::Info);
    assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
    assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
    assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
    assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
    assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
    assert_eq!("off".parse::<LogLevel>().unwrap(), LogLevel::Off);
    
    assert!("invalid".parse::<LogLevel>().is_err());
}

#[test]
fn test_log_level_display() {
    assert_eq!(LogLevel::Info.to_string(), "info");
    assert_eq!(LogLevel::Warn.to_string(), "warn");
    assert_eq!(LogLevel::Error.to_string(), "error");
    assert_eq!(LogLevel::Debug.to_string(), "debug");
    assert_eq!(LogLevel::Trace.to_string(), "trace");
    assert_eq!(LogLevel::Off.to_string(), "off");
}

#[test]
fn test_log_level_conversion() {
    let level: Option<Level> = LogLevel::Info.into();
    assert_eq!(level, Some(Level::INFO));
    
    let level: Option<Level> = LogLevel::Off.into();
    assert_eq!(level, None);
    
    let filter: String = LogLevel::Debug.into();
    assert_eq!(filter, "debug");
}

#[test]
fn test_simple_logger_initialization() {
    // This should not fail for Off level
    let result = init_simple_logger(LogLevel::Off);
    // Note: This might fail if called multiple times, which is expected behavior
    // for tracing subscriber initialization
    match result {
        Ok(()) => {
            // Test that we can use logging macros after initialization
            info!("Test log message");
            debug!(key = "value", "Test debug message");
        }
        Err(_) => {
            // Expected if subscriber is already initialized
            info!("Logger already initialized");
        }
    }
}

#[test]
fn test_log_config_default() {
    let config = LogConfig::default();
    assert_eq!(config.format, LogFormat::Pretty);
    assert!(config.with_timestamp);
    assert!(!config.with_target);
    assert!(!config.with_thread_names);
    assert!(!config.with_line_number);
    assert!(config.env_filter.is_none());
}

#[test]
fn test_advanced_logger_initialization() {
    let config = LogConfig {
        format: LogFormat::Compact,
        with_timestamp: false,
        with_target: true,
        with_thread_names: false,
        with_line_number: true,
        env_filter: Some("debug".to_string()),
    };
    
    // This might fail if subscriber is already initialized, which is fine
    let _ = init_logger(LogLevel::Debug, &config);
}

#[test]
fn test_span_creation() {
    // Initialize logger for span tests
    let _ = init_simple_logger(LogLevel::Off);
    
    let span = info_span!("test_span");
    let _enter = span.enter();
    
    info!("Message within span");
    
    let custom_span = create_span(Level::DEBUG, "custom_span");
    let _enter2 = custom_span.enter();
    
    debug!("Message in custom span");
}

#[test]
fn test_structured_logging() {
    let _ = init_simple_logger(LogLevel::Off);
    
    // Test structured logging with various field types
    info!(
        user_id = 42,
        action = "login",
        success = true,
        duration_ms = 123.45,
        "User authentication"
    );
    
    warn!(
        attempt = 3,
        max_attempts = 5,
        ip_address = "192.168.1.1",
        "Login attempt warning"
    );
    
    error!(
        error_code = 500,
        message = "Database connection failed",
        retry_count = 2,
        "Critical error occurred"
    );
}

#[test]
fn test_log_format_equality() {
    assert_eq!(LogFormat::Pretty, LogFormat::Pretty);
    assert_eq!(LogFormat::Compact, LogFormat::Compact);
    assert_eq!(LogFormat::Json, LogFormat::Json);
    assert_ne!(LogFormat::Pretty, LogFormat::Compact);
}

#[test]
fn test_error_types() {
    use quillai_log::Error;
    
    let err = Error::InvalidLogLevel("invalid".to_string());
    assert!(format!("{}", err).contains("invalid log level"));
    
    let err = Error::InitializationFailed("test".to_string());
    assert!(format!("{}", err).contains("initialization failed"));
}

#[cfg(feature = "json")]
#[test]
fn test_json_format_with_feature() {
    let config = LogConfig {
        format: LogFormat::Json,
        ..Default::default()
    };
    
    // This should work when json feature is enabled
    let result = init_logger(LogLevel::Off, &config);
    // Might fail if already initialized, but shouldn't fail due to missing feature
    match result {
        Ok(()) => {
            info!(test_field = "test_value", "JSON format test");
        }
        Err(e) => {
            // Should not be a feature error
            assert!(!format!("{}", e).contains("JSON format requires"));
        }
    }
}

#[cfg(not(feature = "json"))]
#[test]
fn test_json_format_without_feature() {
    let config = LogConfig {
        format: LogFormat::Json,
        ..Default::default()
    };
    
    // This should fail when json feature is not enabled
    let result = init_logger(LogLevel::Off, &config);
    assert!(result.is_err());
    
    if let Err(e) = result {
        assert!(format!("{}", e).contains("JSON format requires"));
    }
}