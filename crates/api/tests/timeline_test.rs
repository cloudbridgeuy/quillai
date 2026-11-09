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
    piece: i64,
}

async fn world() -> World {
    let app = quillai_api::app(common::test_pool().await);
    let universe = create(&app, "/universes", json!({"title":"World","position":0})).await;
    let piece = create(
        &app,
        &format!("/universes/{universe}/pieces"),
        json!({"title":"Loose","position":0,"label":"piece","shape":"leaf"}),
    )
    .await;
    World {
        app,
        universe,
        piece,
    }
}

async fn point(app: &Router, universe: i64, name: &str, position: i64) -> i64 {
    create(
        app,
        &format!("/universes/{universe}/timeline"),
        json!({"name":name,"position":position}),
    )
    .await
}

async fn set_point(w: &World, body: Value, expected: StatusCode) -> Value {
    request(
        &w.app,
        "PUT",
        &format!("/pieces/{}", w.piece),
        body,
        expected,
    )
    .await
}

#[tokio::test]
async fn points_list_in_order_and_stay_in_their_universe() {
    let w = world().await;
    let other = create(&w.app, "/universes", json!({"title":"Other","position":1})).await;
    let late = point(&w.app, w.universe, "Late", 2).await;
    let early = point(&w.app, w.universe, "Early", 1).await;
    point(&w.app, other, "Elsewhere", 0).await;
    let list = request(
        &w.app,
        "GET",
        &format!("/universes/{}/timeline", w.universe),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    let ids: Vec<i64> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, [early, late]);
    assert_eq!(list[0]["universeId"], w.universe);
    request(
        &w.app,
        "GET",
        "/universes/9999/timeline",
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    request(
        &w.app,
        "POST",
        &format!("/universes/{}/timeline", w.universe),
        json!({"name":"  ","position":0}),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    let renamed = request(
        &w.app,
        "PUT",
        &format!("/timeline/{late}"),
        json!({"name":"Later","position":0}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(renamed["name"], "Later");
    request(
        &w.app,
        "PUT",
        &format!("/timeline/{late}"),
        json!({"name":""}),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
}

#[tokio::test]
async fn delete_then_restore_a_point() {
    let w = world().await;
    let id = point(&w.app, w.universe, "Act one", 0).await;
    let list_uri = format!("/universes/{}/timeline", w.universe);
    request(
        &w.app,
        "DELETE",
        &format!("/timeline/{id}"),
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        request(&w.app, "GET", &list_uri, Value::Null, StatusCode::OK).await,
        json!([])
    );
    request(
        &w.app,
        "DELETE",
        &format!("/timeline/{id}"),
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    let restored = request(
        &w.app,
        "POST",
        &format!("/timeline/{id}/restore"),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(restored["id"], id);
    let list = request(&w.app, "GET", &list_uri, Value::Null, StatusCode::OK).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    request(
        &w.app,
        "POST",
        &format!("/timeline/{id}/restore"),
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
}

#[tokio::test]
async fn piece_point_rejects_other_universe_and_deleted_and_missing() {
    let w = world().await;
    let other = create(&w.app, "/universes", json!({"title":"Other","position":1})).await;
    let foreign = point(&w.app, other, "Elsewhere", 0).await;
    set_point(
        &w,
        json!({"timelinePointId":foreign}),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    let gone = point(&w.app, w.universe, "Gone", 0).await;
    request(
        &w.app,
        "DELETE",
        &format!("/timeline/{gone}"),
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    set_point(
        &w,
        json!({"timelinePointId":gone}),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    set_point(
        &w,
        json!({"timelinePointId":9999}),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    set_point(
        &w,
        json!({"timelinePointId":"x"}),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
}

#[tokio::test]
async fn piece_point_sets_keeps_on_absent_and_clears_on_null() {
    let w = world().await;
    let id = point(&w.app, w.universe, "Act one", 0).await;
    let piece_uri = format!("/pieces/{}", w.piece);
    let before = request(&w.app, "GET", &piece_uri, Value::Null, StatusCode::OK).await;
    assert_eq!(before["timelinePointId"], Value::Null);
    let set = set_point(&w, json!({"timelinePointId":id}), StatusCode::OK).await;
    assert_eq!(set["timelinePointId"], id);
    let kept = set_point(&w, json!({"title":"Renamed"}), StatusCode::OK).await;
    assert_eq!(kept["timelinePointId"], id);
    assert_eq!(kept["title"], "Renamed");

    let tree = request(&w.app, "GET", "/tree", Value::Null, StatusCode::OK).await;
    assert_eq!(tree["universes"][0]["loose"][0]["timelinePointId"], id);
    let list = request(
        &w.app,
        "GET",
        &format!("/universes/{}/pieces?container=none", w.universe),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(list[0]["timelinePointId"], id);

    let cleared = set_point(&w, json!({"timelinePointId":null}), StatusCode::OK).await;
    assert_eq!(cleared["timelinePointId"], Value::Null);
    let tree = request(&w.app, "GET", "/tree", Value::Null, StatusCode::OK).await;
    assert_eq!(
        tree["universes"][0]["loose"][0]["timelinePointId"],
        Value::Null
    );
}

#[tokio::test]
async fn renaming_a_leaf_returns_every_integer_the_client_parses() {
    let w = world().await;
    let body = request(
        &w.app,
        "PUT",
        &format!("/pieces/{}", w.piece),
        json!({"title":"Renamed"}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["title"], "Renamed");
    for field in ["id", "universeId", "position", "headRev"] {
        assert!(body[field].is_i64(), "{field} is not an integer: {body}");
    }
    assert!(body["containerId"].is_null(), "{body}");
    assert!(body["parentId"].is_null(), "{body}");
}
