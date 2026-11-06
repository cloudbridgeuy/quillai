use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

async fn request(
    app: &Router,
    method: &str,
    uri: &str,
    body: Value,
    expected: StatusCode,
) -> Value {
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
    assert_eq!(response.status(), expected, "{method} {uri}");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn create(app: &Router, uri: &str, body: Value) -> i64 {
    request(app, "POST", uri, body, StatusCode::CREATED).await["id"]
        .as_i64()
        .unwrap()
}

struct World {
    app: Router,
    universe: i64,
    entry: i64,
}

impl World {
    fn states(&self) -> String {
        format!("/universes/{}/lore/{}/states", self.universe, self.entry)
    }

    async fn point(&self, name: &str, position: i64) -> i64 {
        create(
            &self.app,
            &format!("/universes/{}/timeline", self.universe),
            json!({"name":name,"position":position}),
        )
        .await
    }

    async fn state(&self, name: &str, point: i64, expected: StatusCode) -> Value {
        request(
            &self.app,
            "POST",
            &self.states(),
            json!({"name":name,"pointId":point,"visibleText":"seen","hiddenNotes":"secret"}),
            expected,
        )
        .await
    }

    async fn entry_json(&self) -> Value {
        request(
            &self.app,
            "GET",
            &format!("/universes/{}/lore/{}", self.universe, self.entry),
            Value::Null,
            StatusCode::OK,
        )
        .await
    }
}

async fn world() -> World {
    let app = quillai_api::app(common::test_pool().await);
    let universe = create(&app, "/universes", json!({"title":"World","position":0})).await;
    let entry = create(
        &app,
        &format!("/universes/{universe}/lore"),
        json!({"kind":"character","name":"Mara"}),
    )
    .await;
    World {
        app,
        universe,
        entry,
    }
}

fn ids(states: &Value) -> Vec<i64> {
    states
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_i64().unwrap())
        .collect()
}

