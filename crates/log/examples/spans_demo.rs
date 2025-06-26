//! Comprehensive spans demonstration for QuillAI Log
//!
//! This example focuses specifically on spans - what they are, how they work,
//! and why they're powerful for distributed tracing and debugging.
//!
//! Spans represent units of work and provide context to all logging that
//! happens within them. They can be nested, carry fields, and track timing.
//!
//! Run with:
//! ```bash
//! RUST_LOG=debug cargo run --example spans_demo
//! ```

use quillai_log::{
    create_span, debug, debug_span, error, info, info_span, init_logger, warn, warn_span,
    Instrument, Level, LogConfig, LogLevel, Span,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize with debug level to see all span entries/exits
    let config = LogConfig {
        with_target: true,      // Show the module/target
        with_line_number: true, // Show line numbers
        ..Default::default()
    };
    init_logger(LogLevel::Debug, &config)?;

    println!("🕸️  QuillAI Log - Spans Demonstration");
    println!("====================================\n");

    // Demo 1: Basic span creation and usage
    demo_basic_spans().await?;

    // Demo 2: Nested spans showing hierarchy
    demo_nested_spans().await?;

    // Demo 3: Spans with fields and context
    demo_spans_with_fields().await?;

    // Demo 4: Async operations with spans
    demo_async_spans().await?;

    // Demo 5: Span timing and performance tracking
    demo_span_timing().await?;

    // Demo 6: Error handling within spans
    demo_error_spans().await?;

    // Demo 7: Advanced span patterns
    demo_advanced_span_patterns().await?;

    println!("\n✅ All span demonstrations completed!");
    Ok(())
}

async fn demo_basic_spans() -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 Demo 1: Basic Span Creation and Usage");
    println!("----------------------------------------");

    // Method 1: Create a span and enter it manually
    let span = info_span!("manual_span");
    let _guard = span.enter();

    info!("This log message appears within the 'manual_span'");
    debug!("Debug messages also inherit the span context");

    // The span ends when _guard is dropped
    drop(_guard);

    info!("This message is outside the span");

    // Method 2: Using span with a block
    {
        let span = debug_span!("block_span", operation = "demo");
        let _enter = span.enter();

        info!("Inside block_span with operation field");
        warn!("Warnings also show span context");
    } // span automatically ends here

    // Method 3: Using .instrument() for async operations
    async {
        info!("This is inside an instrumented async block");
        debug!("All logs in this block share the same span");
    }
    .instrument(info_span!("async_block"))
    .await;

    println!("ℹ️  Notice how logs show span names like [manual_span] and [block_span]\n");

    Ok(())
}

async fn demo_nested_spans() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  Demo 2: Nested Spans Showing Hierarchy");
    println!("------------------------------------------");

    // Simulate processing an order with nested operations
    let order_span = info_span!("process_order", order_id = "ORD-123", customer_id = 456);

    async {
        info!("Starting order processing");

        // Nested span for payment processing
        let payment_span = debug_span!("payment_processing", amount = 99.99, currency = "USD");
        async {
            debug!("Validating payment method");
            sleep(Duration::from_millis(50)).await;

            // Even deeper nesting for external API call
            let api_span = debug_span!("payment_api_call", provider = "stripe");
            async {
                debug!("Calling external payment API");
                sleep(Duration::from_millis(100)).await;
                info!("Payment authorized successfully");
            }
            .instrument(api_span)
            .await;

            info!("Payment processing completed");
        }
        .instrument(payment_span)
        .await;

        // Another nested span for inventory
        let inventory_span = debug_span!("inventory_check", product_id = "PROD-789");
        async {
            debug!("Checking inventory levels");
            sleep(Duration::from_millis(30)).await;
            info!("Inventory available, reserving items");
        }
        .instrument(inventory_span)
        .await;

        // Final nested span for fulfillment
        let fulfillment_span = debug_span!("fulfillment", warehouse = "US-WEST");
        async {
            debug!("Creating fulfillment order");
            sleep(Duration::from_millis(40)).await;
            info!("Order queued for shipping");
        }
        .instrument(fulfillment_span)
        .await;

        info!("Order processing completed successfully");
    }
    .instrument(order_span)
    .await;

    println!("ℹ️  Notice the nested structure: [process_order]:[payment_processing]:[payment_api_call]\n");

    Ok(())
}

