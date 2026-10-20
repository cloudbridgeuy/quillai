use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use clap::Parser;
use tower::ServiceExt;

#[path = "../src/cli.rs"]
mod cli;
mod common;

async fn assert_preflight(app: Router, origin: &str, allowed: bool) {
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/series")
                .header(header::ORIGIN, origin)
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .map(|value| value.to_str().unwrap()),
        allowed.then_some(origin)
    );
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_METHODS],
        "GET,POST,PUT,DELETE,OPTIONS"
    );
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_HEADERS],
        "content-type"
    );
}

#[tokio::test]
async fn default_cors_origins_are_unchanged() {
    let args = cli::App::try_parse_from(["api"]).unwrap();
    assert!(args.cors_origin.is_empty());
    let app = quillai_api::app_with_cors(common::test_pool().await, args.cors_origin);
    for origin in ["http://localhost:3001", "http://127.0.0.1:3001"] {
        assert_preflight(app.clone(), origin, true).await;
    }
    for origin in ["http://127.0.0.1:3107", "https://example.com", "null"] {
        assert_preflight(app.clone(), origin, false).await;
    }
}

#[tokio::test]
async fn repeated_cors_flags_replace_default_origins() {
    let args = cli::App::try_parse_from([
        "api",
        "--cors-origin",
        "http://127.0.0.1:3107",
        "--cors-origin",
        "http://localhost:3107",
    ])
    .unwrap();
    assert_eq!(args.cors_origin.len(), 2);
    let app = quillai_api::app_with_cors(common::test_pool().await, args.cors_origin);
    for origin in ["http://127.0.0.1:3107", "http://localhost:3107"] {
        assert_preflight(app.clone(), origin, true).await;
    }
    for origin in [
        "http://127.0.0.1:3001",
        "http://localhost:3001",
        "https://example.com",
    ] {
        assert_preflight(app.clone(), origin, false).await;
    }
}

#[test]
fn invalid_cors_header_is_rejected_by_cli() {
    assert!(cli::App::try_parse_from(["api", "--cors-origin", "bad\norigin"]).is_err());
}
