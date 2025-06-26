# QuillAI Log

A structured logging library built on the `tracing` ecosystem, providing convenient interfaces for setting up logging with support for both pretty-printed and JSON output formats.

## Features

- **Structured Logging**: Built on `tracing` for structured, async-aware logging
- **Multiple Output Formats**: Pretty-printed, compact, and JSON output
- **Flexible Configuration**: Extensive configuration options for different use cases
- **CLI Integration**: Easy integration with `clap` for command-line applications
- **Performance**: Efficient async-aware logging with minimal overhead
- **Backward Compatibility**: Maintains compatibility with existing `log` crate usage

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
quillai_log = { version = "0.0.0", path = "../log" }

# For JSON output support
quillai_log = { version = "0.0.0", path = "../log", features = ["json"] }
```

### Basic Usage

```rust
use quillai_log::{LogLevel, init_simple_logger, info, debug, warn, error};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize with default pretty-printed output
    init_simple_logger(LogLevel::Info)?;
    
    // Use tracing macros
    info!("Application started");
    debug!(user_id = 42, "User logged in");
    warn!(attempts = 3, "Login attempts exceeded");
    error!("Failed to connect to database");
    
    Ok(())
}
```

### Advanced Configuration

```rust
use quillai_log::{LogLevel, LogConfig, LogFormat, init_logger, info_span};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LogConfig {
        format: LogFormat::Pretty,
        with_timestamp: true,
        with_target: true,
        with_thread_names: false,
        with_line_number: true,
        env_filter: Some("info,hyper=warn".to_string()),
    };
    
    init_logger(LogLevel::Debug, &config)?;
    
    // Create a span for structured context
    let span = info_span!("request");
    let _enter = span.enter();
    
    info!("Processing request");
    
    Ok(())
}
```

### CLI Integration

```rust
use clap::Parser;
use quillai_log::{LogLevel, init_simple_logger};

#[derive(Parser)]
struct Args {
    #[clap(long, value_enum, default_value = "info")]
    log_level: LogLevel,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    init_simple_logger(args.log_level)?;
    
    // Your application logic here
    
    Ok(())
}
```

## Configuration Options

### LogLevel

Available log levels (from most to least verbose):

- `Trace`: Most verbose, includes all messages
- `Debug`: Debug information and above
- `Info`: Informational messages and above (default)
- `Warn`: Warnings and errors only
- `Error`: Only error messages
- `Off`: Disable all logging

### LogFormat

- `Pretty`: Human-readable format with colors (default)
- `Compact`: Compact pretty-printed format
- `Json`: Structured JSON output (requires `json` feature)

### LogConfig

```rust
pub struct LogConfig {
    /// Output format (Pretty, Compact, Json)
    pub format: LogFormat,
    /// Include timestamps in output
    pub with_timestamp: bool,
    /// Include module/target information
    pub with_target: bool,
    /// Include thread names
    pub with_thread_names: bool,
    /// Include line numbers
    pub with_line_number: bool,
    /// Custom environment filter
    pub env_filter: Option<String>,
}
```

## Structured Logging Examples

### Adding Context to Logs

```rust
use quillai_log::{info, debug};

// Simple structured logging
info!(user_id = 42, action = "login", "User authentication successful");

// Multiple fields
debug!(
    request_id = "abc123",
    method = "POST", 
    path = "/api/users",
    duration_ms = 156,
    "API request completed"
);
```

### Using Spans for Context

```rust
use quillai_log::{info_span, info, Instrument};

async fn handle_request(user_id: u64) {
    let span = info_span!("request", user_id = user_id);
    
    async {
        info!("Processing user request");
        // ... your logic here
        info!("Request completed");
    }
    .instrument(span)
    .await;
}
```

### Creating Custom Spans

```rust
use quillai_log::{Level, create_span, info};

let span = create_span(Level::INFO, "database_operation");
let _enter = span.enter();

info!(table = "users", operation = "select", "Querying database");
```

## Environment Variables

The logger respects standard tracing environment variables:

- `RUST_LOG`: Set log level and filters (e.g., `RUST_LOG=debug,hyper=warn`)
- Environment filters override the programmatic log level

## JSON Output

Enable JSON output with the `json` feature:

```toml
[dependencies]
quillai_log = { version = "0.0.0", path = "../log", features = ["json"] }
```

```rust
use quillai_log::{LogLevel, LogConfig, LogFormat, init_logger};

let config = LogConfig {
    format: LogFormat::Json,
    ..Default::default()
};

init_logger(LogLevel::Info, &config)?;
```

Example JSON output:
```json
{"timestamp":"2024-01-15T10:30:45.123456Z","level":"INFO","message":"User logged in","fields":{"user_id":42}}
```

## Error Handling

The library provides detailed error information:

```rust
match init_simple_logger(LogLevel::Info) {
    Ok(()) => println!("Logger initialized successfully"),
    Err(e) => eprintln!("Failed to initialize logger: {}", e),
}
```

## Migration from `log` Crate

If you're migrating from the standard `log` crate:

1. Replace `log::info!` with `quillai_log::info!` (or import the macros)
2. Replace logger initialization with `init_simple_logger()`
3. Take advantage of structured logging by adding fields to your log statements

For backward compatibility, the old `log::LevelFilter` conversion is still available:

```rust
use quillai_log::log_compat::*; // Deprecated, but available
```

## Performance

The `tracing` ecosystem is designed for high-performance, async-aware logging:

- Zero-cost when disabled
- Efficient structured data handling
- Async-aware span tracking
- Minimal allocation overhead

## Testing

For testing applications that use this logging library, you can disable logging or capture output:

```rust
#[cfg(test)]
mod tests {
    use quillai_log::{LogLevel, init_simple_logger};
    
    #[test]
    fn test_with_logging() {
        // Initialize with Off level for tests
        init_simple_logger(LogLevel::Off).unwrap();
        
        // Your test code here
    }
}
```

## Contributing

This crate is part of the QuillAI project. Contributions are welcome!

## License

This project is licensed under the MIT License.