async fn demo_spans_with_fields() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏷️  Demo 3: Spans with Fields and Context");
    println!("----------------------------------------");

    // Spans can carry structured data that applies to all contained logs
    let user_session = info_span!(
        "user_session",
        user_id = 12345,
        session_id = "sess_abc123",
        ip_address = "192.168.1.100",
        user_agent = "Mozilla/5.0"
    );

    async {
        info!("User session started");

        // Each operation within the session inherits the user context
        let page_view = debug_span!("page_view", page = "/dashboard", load_time_ms = 245);
        async {
            debug!("Page view recorded");
            info!("User interaction tracked");
        }
        .instrument(page_view)
        .await;

        let api_call = debug_span!("api_call", endpoint = "/api/v1/profile", method = "GET");
        async {
            debug!("Making API call for user profile");
            info!("Profile data retrieved successfully");
        }
        .instrument(api_call)
        .await;

        // You can also add fields to existing spans
        let current_span = Span::current();
        current_span.record("total_requests", &3);
        current_span.record("session_duration_sec", &450);

        info!("Session activity completed");
    }
    .instrument(user_session)
    .await;

    println!("ℹ️  All logs within user_session include user_id, session_id, etc.\n");

    Ok(())
}

async fn demo_async_spans() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Demo 4: Async Operations with Spans");
    println!("-------------------------------------");

    // Simulate concurrent operations, each with their own span
    let task1 = process_async_task("task_1", "high", 80).instrument(info_span!("worker_1"));
    let task2 = process_async_task("task_2", "medium", 120).instrument(info_span!("worker_2"));
    let task3 = process_async_task("task_3", "low", 60).instrument(info_span!("worker_3"));

    // Run all tasks concurrently
    let batch_span = info_span!("batch_processing", batch_id = "BATCH-456", task_count = 3);
    async {
        info!("Starting batch processing");

        let _ = tokio::try_join!(task1, task2, task3)?;

        info!(
            completed_tasks = 3,
            "Batch processing completed successfully"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .instrument(batch_span)
    .await?;

    println!("ℹ️  Each async task maintains its own span context even when running concurrently\n");

    Ok(())
}

async fn process_async_task(
    task_id: &str,
    priority: &str,
    duration_ms: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let task_span = debug_span!("async_task", task_id = task_id, priority = priority);

    async move {
        debug!("Task started");

        // Simulate some work
        sleep(Duration::from_millis(duration_ms)).await;

        // Simulate different outcomes based on task
        match task_id {
            "task_2" => {
                warn!("Task encountered a minor issue but recovered");
            }
            _ => {
                info!("Task completed successfully");
            }
        }

        Ok(format!("Result from {}", task_id))
    }
    .instrument(task_span)
    .await
}

async fn demo_span_timing() -> Result<(), Box<dyn std::error::Error>> {
    println!("⏱️  Demo 5: Span Timing and Performance Tracking");
    println!("-----------------------------------------------");

    // Spans automatically track timing when they start and end
    let database_operation =
        info_span!("database_operation", table = "users", operation = "SELECT");

    async {
        info!("Starting database query");

        // Simulate slow database operation
        sleep(Duration::from_millis(150)).await;

        info!(rows_returned = 42, "Query completed");

        // Add performance metrics to the span
        Span::current().record("execution_time_ms", &150);
        Span::current().record("rows_scanned", &1000);
        Span::current().record("index_used", &true);
    }
    .instrument(database_operation)
    .await;

    // Another example with performance monitoring
    let cache_operation = debug_span!("cache_lookup", key = "user:12345", cache_type = "redis");

    async {
        debug!("Checking cache");
        sleep(Duration::from_millis(5)).await;

        info!(cache_hit = true, ttl_seconds = 3600, "Cache hit");

        Span::current().record("response_time_ms", &5);
    }
    .instrument(cache_operation)
    .await;

    println!("ℹ️  Spans can track timing and performance metrics for monitoring\n");

    Ok(())
}

async fn demo_error_spans() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚨 Demo 6: Error Handling within Spans");
    println!("--------------------------------------");

    // Demonstrate how spans help with error tracking and debugging
    let error_demo_span = warn_span!("error_handling_demo", component = "payment_service");

    async {
        info!("Starting error handling demonstration");

        // Simulate a failed operation
        let payment_span = info_span!(
            "payment_attempt",
            transaction_id = "tx_789",
            amount = 150.00
        );
        let result: Result<(), &str> = async {
            debug!("Processing payment");
            sleep(Duration::from_millis(100)).await;

            // Simulate payment failure
            error!(
                error_code = "INSUFFICIENT_FUNDS",
                account_balance = 50.00,
                attempted_amount = 150.00,
                "Payment failed due to insufficient funds"
            );

            Span::current().record("payment_status", &"failed");
            Span::current().record("failure_reason", &"insufficient_funds");

            Err("Payment failed")
        }
        .instrument(payment_span)
        .await;

        // Handle the error with context from the span
        match result {
            Ok(_) => info!("Payment processed successfully"),
            Err(e) => {
                error!(error = e, "Payment processing failed, initiating recovery");

                // Simulate retry logic
                let retry_span = debug_span!("payment_retry", attempt = 2);
                async {
                    debug!("Attempting payment retry with different method");
                    sleep(Duration::from_millis(50)).await;
                    info!("Retry payment successful");
                }
                .instrument(retry_span)
                .await;
            }
        }

        info!("Error handling demonstration completed");
    }
    .instrument(error_demo_span)
    .await;

    println!("ℹ️  Spans provide context for errors, making debugging much easier\n");

    Ok(())
}

