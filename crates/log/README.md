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

### Understanding Spans

**Spans** are the core concept in structured tracing. Think of a span as a "unit of work" that:
- Has a name and duration (start/end time)
- Can carry structured fields (key-value pairs)
- Provides context to all logging that happens within it
- Can be nested to show relationships between operations
- Tracks the flow of execution across async boundaries

```rust
use quillai_log::{info_span, info, Instrument};

async fn handle_request(user_id: u64) {
    // Create a span that will track this entire request
    let span = info_span!("request", user_id = user_id, request_type = "api");
    
    async {
        info!("Processing user request");  // This log includes user_id automatically
        
        // Nested span for database operation
        let db_span = debug_span!("database", table = "users", operation = "SELECT");
        async {
            info!("Querying database");  // This log includes user_id + table + operation
        }
        .instrument(db_span)
        .await;
        
        info!("Request completed");  // Back to request span context
    }
    .instrument(span)  // All logs in this block will include the span context
    .await;
}
```

**Why spans are powerful:**
- **Automatic Context**: All logs within a span inherit its fields
- **Distributed Tracing**: Track requests across service boundaries  
- **Performance Monitoring**: Built-in timing and metrics
- **Debugging**: See the exact path of execution that led to an error
- **Async-Aware**: Context flows correctly across async operations

### Creating Custom Spans

```rust
use quillai_log::{Level, create_span, info};

let span = create_span(Level::INFO, "database_operation");
let _enter = span.enter();

info!(table = "users", operation = "select", "Querying database");
```

## Environment Variables

The logger respects standard tracing environment variables for fine-grained control:

### Basic Usage
```bash
# Set global log level
RUST_LOG=debug cargo run

# Set different levels for different crates
RUST_LOG=info,quillai_log=debug,reqwest=warn cargo run

# Very detailed logging
RUST_LOG=trace cargo run
```

### Advanced Filtering
```bash
# Only show logs from specific modules
RUST_LOG=quillai_log::auth,quillai_log::database cargo run

# Complex filtering with levels
RUST_LOG="info,hyper=warn,h2=off,rustls=error" cargo run

# Target-based filtering
RUST_LOG="[quillai_log::api]=debug,[hyper::client]=info" cargo run
```

### Production Examples
```bash
# Production: minimal logging
RUST_LOG=warn,quillai_log=info cargo run

# Debugging: detailed logs for specific components
RUST_LOG=info,quillai_log::auth=debug,sqlx=debug cargo run

# Performance monitoring: trace spans only
RUST_LOG="[{user_request}]=trace" cargo run
```

Environment filters override the programmatic log level, giving you runtime control without code changes.

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

## Examples

This crate includes several comprehensive examples that demonstrate different use cases:

### Basic Usage
```bash
cargo run --example basic_usage
```
Shows simple logging with different levels and structured fields.

### Comprehensive Logging
```bash
# Show all log levels including debug/trace
RUST_LOG=debug cargo run --example comprehensive_logging
```
Demonstrates:
- All log levels and structured logging
- Span-based context tracking across async operations
- Integration with external crates (reqwest/hyper)
- Error handling and performance monitoring

### Spans Demonstration
```bash
RUST_LOG=debug cargo run --example spans_demo
```
Deep dive into spans - the core feature of structured tracing:
- Basic span creation and manual/automatic entry
- Nested spans showing hierarchy and context inheritance
- Spans with fields that apply to all contained logs
- Async operations with concurrent span tracking
- Span timing and performance metrics
- Error handling within span contexts
- Advanced patterns and dynamic span creation

### JSON Output
```bash
cargo run --example json_output --features json
```
Shows structured JSON logging perfect for log aggregation systems.

### External Crate Integration

The library seamlessly integrates with the tracing ecosystem, so logs from external crates like `reqwest`, `hyper`, `sqlx`, `tokio`, etc. will automatically appear in your structured logs:

```rust
// Your application logs
info!(user_id = 42, "Processing request");

// External crate logs (reqwest/hyper) will also appear:
// DEBUG starting new connection: https://api.example.com/
// DEBUG connected to 1.2.3.4:443
```

This provides complete observability across your entire application stack.

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