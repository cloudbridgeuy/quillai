//! Comprehensive example demonstrating QuillAI Log capabilities
//!
//! This example shows:
//! - Different log levels and their output
//! - Structured logging with fields
//! - Span creation and context tracking
//! - Integration with external crates (reqwest)
//! - Different output formats
//! - Error handling and debugging
//!
//! Run with different log levels:
//! ```bash
//! cargo run --example comprehensive_logging
//! RUST_LOG=debug cargo run --example comprehensive_logging
//! RUST_LOG=trace cargo run --example comprehensive_logging
//! ```

use quillai_log::{
    LogLevel, LogConfig, LogFormat, init_logger, init_simple_logger,
    info, debug, warn, error, trace,
    info_span, debug_span, create_span, Level, Instrument
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    users: Vec<User>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 QuillAI Log Comprehensive Example");
    println!("=====================================\n");

    // Example 1: Basic logging with different levels
    demo_basic_logging().await?;
    
    // Example 2: Structured logging
    demo_structured_logging().await?;
    
    // Example 3: Span-based context tracking
    demo_span_tracking().await?;
    
    // Example 4: Integration with external crates
    demo_external_crate_integration().await?;
    
    // Example 5: Different output formats
    demo_output_formats().await?;
    
    // Example 6: Error handling and debugging
    demo_error_handling().await?;

    println!("\n✅ All logging examples completed!");
    Ok(())
}

async fn demo_basic_logging() -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 Demo 1: Basic Logging with Different Levels");
    println!("-----------------------------------------------");
    
    // Initialize with Info level (default)
    init_simple_logger(LogLevel::Info)?;
    
    trace!("This is a TRACE message - won't show with Info level");
    debug!("This is a DEBUG message - won't show with Info level");
    info!("This is an INFO message - will show");
    warn!("This is a WARN message - will show");
    error!("This is an ERROR message - will show");
    
    println!("ℹ️  Only INFO, WARN, and ERROR messages are shown above (trace/debug filtered out)\n");
    
    Ok(())
}

async fn demo_structured_logging() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Demo 2: Structured Logging with Fields");
    println!("------------------------------------------");
    
    // User authentication example
    info!(
        user_id = 12345,
        username = "alice",
        action = "login",
        success = true,
        session_duration = 3600,
        ip_address = "192.168.1.100",
        user_agent = "Mozilla/5.0 (Chrome)",
        "User successfully authenticated"
    );
    
    // API request example
    info!(
        request_id = "req_abc123",
        method = "POST",
        path = "/api/v1/users",
        status_code = 201,
        response_time_ms = 45.67,
        content_length = 1024,
        "API request processed"
    );
    
    // Database operation example
    debug!(
        query = "SELECT * FROM users WHERE active = $1",
        params = ?vec!["true"],
        rows_returned = 42,
        execution_time_ms = 12.34,
        connection_pool_size = 10,
        "Database query executed"
    );
    
    // Business logic example
    warn!(
        user_id = 67890,
        account_balance = 15.50,
        attempted_withdrawal = 100.00,
        transaction_id = "tx_xyz789",
        reason = "insufficient_funds",
        "Transaction failed due to insufficient funds"
    );
    
    println!("ℹ️  Notice how each log entry includes structured fields for better analysis\n");
    
    Ok(())
}

async fn demo_span_tracking() -> Result<(), Box<dyn std::error::Error>> {
    println!("🕸️  Demo 3: Span-based Context Tracking");
    println!("---------------------------------------");
    
    // Simulate processing multiple user requests
    for user_id in [1001, 1002, 1003] {
        process_user_request(user_id).await?;
    }
    
    println!("ℹ️  Each request is tracked in its own span for better context\n");
    
    Ok(())
}

async fn process_user_request(user_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    // Create a span for this entire request
    let request_span = info_span!("user_request", user_id = user_id);
    
    async move {
        info!("Starting user request processing");
        
        // Simulate authentication
        authenticate_user(user_id).await?;
        
        // Simulate data fetching
        fetch_user_data(user_id).await?;
        
        // Simulate response generation
        generate_response(user_id).await?;
        
        info!("User request processing completed");
        
        Ok(())
    }
    .instrument(request_span)
    .await
}

async fn authenticate_user(user_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let auth_span = debug_span!("authentication", user_id = user_id);
    let _enter = auth_span.enter();
    
    debug!("Validating user credentials");
    tokio::time::sleep(Duration::from_millis(10)).await; // Simulate work
    
    info!(
        user_id = user_id,
        auth_method = "jwt",
        token_expiry = "2024-12-31T23:59:59Z",
        "User authenticated successfully"
    );
    
    Ok(())
}

async fn fetch_user_data(user_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let data_span = debug_span!("data_fetch", user_id = user_id);
    let _enter = data_span.enter();
    
    debug!("Querying user database");
    tokio::time::sleep(Duration::from_millis(20)).await; // Simulate database query
    
    info!(
        user_id = user_id,
        tables_queried = 3,
        records_found = 1,
        cache_hit = false,
        query_time_ms = 18.5,
        "User data retrieved from database"
    );
    
    Ok(())
}