async fn demo_advanced_span_patterns() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Demo 7: Advanced Span Patterns");
    println!("----------------------------------");

    // Pattern 1: Conditional spans
    let enable_detailed_tracing = true;

    if enable_detailed_tracing {
        let detailed_span = create_span(Level::DEBUG, "detailed_processing");
        let _enter = detailed_span.enter();

        debug!("Detailed tracing enabled");
        info!("Processing with full instrumentation");
    } else {
        info!("Processing with minimal instrumentation");
    }

    // Pattern 2: Span inheritance across function boundaries
    await_with_inherited_span().await?;

    // Pattern 3: Custom span with dynamic fields
    let request_id = generate_request_id();
    let dynamic_span = info_span!(
        "dynamic_request",
        request_id = %request_id,
        timestamp = %chrono::Utc::now(),
        source = "api"
    );

    async {
        info!("Processing dynamic request");

        // Add more fields as we learn more about the request
        Span::current().record("user_type", &"premium");
        Span::current().record("processing_time_ms", &75);

        debug!("Request processing completed");
    }
    .instrument(dynamic_span)
    .await;

    // Pattern 4: Span events (manual events within spans)
    let event_span = info_span!("event_demo");
    let _enter = event_span.enter();

    info!("Span started");

    // You can also emit events at specific points
    tracing::event!(
        Level::INFO,
        milestone = "checkpoint_1",
        progress_percent = 25,
        "Reached processing checkpoint"
    );

    sleep(Duration::from_millis(25)).await;

    tracing::event!(
        Level::INFO,
        milestone = "checkpoint_2",
        progress_percent = 75,
        "Reached second checkpoint"
    );

    info!("Processing completed");

    println!("ℹ️  Advanced patterns give you fine-grained control over tracing\n");

    Ok(())
}

async fn await_with_inherited_span() -> Result<(), Box<dyn std::error::Error>> {
    // This function will inherit the span context from its caller
    info!("This log inherits the span context from the caller");

    let nested_span = debug_span!("nested_function");
    async {
        debug!("Inside nested function span");
        sleep(Duration::from_millis(10)).await;
        info!("Nested operation completed");
    }
    .instrument(nested_span)
    .await;

    Ok(())
}

fn generate_request_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .hash(&mut hasher);
    format!("req_{:x}", hasher.finish())
}

