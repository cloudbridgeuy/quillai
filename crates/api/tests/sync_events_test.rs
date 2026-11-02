use std::time::Duration;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

async fn call(app: &Router, method: &str, uri: &str, body: Value) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn created(app: &Router, uri: &str, body: Value) -> i64 {
    let response = call(app, "POST", uri, body).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice::<Value>(&body).unwrap()["id"]
        .as_i64()
        .unwrap()
}

async fn frame(body: &mut Body) -> String {
    let bytes = tokio::time::timeout(Duration::from_secs(2), body.frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_data()
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn assert_changed(stream: &mut Body, id: i64, rev: i64) {
    let event = frame(stream).await;
    assert!(event.lines().any(|line| line == "event: changed"));
    let data = event
        .lines()
        .find_map(|line| line.strip_prefix("data:"))
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(data).unwrap(),
        json!({"pieceId":id,"rev":rev})
    );
}

async fn piece(app: &Router, universe: i64, shape: &str, label: &str, position: i64) -> i64 {
    created(
        app,
        &format!("/universes/{universe}/pieces"),
        json!({"title":"test", "position":position, "shape":shape, "label":label}),
    )
    .await
}

#[tokio::test]
async fn commits_filter_and_history() {
    let app = quillai_api::app(common::test_pool().await);
    let universe = created(&app, "/universes", json!({"title":"World", "position":0})).await;
    let a = piece(&app, universe, "leaf", "piece", 0).await;
    let b = piece(&app, universe, "leaf", "piece", 1).await;
    let response = call(
        &app,
        "GET",
        &format!("/sync/events?pieceIds={a}"),
        Value::Null,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    assert_eq!(response.headers()["cache-control"], "no-cache");
    assert_eq!(response.headers()["x-accel-buffering"], "no");
    let mut stream = response.into_body();
    assert!(frame(&mut stream)
        .await
        .contains(&format!("\"pieceIds\":[{a}]")));
    let save = |id, base| {
        (
            format!("/pieces/{id}/deltas"),
            json!({"baseRev":base,"delta":{"ops":[{"insert":"x"}]}}),
        )
    };
    let mut multiple = call(
        &app,
        "GET",
        &format!("/sync/events?pieceIds={a},{b}"),
        Value::Null,
    )
    .await
    .into_body();
    let (uri, body) = save(b, 0);
    assert_eq!(
        call(&app, "POST", &uri, body).await.status(),
        StatusCode::OK
    );
    let ready = frame(&mut multiple).await;
    assert!(ready.contains("event: ready"));
    assert!(ready.contains(&format!("\"pieceIds\":[{a},{b}]")));
    assert!(ready.lines().any(|line| line
        .strip_prefix("retry:")
        .is_some_and(|ms| ms.trim() == "1000")));
    assert!(!ready.contains("\nid:"));
    assert!(frame(&mut multiple)
        .await
        .contains(&format!("\"pieceId\":{b},\"rev\":1")));
    drop(multiple);
    let (uri, body) = save(a, 999);
    assert_eq!(
        call(&app, "POST", &uri, body).await.status(),
        StatusCode::CONFLICT
    );
    let (uri, body) = save(a, 0);
    assert_eq!(
        call(&app, "POST", &uri, body).await.status(),
        StatusCode::OK
    );
    assert_changed(&mut stream, a, 1).await;
    assert_eq!(
        call(&app, "POST", &uri, json!({"baseRev":"bad"}))
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (uri, body) = save(a, 1);
    assert_eq!(
        call(&app, "POST", &uri, body).await.status(),
        StatusCode::OK
    );
    assert_changed(&mut stream, a, 2).await;
    let (uri, body) = save(b, 1);
    assert_eq!(
        call(&app, "POST", &uri, body).await.status(),
        StatusCode::OK
    );
    let (uri, body) = save(a, 2);
    assert_eq!(
        call(&app, "POST", &uri, body).await.status(),
        StatusCode::OK
    );
    assert_changed(&mut stream, a, 3).await;
    let response = call(
        &app,
        "GET",
        &format!("/pieces/{a}/deltas?since=0"),
        Value::Null,
    )
    .await;
    let data = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(serde_json::from_slice::<Value>(&data).unwrap()[0]["rev"], 1);
    drop(stream);
    let (uri, body) = save(a, 3);
    assert_eq!(
        call(&app, "POST", &uri, body).await.status(),
        StatusCode::OK
    );
    let mut reconnected = call(
        &app,
        "GET",
        &format!("/sync/events?pieceIds={a}"),
        Value::Null,
    )
    .await
    .into_body();
    let (uri, body) = save(a, 4);
    assert_eq!(
        call(&app, "POST", &uri, body).await.status(),
        StatusCode::OK
    );
    assert!(frame(&mut reconnected).await.contains("event: ready"));
    assert_changed(&mut reconnected, a, 5).await;
    let response = call(
        &app,
        "GET",
        &format!("/pieces/{a}/deltas?since=3"),
        Value::Null,
    )
    .await;
    let data = response.into_body().collect().await.unwrap().to_bytes();
    let revisions: Value = serde_json::from_slice(&data).unwrap();
    assert_eq!(revisions[0]["rev"], 4);
    assert_eq!(revisions[1]["rev"], 5);
}

#[tokio::test]
async fn validation_and_legacy_stream() {
    let pool = common::test_pool().await;
    let app = quillai_api::app(pool.clone());
    let universe = created(&app, "/universes", json!({"title":"World", "position":0})).await;
    let leaf = piece(&app, universe, "leaf", "piece", 0).await;
    let group = piece(&app, universe, "group", "screenplay", 1).await;
    let long = "1".repeat(4097);
    let many = (1..=257)
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    for query in [
        "".to_string(),
        "pieceIds=".into(),
        "pieceIds=1,1".into(),
        "pieceIds=1,,2".into(),
        "pieceIds=x".into(),
        "pieceIds=%2B1".into(),
        "pieceIds=%201".into(),
        "pieceIds=9223372036854775808".into(),
        format!("pieceIds={long}"),
        format!("pieceIds={many}"),
    ] {
        assert_eq!(
            call(&app, "GET", &format!("/sync/events?{query}"), Value::Null)
                .await
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    for id in [group, 999999] {
        assert_eq!(
            call(
                &app,
                "GET",
                &format!("/sync/events?pieceIds={id}"),
                Value::Null
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        call(&app, "DELETE", &format!("/pieces/{leaf}"), Value::Null)
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/sync/events?pieceIds={leaf}"),
            Value::Null
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    let series = created(&app, "/series", json!({"title":"Series", "position":0})).await;
    let book = created(
        &app,
        &format!("/series/{series}/books"),
        json!({"title":"Book", "position":0}),
    )
    .await;
    let chapter = created(
        &app,
        &format!("/series/{series}/books/{book}/chapters"),
        json!({"title":"Chapter", "position":0, "kind":"chapter"}),
    )
    .await;
    let path = format!("/series/{series}/books/{book}/chapters/{chapter}");
    let piece_id: i64 = sqlx::query_scalar("SELECT piece_id FROM legacy_chapters WHERE id = ?")
        .bind(chapter)
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut stream = call(&app, "GET", &format!("{path}/events"), Value::Null)
        .await
        .into_body();
    assert!(frame(&mut stream)
        .await
        .contains(&format!("\"pieceIds\":[{piece_id}]")));
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/series/{series}/books/999999/chapters/{chapter}/events"),
            Value::Null
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("{path}/deltas"),
            json!({"baseRev":0,"delta":{"ops":[{"insert":"x"}]}})
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert!(frame(&mut stream)
        .await
        .contains(&format!("\"pieceId\":{piece_id},\"rev\":1")));
    assert_eq!(
        call(&app, "DELETE", &format!("/pieces/{piece_id}"), Value::Null)
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&app, "GET", &format!("{path}/events"), Value::Null)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn lag_requests_resync_and_continues() {
    let app = quillai_api::app(common::test_pool().await);
    let universe = created(&app, "/universes", json!({"title":"World", "position":0})).await;
    let id = piece(&app, universe, "leaf", "piece", 0).await;
    let mut stream = call(
        &app,
        "GET",
        &format!("/sync/events?pieceIds={id}"),
        Value::Null,
    )
    .await
    .into_body();
    frame(&mut stream).await;
    for rev in 0..257 {
        assert_eq!(
            call(
                &app,
                "POST",
                &format!("/pieces/{id}/deltas"),
                json!({"baseRev":rev,"delta":{"ops":[]}})
            )
            .await
            .status(),
            StatusCode::OK
        );
    }
    assert!(frame(&mut stream).await.contains("event: resync"));
    assert_changed(&mut stream, id, 2).await;
}

#[tokio::test]
async fn query_boundaries_accept_active_ids() {
    let app = quillai_api::app(common::test_pool().await);
    let universe = created(&app, "/universes", json!({"title":"World", "position":0})).await;
    let mut ids = Vec::new();
    for position in 0..256 {
        ids.push(piece(&app, universe, "leaf", "piece", position).await);
    }
    let query = format!(
        "pieceIds={}",
        ids.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    let response = call(&app, "GET", &format!("/sync/events?{query}"), Value::Null).await;
    assert_eq!(response.status(), StatusCode::OK);
    let ready = frame(&mut response.into_body()).await;
    assert!(ready.lines().any(|line| line == "event: ready"));
    let data = ready
        .lines()
        .find_map(|line| line.strip_prefix("data:"))
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(data).unwrap(),
        json!({"pieceIds":ids})
    );
    let id = ids[0].to_string();
    let query = format!("pieceIds={:0>width$}", id, width = 4096 - "pieceIds=".len());
    assert_eq!(query.len(), 4096);
    assert_eq!(
        call(&app, "GET", &format!("/sync/events?{query}"), Value::Null)
            .await
            .status(),
        StatusCode::OK
    );
    let query = format!("pieceIds=0{}", &query["pieceIds=".len()..]);
    assert_eq!(query.len(), 4097);
    assert_eq!(
        call(&app, "GET", &format!("/sync/events?{query}"), Value::Null)
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn events_cors_allows_default_origin() {
    let app = quillai_api::app(common::test_pool().await);
    let universe = created(&app, "/universes", json!({"title":"World", "position":0})).await;
    let id = piece(&app, universe, "leaf", "piece", 0).await;
    for (origin, allowed) in [
        ("http://localhost:3001", true),
        ("https://example.com", false),
    ] {
        for method in ["OPTIONS", "GET"] {
            let request = Request::builder()
                .method(method)
                .uri(format!("/sync/events?pieceIds={id}"))
                .header(header::ORIGIN, origin);
            let request = if method == "OPTIONS" {
                request.header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            } else {
                request
            };
            let response = app
                .clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response
                    .headers()
                    .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                    .map(|v| v.to_str().unwrap()),
                allowed.then_some(origin)
            );
        }
    }
}
