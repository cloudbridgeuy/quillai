use std::{
    io,
    sync::{Arc, Mutex},
};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use quillai_log::tracing_subscriber::{self, util::SubscriberInitExt};
use tower::ServiceExt;

mod common;

#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn missing_item_logs_one_warning_without_request_data() {
    let pool = common::test_pool().await;
    let output = Arc::new(Mutex::new(Vec::new()));
    let writer = Capture(output.clone());
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(quillai_log::Level::DEBUG)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = subscriber.set_default();
    let response = quillai_api::app(pool)
        .oneshot(
            Request::builder()
                .uri("/series/999?secret=query-secret")
                .header("x-secret", "header-secret")
                .body(Body::from("body-secret"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let log = String::from_utf8(output.lock().unwrap().clone()).unwrap();
    let lines: Vec<_> = log
        .lines()
        .filter(|line| line.starts_with(" WARN "))
        .collect();
    assert_eq!(lines.len(), 1, "{log}");
    let line = lines[0];
    for expected in [
        "request completed",
        "method=GET",
        "path=/series/999",
        "status=404",
        "elapsed_ms=",
    ] {
        assert!(line.contains(expected), "missing {expected}: {line}");
    }
    for secret in ["query-secret", "header-secret", "body-secret"] {
        assert!(!log.contains(secret), "leaked {secret}: {log}");
    }
}
