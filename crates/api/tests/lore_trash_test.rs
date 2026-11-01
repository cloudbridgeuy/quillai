use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

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
    (
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        },
    )
}

#[tokio::test]
async fn lore_trash_is_scoped_and_shared_across_route_families() {
    let app = quillai_api::app(common::test_pool().await);
    let (_, series) = request(
        &app,
        "POST",
        "/series",
        json!({"title": "Series", "position": 0}),
    )
    .await;
    let (_, other) = request(
        &app,
        "POST",
        "/universes",
        json!({"title": "Other", "position": 0}),
    )
    .await;
    let collection = format!("/series/{}/lore", series["id"]);
    let (_, entry) = request(
        &app,
        "POST",
        &collection,
        json!({"kind": "note", "name": "Ada", "visible_text": "Public"}),
    )
    .await;
    let uri = format!("{collection}/{}", entry["id"]);
    let (_, universes) = request(&app, "GET", "/universes", Value::Null).await;
    let universe = universes
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] != other["id"])
        .unwrap();
    let universe_uri = format!("/universes/{}/lore/{}", universe["id"], entry["id"]);
    let wrong = format!("/universes/{}/lore/{}", other["id"], entry["id"]);
    assert_eq!(
        request(&app, "DELETE", &uri, Value::Null).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await.1,
        json!([])
    );
    for path in [&uri, &universe_uri] {
        for method in ["GET", "PUT", "DELETE"] {
            assert_eq!(
                request(&app, method, path, json!({"name": "Changed"}))
                    .await
                    .0,
                StatusCode::NOT_FOUND
            );
        }
    }
    assert_eq!(
        request(&app, "POST", &format!("{wrong}/restore"), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &app,
            "DELETE",
            &format!("{wrong}?permanent=true"),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (status, restored) = request(
        &app,
        "POST",
        &format!("{universe_uri}/restore"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(restored["visible_text"], "Public");
    assert_eq!(request(&app, "GET", &uri, Value::Null).await.1, entry);
    assert_eq!(
        request(&app, "POST", &format!("{uri}/restore"), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(&app, "DELETE", &universe_uri, Value::Null).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "DELETE",
            &format!("{uri}?permanent=true"),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "POST", &format!("{uri}/restore"), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("{universe_uri}/restore"),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}
