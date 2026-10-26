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

async fn post(app: &Router, uri: &str, body: Value, expected: StatusCode) -> Value {
    let (status, body) = request(app, "POST", uri, body).await;
    assert_eq!(status, expected, "{uri}: {body}");
    body
}

async fn id(app: &Router, uri: &str, body: Value) -> i64 {
    post(app, uri, body, StatusCode::CREATED).await["id"]
        .as_i64()
        .unwrap()
}

async fn universe(app: &Router) -> i64 {
    let universe = id(app, "/universes", json!({"title": "World", "position": 0})).await;
    post(
        app,
        &format!("/universes/{universe}/lore"),
        json!({"kind": "character", "name": "Ada", "keywords": ["engine"], "pinned": true}),
        StatusCode::CREATED,
    )
    .await;
    universe
}

async fn container(app: &Router, universe: i64, kind: &str) -> i64 {
    id(
        app,
        &format!("/universes/{universe}/containers"),
        json!({"title": kind, "position": 0, "kind": kind}),
    )
    .await
}

async fn leaf(app: &Router, uri: &str, label: &str) -> i64 {
    let leaf = id(
        app,
        uri,
        json!({"title": "Draft", "position": 3, "label": label, "shape": "leaf"}),
    )
    .await;
    let deltas = format!("/pieces/{leaf}/deltas");
    post(
        app,
        &deltas,
        json!({"baseRev": 0, "delta": {"ops": [{"insert": "Hello"}]}}),
        StatusCode::OK,
    )
    .await;
    post(
        app,
        &deltas,
        json!({"baseRev": 1, "delta": {"ops": [{"retain": 5}, {"insert": " world"}]}}),
        StatusCode::OK,
    )
    .await;
    leaf
}

async fn group(app: &Router, uri: &str, label: &str) -> i64 {
    id(
        app,
        uri,
        json!({"title": "Book", "position": 0, "label": label, "shape": "group"}),
    )
    .await
}

struct Snapshot {
    content: Value,
    head_rev: Value,
    revisions: Value,
    lore: Value,
}

async fn snapshot(app: &Router, universe: i64, leaf: i64) -> Snapshot {
    let detail = get(app, &format!("/pieces/{leaf}")).await;
    Snapshot {
        content: detail["content"].clone(),
        head_rev: detail["headRev"].clone(),
        revisions: get(app, &format!("/pieces/{leaf}/deltas?since=0")).await,
        lore: get(app, &format!("/universes/{universe}/lore")).await,
    }
}

