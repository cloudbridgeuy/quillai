//! JSON output example for QuillAI Log
//!
//! This example demonstrates structured JSON logging output,
//! perfect for log aggregation systems like ELK stack, Grafana, etc.
//!
//! Run with:
//! ```bash
//! cargo run --example json_output --features json
//! ```

use quillai_log::{LogLevel, LogConfig, LogFormat, init_logger, info, debug, warn, error};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📄 QuillAI Log - JSON Output Example");
    println!("====================================");
    println!("JSON logs will appear below:\n");
    
    // Configure JSON output
    let config = LogConfig {
        format: LogFormat::Json,
        with_timestamp: true,
        with_target: true,
        with_thread_names: false,
        with_line_number: true,
        env_filter: None,
    };
    
    // Initialize with JSON formatting
    init_logger(LogLevel::Debug, &config)?;
    
    // Application lifecycle events
    info!(
        event = "app_start",
        version = "2.1.0",
        environment = "production",
        config = ?serde_json::json!({
            "database_pool_size": 10,
            "cache_ttl_seconds": 3600,
            "api_rate_limit": 1000
        }),
        "Application started"
    );
    
    // User session events
    info!(
        event = "user_login",
        user_id = 12345,
        session_id = "sess_abc123def456",
        login_method = "oauth2",
        provider = "google",
        user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        ip_address = "203.0.113.42",
        geolocation = ?serde_json::json!({
            "country": "US",
            "city": "San Francisco",
            "lat": 37.7749,
            "lon": -122.4194
        }),
        "User logged in successfully"
    );
    
    // API request tracking
    debug!(
        event = "api_request",
        request_id = "req_789xyz",
        method = "POST",
        path = "/api/v1/users",
        query_params = ?serde_json::json!({"include": "profile,settings"}),
        request_size_bytes = 1024,
        client_id = "client_web_app",
        "Processing API request"
    );
    
    // Database operations
    debug!(
        event = "database_query",
        query_id = "q_456789",
        operation = "SELECT",
        table = "users",
        query = "SELECT id, name, email FROM users WHERE active = $1 LIMIT $2",
        params = ?vec!["true", "50"],
        execution_time_ms = 23.45,
        rows_returned = 42,
        connection_pool = ?serde_json::json!({
            "active_connections": 7,
            "idle_connections": 3,
            "max_connections": 10
        }),
        "Database query executed"
    );
    
    // Business metrics
    info!(
        event = "business_metric",
        metric_name = "user_registration",
        metric_value = 1,
        user_id = 67890,
        registration_source = "organic",
        plan_type = "premium",
        revenue_impact_usd = 29.99,
        metadata = ?serde_json::json!({
            "campaign_id": "summer_2024",
            "referrer": "https://google.com",
            "utm_source": "google",
            "utm_medium": "cpc",
            "utm_campaign": "brand_keywords"
        }),
        "New user registered"
    );
    
    // Performance monitoring
    warn!(
        event = "performance_alert",
        alert_type = "high_latency",
        service = "user_service",
        endpoint = "/api/v1/users/profile",
        response_time_ms = 2340,
        threshold_ms = 2000,
        p95_latency_ms = 1890,
        p99_latency_ms = 3200,
        request_count_last_5min = 1250,
        error_rate_percent = 0.8,
        "API endpoint latency exceeded threshold"
    );
    
    // Error tracking
    error!(
        event = "application_error",
        error_type = "external_service_error",
        service = "payment_processor",
        error_code = "PAYMENT_DECLINED",
        error_message = "Insufficient funds",
        transaction_id = "txn_error_123",
        user_id = 98765,
        amount_cents = 9999,
        currency = "USD",
        retry_count = 2,
        max_retries = 3,
        stack_trace = "PaymentProcessor.process() line 45\nOrderService.completeOrder() line 123",
        context = ?serde_json::json!({
            "order_id": "order_456789",
            "payment_method": "credit_card",
            "card_last_four": "1234",
            "billing_country": "US"
        }),
        "Payment processing failed"
    );
    
    // System health metrics
    info!(
        event = "system_health",
        timestamp = chrono::Utc::now().to_rfc3339(),
        system = ?serde_json::json!({
            "cpu_usage_percent": 45.2,
            "memory_usage_percent": 67.8,
            "disk_usage_percent": 23.1,
            "load_average": [1.2, 1.5, 1.8]
        }),
        database = ?serde_json::json!({
            "active_connections": 8,
            "slow_queries_count": 2,
            "cache_hit_rate_percent": 94.5
        }),
        application = ?serde_json::json!({
            "active_sessions": 1234,
            "requests_per_second": 567.8,
            "error_rate_percent": 0.02
        }),
        "System health check completed"
    );
    
    info!(
        event = "app_shutdown",
        uptime_seconds = 3661,
        requests_processed = 145789,
        errors_encountered = 23,
        "Application shutting down"
    );
    
    println!("\n✅ JSON logging example completed!");
    println!("💡 These JSON logs can be easily parsed by log aggregation systems");
    
    Ok(())
}