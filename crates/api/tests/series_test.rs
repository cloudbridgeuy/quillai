use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

use common::test_pool;

async fn request(app: &Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

async fn create_series(app: &Router, title: &str, position: i64) -> Value {
    let (status, body) = request(
        app,
        "POST",
        "/series",
        json!({"title": title, "position": position}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["id"].as_i64().unwrap() > 0);
    assert_eq!(body["title"], title);
    assert_eq!(body["position"], position);
    body
}

#[tokio::test]
async fn series_crud_and_partial_updates() {
    let app = quillai_api::app(test_pool().await);
    assert_eq!(
        request(&app, "GET", "/series", Value::Null).await,
        (StatusCode::OK, json!([]))
    );
    let mut series = create_series(&app, "Original", 0).await;
    let uri = format!("/series/{}", series["id"]);
    assert_eq!(
        request(&app, "GET", &uri, Value::Null).await,
        (StatusCode::OK, series.clone())
    );

    for update in [
        json!({"title": "Renamed"}),
        json!({"position": 3}),
        json!({"title": "Final", "position": 1}),
        json!({}),
        json!({"title": null, "position": null}),
    ] {
        for field in ["title", "position"] {
            if !update[field].is_null() {
                series[field] = update[field].clone();
            }
        }
        assert_eq!(
            request(&app, "PUT", &uri, update).await,
            (StatusCode::OK, series.clone())
        );
        assert_eq!(
            request(&app, "GET", &uri, Value::Null).await,
            (StatusCode::OK, series.clone())
        );
    }

    assert_eq!(
        request(&app, "DELETE", &uri, Value::Null).await,
        (StatusCode::NO_CONTENT, Value::Null)
    );
    assert_eq!(
        request(&app, "GET", &uri, Value::Null).await,
        (
            StatusCode::NOT_FOUND,
            json!({"error": "not found", "code": "NOT_FOUND"})
        )
    );
    assert_eq!(
        request(&app, "GET", "/series", Value::Null).await,
        (StatusCode::OK, json!([]))
    );
}

#[tokio::test]
async fn missing_series_returns_json_404() {
    let app = quillai_api::app(test_pool().await);
    for method in ["GET", "PUT", "DELETE"] {
        assert_eq!(
            request(&app, method, "/series/999", json!({})).await,
            (
                StatusCode::NOT_FOUND,
                json!({"error": "not found", "code": "NOT_FOUND"})
            )
        );
    }
}

#[tokio::test]
async fn invalid_series_returns_422_without_mutation() {
    let app = quillai_api::app(test_pool().await);
    let series = create_series(&app, "Unchanged", 0).await;
    let uri = format!("/series/{}", series["id"]);
    for input in [
        json!({"title": "", "position": 0}),
        json!({"title": " \t\n", "position": 0}),
        json!({"title": "Valid", "position": -1}),
    ] {
        for (method, path) in [("POST", "/series"), ("PUT", uri.as_str())] {
            let (status, body) = request(&app, method, path, input.clone()).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], "VALIDATION_ERROR");
            assert!(body["error"]
                .as_str()
                .unwrap()
                .starts_with("validation error:"));
        }
    }
    assert_eq!(
        request(&app, "GET", "/series", Value::Null).await,
        (StatusCode::OK, json!([series]))
    );
}

#[tokio::test]
async fn series_order_by_position_then_id_after_update() {
    let app = quillai_api::app(test_pool().await);
    let late = create_series(&app, "Late", 9).await;
    let first = create_series(&app, "First", 1).await;
    let tied = create_series(&app, "Tied", 1).await;
    assert_eq!(
        request(&app, "GET", "/series", Value::Null).await,
        (StatusCode::OK, json!([first, tied, late]))
    );
    let (status, moved) = request(
        &app,
        "PUT",
        &format!("/series/{}", late["id"]),
        json!({"position": 0}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        request(&app, "GET", "/series", Value::Null).await,
        (StatusCode::OK, json!([moved, first, tied]))
    );
}