async fn assert_stable(app: &Router, universe: i64, leaf: i64, before: &Snapshot) {
    let after = snapshot(app, universe, leaf).await;
    assert_eq!(after.content, json!({"ops": [{"insert": "Hello world"}]}));
    assert_eq!(after.content, before.content);
    assert_eq!(after.head_rev, json!(2));
    assert_eq!(after.head_rev, before.head_rev);
    assert_eq!(after.revisions, before.revisions);
    assert_eq!(after.lore, before.lore);
    assert_eq!(after.lore.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn placement_moves_roots_inside_the_universe() {
    let app = quillai_api::app(test_pool().await);
    let world = universe(&app).await;
    let first = container(&app, world, "collection").await;
    let second = container(&app, world, "collection").await;
    let books = container(&app, world, "book_series").await;
    let piece = leaf(&app, &format!("/containers/{first}/pieces"), "novel").await;
    let before = snapshot(&app, world, piece).await;
    let placement = format!("/pieces/{piece}/placement");

    let moved = post(
        &app,
        &placement,
        json!({"containerId": second, "position": 7}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(moved["id"], piece);
    assert_eq!(moved["containerId"], second);
    assert_eq!(moved["position"], 7);
    assert_eq!(
        get(&app, &format!("/containers/{first}/pieces")).await,
        json!([])
    );
    assert_eq!(
        get(&app, &format!("/containers/{second}/pieces")).await[0]["id"],
        piece
    );
    assert_stable(&app, world, piece, &before).await;

    let stored = post(
        &app,
        &placement,
        json!({"containerId": null, "position": 0}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(stored["containerId"], Value::Null);
    assert_eq!(
        get(&app, &format!("/universes/{world}/pieces?container=none")).await[0]["id"],
        piece
    );
    post(
        &app,
        &placement,
        json!({"containerId": books, "position": 0}),
        StatusCode::OK,
    )
    .await;
    assert_stable(&app, world, piece, &before).await;

    let novel = group(&app, &format!("/containers/{first}/pieces"), "novel").await;
    let moved = post(
        &app,
        &format!("/pieces/{novel}/placement"),
        json!({"containerId": second, "position": 1}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(moved["shape"], "group");
    assert_eq!(moved["children"], json!([]));
}

#[tokio::test]
async fn placement_rejects_other_universes_membership_and_bad_input() {
    let app = quillai_api::app(test_pool().await);
    let world = universe(&app).await;
    let other = universe(&app).await;
    let foreign = container(&app, other, "collection").await;
    let tv = container(&app, world, "tv_series").await;
    let books = container(&app, world, "book_series").await;
    let piece = leaf(&app, &format!("/universes/{world}/pieces"), "short_story").await;
    let before = snapshot(&app, world, piece).await;
    let placement = format!("/pieces/{piece}/placement");
    for body in [
        json!({"containerId": foreign, "position": 0}),
        json!({"containerId": tv, "position": 0}),
        json!({"containerId": books, "position": 0}),
        json!({"containerId": 999, "position": 0}),
        json!({"containerId": null, "position": -1}),
        json!({"containerId": null}),
        json!({"containerId": "x", "position": 0}),
    ] {
        post(&app, &placement, body, UNPROCESSABLE).await;
    }
    post(
        &app,
        "/pieces/999/placement",
        json!({"containerId": null, "position": 0}),
        StatusCode::NOT_FOUND,
    )
    .await;
    let detail = get(&app, &format!("/pieces/{piece}")).await;
    assert_eq!(detail["containerId"], Value::Null);
    assert_eq!(detail["position"], 3);
    assert_stable(&app, world, piece, &before).await;
}

#[tokio::test]
async fn attach_joins_a_root_leaf_to_a_group_and_detach_returns_it() {
    let app = quillai_api::app(test_pool().await);
    let world = universe(&app).await;
    let books = container(&app, world, "book_series").await;
    let collection = container(&app, world, "collection").await;
    let novel = group(&app, &format!("/containers/{books}/pieces"), "novel").await;
    let piece = leaf(&app, &format!("/containers/{collection}/pieces"), "chapter").await;
    let before = snapshot(&app, world, piece).await;

    let attached = post(
        &app,
        &format!("/pieces/{novel}/attach"),
        json!({"pieceId": piece, "position": 4}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(attached["id"], novel);
    let child = &attached["children"][0];
    assert_eq!(child["id"], piece);
    assert_eq!(child["parentId"], novel);
    assert_eq!(child["containerId"], Value::Null);
    assert_eq!(child["position"], 4);
    assert_eq!(child["label"], "chapter");
    assert_eq!(
        get(&app, &format!("/containers/{collection}/pieces")).await,
        json!([])
    );
    assert_stable(&app, world, piece, &before).await;

    let book = get(&app, &format!("/series/{books}/books")).await[0]["id"].clone();
    let chapters = get(&app, &format!("/series/{books}/books/{book}/chapters")).await;
    assert_eq!(chapters[0]["title"], "Draft");
    assert_eq!(chapters[0]["head_rev"], 2);

    post(
        &app,
        &format!("/pieces/{piece}/placement"),
        json!({"containerId": collection, "position": 0}),
        UNPROCESSABLE,
    )
    .await;
    let detached = post(
        &app,
        &format!("/pieces/{piece}/placement"),
        json!({"containerId": null, "position": 2}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(detached["parentId"], Value::Null);
    assert_eq!(detached["containerId"], Value::Null);
    assert_eq!(detached["position"], 2);
    assert_eq!(detached["label"], "chapter");
    assert_eq!(
        get(&app, &format!("/pieces/{novel}")).await["children"],
        json!([])
    );
    assert_eq!(
        get(&app, &format!("/universes/{world}/pieces?container=none")).await[0]["id"],
        piece
    );
    assert_stable(&app, world, piece, &before).await;
}

#[tokio::test]
async fn attach_rejects_groups_children_formats_and_other_universes() {
    let app = quillai_api::app(test_pool().await);
    let world = universe(&app).await;
    let other = universe(&app).await;
    let loose = format!("/universes/{world}/pieces");
    let novel = group(&app, &loose, "novel").await;
    let screenplay = group(&app, &loose, "screenplay").await;
    let second_novel = group(&app, &loose, "novel").await;
    let foreign_novel = group(&app, &format!("/universes/{other}/pieces"), "novel").await;
    let foreign_leaf = leaf(&app, &format!("/universes/{other}/pieces"), "piece").await;
    let piece = leaf(&app, &loose, "piece").await;
    let target_leaf = leaf(&app, &loose, "piece").await;
    let before = snapshot(&app, world, piece).await;
    let attach = format!("/pieces/{novel}/attach");
    for (uri, body) in [
        (
            attach.clone(),
            json!({"pieceId": second_novel, "position": 0}),
        ),
        (
            attach.clone(),
            json!({"pieceId": foreign_leaf, "position": 0}),
        ),
        (attach.clone(), json!({"pieceId": 999, "position": 0})),
        (attach.clone(), json!({"pieceId": piece, "position": -1})),
        (attach.clone(), json!({"pieceId": piece})),
        (
            format!("/pieces/{foreign_novel}/attach"),
            json!({"pieceId": piece, "position": 0}),
        ),
        (
            format!("/pieces/{screenplay}/attach"),
            json!({"pieceId": piece, "position": 0}),
        ),
        (
            format!("/pieces/{target_leaf}/attach"),
            json!({"pieceId": piece, "position": 0}),
        ),
    ] {
        post(&app, &uri, body, UNPROCESSABLE).await;
    }
    post(
        &app,
        "/pieces/999/attach",
        json!({"pieceId": piece, "position": 0}),
        StatusCode::NOT_FOUND,
    )
    .await;
    post(
        &app,
        &attach,
        json!({"pieceId": piece, "position": 0}),
        StatusCode::OK,
    )
    .await;
    post(
        &app,
        &format!("/pieces/{second_novel}/attach"),
        json!({"pieceId": piece, "position": 0}),
        UNPROCESSABLE,
    )
    .await;
    assert_eq!(
        get(&app, &format!("/pieces/{piece}")).await["parentId"],
        novel
    );
    assert_stable(&app, world, piece, &before).await;
}

#[tokio::test]
async fn split_wraps_a_root_leaf_in_a_new_group() {
    let app = quillai_api::app(test_pool().await);
    let world = universe(&app).await;
    let books = container(&app, world, "book_series").await;
    let piece = leaf(&app, &format!("/containers/{books}/pieces"), "novel").await;
    let before = snapshot(&app, world, piece).await;
    let facade_before = get(&app, &format!("/series/{books}/books")).await;
    assert_eq!(facade_before, json!([]));

    let split = post(
        &app,
        &format!("/pieces/{piece}/split"),
        json!({}),
        StatusCode::CREATED,
    )
    .await;
    let group_id = split["id"].as_i64().unwrap();
    assert_ne!(group_id, piece);
    assert_eq!(split["universeId"], world);
    assert_eq!(split["containerId"], books);
    assert_eq!(split["parentId"], Value::Null);
    assert_eq!(split["title"], "Draft");
    assert_eq!(split["position"], 3);
    assert_eq!(split["label"], "novel");
    assert_eq!(split["format"], "prose");
    assert_eq!(split["shape"], "group");
    let child = &split["children"][0];
    assert_eq!(split["children"].as_array().unwrap().len(), 1);
    assert_eq!(child["id"], piece);
    assert_eq!(child["parentId"], group_id);
    assert_eq!(child["containerId"], Value::Null);
    assert_eq!(child["position"], 0);
    assert_eq!(child["label"], "chapter");
    assert_eq!(
        get(&app, &format!("/containers/{books}/pieces")).await[0]["id"],
        group_id
    );
    assert_stable(&app, world, piece, &before).await;

    let book = get(&app, &format!("/series/{books}/books")).await[0]["id"].clone();
    let chapter = &get(
        &app,
        &format!("/series/{books}/books/{book}/chapters?content=true"),
    )
    .await[0];
    assert_eq!(chapter["content"], before.content);
    assert_eq!(chapter["kind"], "chapter");

    let tv = container(&app, world, "tv_series").await;
    let episode = leaf(&app, &format!("/containers/{tv}/pieces"), "episode").await;
    let script = leaf(&app, &format!("/universes/{world}/pieces"), "screenplay").await;
    let split = post(
        &app,
        &format!("/pieces/{script}/split"),
        json!({}),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(split["format"], "script");
    assert_eq!(split["children"][0]["label"], "scene");
    let labelled = post(
        &app,
        &format!("/pieces/{episode}/split"),
        json!({"label": "prologue"}),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(labelled["containerId"], tv);
    assert_eq!(labelled["children"][0]["label"], "prologue");
}

#[tokio::test]
async fn split_rejects_groups_children_and_bad_labels() {
    let app = quillai_api::app(test_pool().await);
    let world = universe(&app).await;
    let piece = leaf(&app, &format!("/universes/{world}/pieces"), "piece").await;
    let before = snapshot(&app, world, piece).await;
    post(
        &app,
        &format!("/pieces/{piece}/split"),
        json!({"label": "poem"}),
        UNPROCESSABLE,
    )
    .await;
    let split = post(
        &app,
        &format!("/pieces/{piece}/split"),
        json!({}),
        StatusCode::CREATED,
    )
    .await;
    let group_id = split["id"].as_i64().unwrap();
    post(
        &app,
        &format!("/pieces/{piece}/split"),
        json!({}),
        UNPROCESSABLE,
    )
    .await;
    post(
        &app,
        &format!("/pieces/{group_id}/split"),
        json!({}),
        UNPROCESSABLE,
    )
    .await;
    post(&app, "/pieces/999/split", json!({}), StatusCode::NOT_FOUND).await;
    let loose = get(&app, &format!("/universes/{world}/pieces?container=none")).await;
    assert_eq!(loose.as_array().unwrap().len(), 1);
    assert_eq!(loose[0]["id"], group_id);
    assert_stable(&app, world, piece, &before).await;
}