#[tokio::test]
async fn create_state_returns_camel_case_fields_and_lists_on_entry() {
    let w = world().await;
    let p = w.point("Act 1", 1).await;
    let s = w.state("Young", p, StatusCode::CREATED).await;
    assert_eq!(
        s,
        json!({"id":s["id"],"name":"Young","pointId":p,"visibleText":"seen","hiddenNotes":"secret"})
    );
    let listed = request(&w.app, "GET", &w.states(), Value::Null, StatusCode::OK).await;
    assert_eq!(listed, json!([s]));
    assert_eq!(w.entry_json().await["states"], json!([s]));
    let list = request(
        &w.app,
        "GET",
        &format!("/universes/{}/lore", w.universe),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(list[0]["states"], json!([s]));
    let other = create(&w.app, "/universes", json!({"title":"Other","position":1})).await;
    request(
        &w.app,
        "GET",
        &format!("/universes/{other}/lore/{}/states", w.entry),
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
}

#[tokio::test]
async fn duplicate_point_is_conflict_on_create_and_update() {
    let w = world().await;
    let (a, b) = (w.point("A", 1).await, w.point("B", 2).await);
    w.state("One", a, StatusCode::CREATED).await;
    w.state("Dup", a, StatusCode::CONFLICT).await;
    let second = w.state("Two", b, StatusCode::CREATED).await;
    request(
        &w.app,
        "PUT",
        &format!("{}/{}", w.states(), second["id"]),
        json!({"pointId":a}),
        StatusCode::CONFLICT,
    )
    .await;
}

#[tokio::test]
async fn cross_universe_and_deleted_points_are_unprocessable() {
    let w = world().await;
    let other = create(&w.app, "/universes", json!({"title":"Other","position":1})).await;
    let foreign = create(
        &w.app,
        &format!("/universes/{other}/timeline"),
        json!({"name":"Far","position":0}),
    )
    .await;
    w.state("X", foreign, StatusCode::UNPROCESSABLE_ENTITY)
        .await;
    w.state("X", 99999, StatusCode::UNPROCESSABLE_ENTITY).await;
    let gone = w.point("Gone", 3).await;
    request(
        &w.app,
        "DELETE",
        &format!("/timeline/{gone}"),
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    w.state("X", gone, StatusCode::UNPROCESSABLE_ENTITY).await;
    let ok = w.point("Ok", 4).await;
    let s = w.state("Y", ok, StatusCode::CREATED).await;
    let uri = format!("{}/{}", w.states(), s["id"]);
    request(
        &w.app,
        "PUT",
        &uri,
        json!({"pointId":foreign}),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    request(
        &w.app,
        "PUT",
        &uri,
        json!({"name":"  "}),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    w.state("  ", ok, StatusCode::UNPROCESSABLE_ENTITY).await;
}

#[tokio::test]
async fn state_delete_and_restore_round_trip() {
    let w = world().await;
    let p = w.point("A", 1).await;
    let s = w.state("One", p, StatusCode::CREATED).await;
    let uri = format!("{}/{}", w.states(), s["id"]);
    request(&w.app, "DELETE", &uri, Value::Null, StatusCode::NO_CONTENT).await;
    assert_eq!(w.entry_json().await["states"], json!([]));
    request(&w.app, "DELETE", &uri, Value::Null, StatusCode::NOT_FOUND).await;
    request(
        &w.app,
        "PUT",
        &uri,
        json!({"name":"n"}),
        StatusCode::NOT_FOUND,
    )
    .await;
    let back = request(
        &w.app,
        "POST",
        &format!("{uri}/restore"),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(back, s);
    assert_eq!(w.entry_json().await["states"], json!([s]));
    request(
        &w.app,
        "POST",
        &format!("{uri}/restore"),
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
}

#[tokio::test]
async fn restore_into_taken_point_is_conflict() {
    let w = world().await;
    let p = w.point("A", 1).await;
    let s = w.state("One", p, StatusCode::CREATED).await;
    let uri = format!("{}/{}", w.states(), s["id"]);
    request(&w.app, "DELETE", &uri, Value::Null, StatusCode::NO_CONTENT).await;
    w.state("Two", p, StatusCode::CREATED).await;
    request(
        &w.app,
        "POST",
        &format!("{uri}/restore"),
        Value::Null,
        StatusCode::CONFLICT,
    )
    .await;
}

#[tokio::test]
async fn deleting_entry_hides_states_and_restore_brings_them_back() {
    let w = world().await;
    let p = w.point("A", 1).await;
    let s = w.state("One", p, StatusCode::CREATED).await;
    let entry_uri = format!("/universes/{}/lore/{}", w.universe, w.entry);
    request(
        &w.app,
        "DELETE",
        &entry_uri,
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    request(
        &w.app,
        "GET",
        &w.states(),
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    w.state("Two", p, StatusCode::NOT_FOUND).await;
    let list = request(
        &w.app,
        "GET",
        &format!("/universes/{}/lore", w.universe),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(list, json!([]));
    let restored = request(
        &w.app,
        "POST",
        &format!("{entry_uri}/restore"),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(restored["states"], json!([s]));
}

#[tokio::test]
async fn states_order_by_point_position_after_reordering_and_keep_deleted_points() {
    let w = world().await;
    let (a, b, c) = (
        w.point("A", 1).await,
        w.point("B", 2).await,
        w.point("C", 3).await,
    );
    let sa = w.state("sa", a, StatusCode::CREATED).await["id"]
        .as_i64()
        .unwrap();
    let sb = w.state("sb", b, StatusCode::CREATED).await["id"]
        .as_i64()
        .unwrap();
    let sc = w.state("sc", c, StatusCode::CREATED).await["id"]
        .as_i64()
        .unwrap();
    assert_eq!(ids(&w.entry_json().await["states"]), [sa, sb, sc]);
    request(
        &w.app,
        "PUT",
        &format!("/timeline/{a}"),
        json!({"position":9}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(ids(&w.entry_json().await["states"]), [sb, sc, sa]);
    request(
        &w.app,
        "DELETE",
        &format!("/timeline/{b}"),
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(ids(&w.entry_json().await["states"]), [sb, sc, sa]);
    let list = request(
        &w.app,
        "GET",
        &format!("/universes/{}/lore", w.universe),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(ids(&list[0]["states"]), [sb, sc, sa]);
}

#[tokio::test]
async fn missing_universe_entry_or_state_is_not_found() {
    let w = world().await;
    request(
        &w.app,
        "GET",
        &format!("/universes/9999/lore/{}/states", w.entry),
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    request(
        &w.app,
        "GET",
        &format!("/universes/{}/lore/9999/states", w.universe),
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    request(
        &w.app,
        "DELETE",
        &format!("{}/9999", w.states()),
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    request(
        &w.app,
        "POST",
        &format!("{}/9999/restore", w.states()),
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
}
