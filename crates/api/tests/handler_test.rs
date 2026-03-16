use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use sqlx::sqlite::SqlitePool;
use tower::ServiceExt;

async fn response_body(res: axum::http::Response<Body>) -> String {
    let bytes = res
        .into_body()
        .collect()
        .await
        .expect("failed to read body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("response body is not valid UTF-8")
}

async fn test_pool() -> SqlitePool {
    SqlitePool::connect("sqlite::memory:")
        .await
        .expect("failed to create in-memory SQLite pool")
}

#[tokio::test]
async fn get_root_returns_hello_world() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool);

    let req = Request::builder()
        .uri("/")
        .body(Body::empty())
        .unwrap();

    let res = app.into_service().oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let body_str = response_body(res).await;
    assert!(
        body_str.contains("hello world"),
        "Expected body to contain 'hello world', got: {body_str}"
    );
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool);

    let req = Request::builder()
        .uri("/nonexistent")
        .body(Body::empty())
        .unwrap();

    let res = app.into_service().oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