async fn generate_response(user_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let response_span = debug_span!("response_generation", user_id = user_id);
    let _enter = response_span.enter();
    
    debug!("Serializing response data");
    tokio::time::sleep(Duration::from_millis(5)).await; // Simulate serialization
    
    info!(
        user_id = user_id,
        response_size_bytes = 2048,
        format = "json",
        compression = "gzip",
        "Response generated and ready to send"
    );
    
    Ok(())
}

async fn demo_external_crate_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 Demo 4: Integration with External Crates (reqwest)");
    println!("----------------------------------------------------");
    
    // Create a custom configuration that shows reqwest's internal tracing
    let config = LogConfig {
        format: LogFormat::Pretty,
        with_target: true,  // Show which crate/module is logging
        with_timestamp: true,
        with_line_number: false,
        with_thread_names: false,
        env_filter: Some("quillai_log=debug,reqwest=debug,hyper=info".to_string()),
    };
    
    // Note: This will fail to initialize if subscriber is already set, which is fine
    let _ = init_logger(LogLevel::Debug, &config);
    
    info!("Making HTTP request to demonstrate external crate integration");
    
    // Make an HTTP request - this will generate tracing events from reqwest/hyper
    let http_span = info_span!("http_request", url = "https://jsonplaceholder.typicode.com/users");
    
    let result = async {
        debug!("Creating HTTP client");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        
        info!(
            method = "GET",
            url = "https://jsonplaceholder.typicode.com/users",
            timeout_secs = 10,
            "Sending HTTP request"
        );
        
        let response = client
            .get("https://jsonplaceholder.typicode.com/users")
            .send()
            .await?;
        
        let status = response.status();
        let content_length = response.content_length();
        
        info!(
            status_code = status.as_u16(),
            status_text = status.as_str(),
            content_length = ?content_length,
            "HTTP response received"
        );
        
        if status.is_success() {
            let users: Vec<User> = response.json().await?;
            
            info!(
                users_count = users.len(),
                first_user = ?users.first().map(|u| &u.name),
                "Successfully parsed user data"
            );
            
            // Log details about the first few users
            for (index, user) in users.iter().take(3).enumerate() {
                debug!(
                    index = index,
                    user_id = user.id,
                    name = %user.name,
                    email = %user.email,
                    "User data details"
                );
            }
        } else {
            warn!(
                status_code = status.as_u16(),
                "HTTP request failed"
            );
        }
        
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .instrument(http_span)
    .await;
    
    match result {
        Ok(()) => info!("HTTP request demo completed successfully"),
        Err(e) => error!(error = %e, "HTTP request demo failed"),
    }
    
    println!("ℹ️  Notice the tracing events from reqwest/hyper libraries mixed with our logs\n");
    
    Ok(())
}

async fn demo_output_formats() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Demo 5: Different Output Formats");
    println!("-----------------------------------");
    
    println!("Current format: Pretty (colored, human-readable)");
    info!(
        demo = "pretty_format",
        timestamp = "2024-01-15T10:30:45Z",
        level = "INFO",
        message = "This is how pretty format looks"
    );
    
    // Note: To see other formats, you'd need to restart with different configurations
    println!("ℹ️  To see other formats, run:");
    println!("   - JSON format: Enable 'json' feature and use LogFormat::Json");
    println!("   - Compact format: Use LogFormat::Compact for denser output");
    println!("   - Custom filters: Use env_filter for fine-grained control\n");
    
    Ok(())
}

async fn demo_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚨 Demo 6: Error Handling and Debugging");
    println!("---------------------------------------");
    
    // Simulate various error scenarios
    let error_span = create_span(Level::ERROR, "error_handling");
    let _enter = error_span.enter();
    
    // Application error
    error!(
        error_type = "validation_error",
        field = "email",
        value = "invalid-email",
        user_input = "not-an-email-address",
        "Input validation failed"
    );
    
    // Database error simulation
    error!(
        error_type = "database_error",
        operation = "INSERT",
        table = "users",
        constraint_violated = "unique_email",
        retry_count = 3,
        "Database operation failed after retries"
    );
    
    // External service error
    warn!(
        service = "payment_gateway",
        endpoint = "/api/v1/charge",
        status_code = 503,
        retry_after_seconds = 30,
        transaction_id = "tx_failed_123",
        "External service temporarily unavailable"
    );
    
    // Performance warning
    warn!(
        operation = "data_processing",
        duration_ms = 5420,
        threshold_ms = 5000,
        records_processed = 10000,
        "Operation exceeded performance threshold"
    );
    
    // Debug information for troubleshooting
    debug!(
        thread_id = ?std::thread::current().id(),
        memory_usage_mb = 245,
        cpu_usage_percent = 78.5,
        active_connections = 42,
        queue_size = 156,
        "System performance metrics"
    );
    
    println!("ℹ️  Error logs include structured data for better debugging and monitoring\n");
    
    Ok(())
}