//! Basic usage example for QuillAI Log
//!
//! This example demonstrates the most common logging patterns you'll use
//! in everyday applications.
//!
//! Run with:
//! ```bash
//! cargo run --example basic_usage
//! RUST_LOG=debug cargo run --example basic_usage
//! ```

use quillai_log::{LogLevel, init_simple_logger, info, debug, warn, error};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with Info level
    init_simple_logger(LogLevel::Info)?;
    
    println!("🌟 QuillAI Log - Basic Usage Example");
    println!("====================================\n");
    
    // Simple string messages
    info!("Application starting up");
    warn!("This is a warning message");
    error!("This is an error message");
    debug!("This debug message won't show (filtered out by Info level)");
    
    println!();
    
    // Structured logging with fields
    info!(
        version = "1.0.0",
        config_file = "/etc/myapp.conf",
        "Application configuration loaded"
    );
    
    // User activity logging
    info!(
        user_id = 42,
        action = "login",
        ip_address = "192.168.1.100",
        "User authentication successful"
    );
    
    // Performance monitoring
    warn!(
        operation = "database_query",
        duration_ms = 1250,
        threshold_ms = 1000,
        "Query execution time exceeded threshold"
    );
    
    // Error with context
    error!(
        error_code = "DB_CONN_FAILED",
        database_url = "postgresql://localhost:5432/mydb",
        retry_count = 3,
        "Failed to connect to database after retries"
    );
    
    info!("Application shutting down gracefully");
    
    println!("\n✅ Basic logging example completed!");
    println!("💡 Try running with RUST_LOG=debug to see debug messages");
    
    Ok(())
}