use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use quillai_api::database::MIGRATOR;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

use common::test_pool;

const BEFORE_RULES: i64 = 20261109000000;

async fn send(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn put(app: &Router, series_id: i64, body: Value) -> (StatusCode, Value) {
    send(
        app,
        "PUT",
        &format!("/series/{series_id}/rules"),
        &body.to_string(),
    )
    .await
}

async fn get(app: &Router, series_id: i64) -> (StatusCode, Value) {
    send(app, "GET", &format!("/series/{series_id}/rules"), "").await
}

async fn create_series(app: &Router) -> i64 {
    let body = json!({"title": "Series", "position": 0}).to_string();
    let (status, body) = send(app, "POST", "/series", &body).await;
    assert_eq!(status, StatusCode::CREATED);
    body["id"].as_i64().unwrap()
}

#[tokio::test]
async fn get_returns_stored_defaults_and_series_keep_their_own_config() {
    let app = quillai_api::app(test_pool().await);
    let first = create_series(&app).await;
    let second = create_series(&app).await;
    assert_eq!(
        get(&app, first).await,
        (StatusCode::OK, json!({"language": "en", "rules": {}}))
    );

    let rules = json!({"long-sentence": {"enabled": false, "scope": "narration", "params": {"maxWords": 35}}});
    let saved = json!({"language": "es", "rules": rules});
    assert_eq!(
        put(&app, first, saved.clone()).await,
        (StatusCode::OK, saved.clone())
    );
    assert_eq!(get(&app, first).await, (StatusCode::OK, saved));
    assert_eq!(
        get(&app, second).await,
        (StatusCode::OK, json!({"language": "en", "rules": {}}))
    );
}

#[tokio::test]
async fn put_replaces_rules_whole_and_keeps_omitted_fields() {
    let app = quillai_api::app(test_pool().await);
    let id = create_series(&app).await;
    put(
        &app,
        id,
        json!({"language": "es", "rules": {"a": {"enabled": true}, "b": {}}}),
    )
    .await;
    let (status, body) = put(&app, id, json!({"rules": {"c": {"scope": "dialogue"}}})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"language": "es", "rules": {"c": {"scope": "dialogue"}}})
    );
    let (_, body) = put(&app, id, json!({"language": "en"})).await;
    assert_eq!(
        body,
        json!({"language": "en", "rules": {"c": {"scope": "dialogue"}}})
    );
}

#[tokio::test]
async fn unknown_rule_ids_and_param_keys_are_kept() {
    let app = quillai_api::app(test_pool().await);
    let id = create_series(&app).await;
    let rules = json!({"future-rule": {"params": {"anything": {"x": [1, "y"]}}, "extra": true}});
    let (status, body) = put(&app, id, json!({"rules": rules})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["rules"], rules);
}

#[tokio::test]
async fn missing_deleted_and_non_book_series_are_not_found() {
    let app = quillai_api::app(test_pool().await);
    assert_eq!(get(&app, 999).await.0, StatusCode::NOT_FOUND);
    assert_eq!(put(&app, 999, json!({})).await.0, StatusCode::NOT_FOUND);

    let deleted = create_series(&app).await;
    send(&app, "DELETE", &format!("/series/{deleted}"), "").await;
    assert_eq!(get(&app, deleted).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        put(&app, deleted, json!({"language": "es"})).await.0,
        StatusCode::NOT_FOUND
    );

    let in_deleted_universe = create_series(&app).await;
    let (_, container) = send(
        &app,
        "GET",
        &format!("/containers/{in_deleted_universe}"),
        "",
    )
    .await;
    let universe = container["universeId"].as_i64().unwrap();
    send(&app, "DELETE", &format!("/universes/{universe}"), "").await;
    assert_eq!(
        get(&app, in_deleted_universe).await.0,
        StatusCode::NOT_FOUND
    );

    let (_, universe) = send(
        &app,
        "POST",
        "/universes",
        &json!({"title": "U", "position": 0}).to_string(),
    )
    .await;
    let tv = json!({"title": "TV", "position": 0, "kind": "tv_series"}).to_string();
    let (status, tv) = send(
        &app,
        "POST",
        &format!("/universes/{}/containers", universe["id"]),
        &tv,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let tv = tv["id"].as_i64().unwrap();
    assert_eq!(get(&app, tv).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        put(&app, tv, json!({"language": "es"})).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn malformed_json_is_400_and_invalid_values_are_422() {
    let app = quillai_api::app(test_pool().await);
    let id = create_series(&app).await;
    let (status, body) = send(&app, "PUT", &format!("/series/{id}/rules"), "{").await;
    assert_eq!(
        (status, body["code"].clone()),
        (StatusCode::BAD_REQUEST, json!("INVALID_JSON"))
    );

    for invalid in [
        json!({"language": "fr"}),
        json!({"language": null}),
        json!({"rules": []}),
        json!({"rules": {"a": true}}),
        json!({"rules": {"a": {"scope": "everywhere"}}}),
        json!({"rules": {"a": {"enabled": "yes"}}}),
        json!({"rules": {"a": {"params": []}}}),
        json!([]),
    ] {
        let (status, body) = put(&app, id, invalid.clone()).await;
        assert_eq!(
            (status, body["code"].clone()),
            (StatusCode::UNPROCESSABLE_ENTITY, json!("VALIDATION_ERROR")),
            "{invalid}"
        );
    }
    assert_eq!(
        get(&app, id).await,
        (StatusCode::OK, json!({"language": "en", "rules": {}}))
    );
}

#[tokio::test]
async fn migration_down_drops_both_columns_and_up_restores_defaults() {
    let pool = test_pool().await;
    let columns = |pool: sqlx::SqlitePool| async move {
        sqlx::query_scalar::<_, String>("SELECT name FROM pragma_table_info('containers') WHERE name IN ('language', 'rules_config') ORDER BY name")
            .fetch_all(&pool)
            .await
            .unwrap()
    };
    assert_eq!(columns(pool.clone()).await, ["language", "rules_config"]);
    MIGRATOR.undo(&pool, BEFORE_RULES).await.unwrap();
    assert!(columns(pool.clone()).await.is_empty());
    MIGRATOR.run(&pool).await.unwrap();
    assert_eq!(columns(pool.clone()).await, ["language", "rules_config"]);
    let insert = |language: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query("INSERT INTO universes (title, position) VALUES ('u', 0)")
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO containers (universe_id, title, position, kind, language) VALUES (last_insert_rowid(), 't', 0, 'collection', ?)")
                .bind(language)
                .execute(&pool)
                .await
                .is_ok()
        }
    };
    assert!(insert("es").await);
    assert!(!insert("fr").await);
}
