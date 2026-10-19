use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn guidance_settings_round_trip_empty_and_utf8_limits() {
    let pool = common::test_pool().await;
    let app = quillai_api::app(pool.clone());
    let full = "💡".repeat(65_536 / 4);
    for (method, input, status, expected) in [
        ("GET", Value::Null, StatusCode::OK, json!({"text": ""})),
        (
            "PUT",
            json!({"text": full}),
            StatusCode::OK,
            json!({"text": full}),
        ),
        ("GET", Value::Null, StatusCode::OK, json!({"text": full})),
        (
            "PUT",
            json!({"text": format!("{full}a")}),
            StatusCode::UNPROCESSABLE_ENTITY,
            Value::Null,
        ),
        (
            "PUT",
            json!({"text": "💡".repeat(16_385)}),
            StatusCode::UNPROCESSABLE_ENTITY,
            Value::Null,
        ),
        ("GET", Value::Null, StatusCode::OK, json!({"text": full})),
        (
            "PUT",
            json!({"text": ""}),
            StatusCode::OK,
            json!({"text": ""}),
        ),
        ("GET", Value::Null, StatusCode::OK, json!({"text": ""})),
        ("PUT", json!({}), StatusCode::BAD_REQUEST, Value::Null),
        (
            "PUT",
            json!({"text": null}),
            StatusCode::BAD_REQUEST,
            Value::Null,
        ),
        (
            "PUT",
            json!({"text": 42}),
            StatusCode::BAD_REQUEST,
            Value::Null,
        ),
    ] {
        let request = Request::builder()
            .method(method)
            .uri("/settings/author-guidance")
            .header("content-type", "application/json")
            .body(if method == "GET" {
                Body::empty()
            } else {
                Body::from(input.to_string())
            })
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), status);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        if status == StatusCode::OK {
            assert_eq!(body, expected);
        } else {
            assert_eq!(
                body["code"],
                if status == StatusCode::BAD_REQUEST {
                    "INVALID_JSON"
                } else {
                    "VALIDATION_ERROR"
                }
            );
        }
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM author_guidance")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}
