use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tower::ServiceExt;

mod common;

use common::test_pool;

const UNPROCESSABLE: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;

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

async fn get(app: &Router, uri: &str) -> Value {
    let (status, body) = request(app, "GET", uri, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    body
}

async fn rejected(app: &Router, uri: &str, expected: StatusCode) {
    let (status, body) = request(app, "GET", uri, Value::Null).await;
    assert_eq!(status, expected, "{uri}: {body}");
}

async fn created(app: &Router, uri: &str, body: Value) -> i64 {
    let (status, body) = request(app, "POST", uri, body).await;
    assert_eq!(status, StatusCode::CREATED, "{uri}: {body}");
    body["id"].as_i64().unwrap()
}

async fn type_text(app: &Router, scene: i64, base_rev: i64, text: &str) {
    let (status, body) = request(
        app,
        "POST",
        &format!("/pieces/{scene}/deltas"),
        json!({"baseRev": base_rev, "delta": {"ops": [{"insert": text}]}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

struct Script {
    app: Router,
    pool: SqlitePool,
    universe: i64,
    group: i64,
    first: i64,
    second: i64,
}

async fn script() -> Script {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let universe = created(&app, "/universes", json!({"title": "World", "position": 0})).await;
    let group = created(
        &app,
        &format!("/universes/{universe}/pieces"),
        json!({"title": "Pilot", "position": 0, "label": "screenplay", "shape": "group"}),
    )
    .await;
    let children = format!("/pieces/{group}/children");
    let second = created(&app, &children, json!({"title": "Two", "position": 1})).await;
    let first = created(&app, &children, json!({"title": "One", "position": 0})).await;
    type_text(&app, first, 0, "A").await;
    type_text(&app, second, 0, "B").await;
    type_text(&app, second, 1, "C").await;
    type_text(&app, second, 2, "D").await;
    Script {
        app,
        pool,
        universe,
        group,
        first,
        second,
    }
}

#[tokio::test]
async fn scenes_load_in_order_with_content_and_head_rev() {
    let s = script().await;
    let body = get(&s.app, &format!("/pieces/{}/scenes", s.group)).await;
    assert_eq!(body["id"], s.group);
    assert_eq!(body["universeId"], s.universe);
    assert_eq!(body["format"], "script");
    assert_eq!(body["shape"], "group");
    assert!(body.get("children").is_none());
    let scenes = body["scenes"].as_array().unwrap();
    assert_eq!(scenes.len(), 2);
    for (scene, id) in scenes.iter().zip([s.first, s.second]) {
        assert_eq!(
            scene,
            &get(&s.app, &format!("/pieces/{id}")).await,
            "scene {id} matches its leaf GET"
        );
    }
    assert_eq!(scenes[0]["title"], "One");
    assert_eq!(scenes[0]["headRev"], 1);
    assert_eq!(scenes[0]["content"], json!({"ops": [{"insert": "A"}]}));
    assert_eq!(scenes[1]["headRev"], 3);
    assert_eq!(scenes[1]["content"], json!({"ops": [{"insert": "DCB"}]}));
    assert_eq!(scenes[1]["parentId"], s.group);
}

#[tokio::test]
async fn empty_script_group_loads_no_scenes() {
    let s = script().await;
    let empty = created(
        &s.app,
        &format!("/universes/{}/pieces", s.universe),
        json!({"title": "Empty", "position": 1, "label": "screenplay", "shape": "group"}),
    )
    .await;
    let body = get(&s.app, &format!("/pieces/{empty}/scenes")).await;
    assert_eq!(body["scenes"], json!([]));
    let poll = get(&s.app, &format!("/pieces/{empty}/scenes/deltas?since=")).await;
    assert_eq!(poll, json!({"children": [], "scenes": {}}));
}

#[tokio::test]
async fn poll_returns_revisions_above_each_scene_since() {
    let s = script().await;
    let uri = format!(
        "/pieces/{}/scenes/deltas?since={}:1,{}:1",
        s.group, s.first, s.second
    );
    let body = get(&s.app, &uri).await;
    assert_eq!(body["children"], json!([s.first, s.second]));
    assert_eq!(
        body["scenes"][s.first.to_string()],
        json!({"revisions": []})
    );
    assert_eq!(
        body["scenes"][s.second.to_string()]["revisions"],
        get(&s.app, &format!("/pieces/{}/deltas?since=1", s.second)).await
    );
    assert_eq!(
        body["scenes"][s.second.to_string()]["revisions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|revision| revision["rev"].clone())
            .collect::<Vec<_>>(),
        [json!(2), json!(3)]
    );
}

#[tokio::test]
async fn poll_reports_only_requested_scenes() {
    let s = script().await;
    let uri = format!("/pieces/{}/scenes/deltas?since={}:0", s.group, s.second);
    let body = get(&s.app, &uri).await;
    assert_eq!(body["children"], json!([s.first, s.second]));
    let scenes = body["scenes"].as_object().unwrap();
    assert_eq!(scenes.len(), 1);
    assert_eq!(
        scenes[&s.second.to_string()]["revisions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[tokio::test]
async fn future_history_is_a_conflict_for_that_scene_only() {
    let s = script().await;
    for since in ["9", "-1"] {
        let uri = format!(
            "/pieces/{}/scenes/deltas?since={}:{since},{}:3",
            s.group, s.first, s.second
        );
        let body = get(&s.app, &uri).await;
        assert_eq!(
            body["scenes"][s.first.to_string()],
            json!({"conflict": "future"})
        );
        assert_eq!(
            body["scenes"][s.second.to_string()],
            json!({"revisions": []})
        );
    }
}

#[tokio::test]
async fn unavailable_history_is_a_conflict_for_that_scene_only() {
    let s = script().await;
    sqlx::query("DELETE FROM piece_revisions WHERE piece_id = ? AND rev = 2")
        .bind(s.second)
        .execute(&s.pool)
        .await
        .unwrap();
    rejected(
        &s.app,
        &format!("/pieces/{}/deltas?since=1", s.second),
        StatusCode::CONFLICT,
    )
    .await;
    let uri = format!(
        "/pieces/{}/scenes/deltas?since={}:0,{}:1",
        s.group, s.first, s.second
    );
    let body = get(&s.app, &uri).await;
    assert_eq!(
        body["scenes"][s.first.to_string()]["revisions"][0]["delta"],
        json!({"ops": [{"insert": "A"}]})
    );
    assert_eq!(
        body["scenes"][s.second.to_string()],
        json!({"conflict": "unavailable"})
    );
    let uri = format!("/pieces/{}/scenes/deltas?since={}:2", s.group, s.second);
    let body = get(&s.app, &uri).await;
    assert_eq!(
        body["scenes"][s.second.to_string()]["revisions"][0]["rev"],
        3
    );
}

#[tokio::test]
async fn poll_rejects_strangers_and_malformed_since() {
    let s = script().await;
    let other = created(
        &s.app,
        &format!("/universes/{}/pieces", s.universe),
        json!({"title": "Loose", "position": 2, "label": "screenplay", "shape": "leaf"}),
    )
    .await;
    let base = format!("/pieces/{}/scenes/deltas", s.group);
    for query in [
        format!("?since={other}:0"),
        format!("?since={}:0,{}:1", s.first, s.first),
        format!("?since={}", s.first),
        format!("?since={}:x", s.first),
        "?since=x:0".to_string(),
        String::new(),
    ] {
        rejected(&s.app, &format!("{base}{query}"), UNPROCESSABLE).await;
    }
}

#[tokio::test]
async fn leaf_prose_group_and_missing_ids_are_rejected() {
    let s = script().await;
    let prose = created(
        &s.app,
        &format!("/universes/{}/pieces", s.universe),
        json!({"title": "Novel", "position": 1, "label": "novel", "shape": "group"}),
    )
    .await;
    created(
        &s.app,
        &format!("/pieces/{prose}/children"),
        json!({"title": "Chapter", "position": 0}),
    )
    .await;
    for id in [s.first, prose] {
        rejected(&s.app, &format!("/pieces/{id}/scenes"), UNPROCESSABLE).await;
        rejected(
            &s.app,
            &format!("/pieces/{id}/scenes/deltas?since="),
            UNPROCESSABLE,
        )
        .await;
    }
    rejected(&s.app, "/pieces/999/scenes", StatusCode::NOT_FOUND).await;
    rejected(
        &s.app,
        "/pieces/999/scenes/deltas?since=",
        StatusCode::NOT_FOUND,
    )
    .await;
}
