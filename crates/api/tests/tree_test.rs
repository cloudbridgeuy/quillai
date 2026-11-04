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

async fn get(app: &Router, uri: &str) -> Value {
    request(app, "GET", uri, Value::Null, StatusCode::OK).await
}

async fn delete(app: &Router, uri: &str) {
    request(app, "DELETE", uri, Value::Null, StatusCode::NO_CONTENT).await;
}

async fn pieces(app: &Router, uri: &str) -> Value {
    let mut list = get(app, uri).await;
    for piece in list.as_array_mut().unwrap() {
        let children = if piece["shape"] == "group" {
            get(app, &format!("/pieces/{}", piece["id"])).await["children"].clone()
        } else {
            json!([])
        };
        piece["children"] = children;
    }
    list
}

#[tokio::test]
async fn tree_matches_lists_and_excludes_trash() {
    let app = quillai_api::app(common::test_pool().await);
    let series = create(&app, "/series", json!({"title":"Series","position":1})).await;
    let books = format!("/series/{series}/books");
    let second = create(&app, &books, json!({"title":"Second","position":1})).await;
    let first = create(&app, &books, json!({"title":"First","position":0})).await;
    for book in [second, first] {
        let chapters = format!("{books}/{book}/chapters");
        create(
            &app,
            &chapters,
            json!({"title":"Later","position":1,"kind":"chapter"}),
        )
        .await;
        create(
            &app,
            &chapters,
            json!({"title":"Earlier","position":0,"kind":"chapter"}),
        )
        .await;
    }
    let chapters = format!("{books}/{first}/chapters");
    create(
        &app,
        &chapters,
        json!({"title":"Tie","position":0,"kind":"chapter"}),
    )
    .await;
    let trashed_chapter = create(
        &app,
        &chapters,
        json!({"title":"Trash","position":0,"kind":"chapter"}),
    )
    .await;
    delete(&app, &format!("{chapters}/{trashed_chapter}")).await;
    let trashed_book = create(&app, &books, json!({"title":"Trash","position":0})).await;
    create(
        &app,
        &format!("{books}/{trashed_book}/chapters"),
        json!({"title":"Hidden","position":0,"kind":"chapter"}),
    )
    .await;
    delete(&app, &format!("{books}/{trashed_book}")).await;
    let trashed_series = create(&app, "/series", json!({"title":"Trash","position":0})).await;
    create(
        &app,
        &format!("/series/{trashed_series}/books"),
        json!({"title":"Hidden","position":0}),
    )
    .await;
    delete(&app, &format!("/series/{trashed_series}")).await;
    let universe = create(&app, "/universes", json!({"title":"World","position":0})).await;
    let containers = format!("/universes/{universe}/containers");
    let container = create(
        &app,
        &containers,
        json!({"title":"Collection","position":0,"kind":"collection"}),
    )
    .await;
    let roots = format!("/containers/{container}/pieces");
    create(
        &app,
        &roots,
        json!({"title":"Plain","position":1,"label":"piece","shape":"leaf"}),
    )
    .await;
    let group = create(
        &app,
        &roots,
        json!({"title":"Group","position":0,"label":"piece","shape":"group"}),
    )
    .await;
    let children = format!("/pieces/{group}/children");
    create(&app, &children, json!({"title":"Later","position":1})).await;
    create(&app, &children, json!({"title":"Earlier","position":0})).await;
    let trashed_piece = create(&app, &children, json!({"title":"Trash","position":0})).await;
    delete(&app, &format!("/pieces/{trashed_piece}")).await;
    create(
        &app,
        &format!("/universes/{universe}/pieces"),
        json!({"title":"Loose","position":0,"label":"piece","shape":"leaf"}),
    )
    .await;
    let trashed_container = create(
        &app,
        &containers,
        json!({"title":"Trash","position":0,"kind":"collection"}),
    )
    .await;
    create(
        &app,
        &format!("/containers/{trashed_container}/pieces"),
        json!({"title":"Hidden","position":0,"label":"piece","shape":"leaf"}),
    )
    .await;
    delete(&app, &format!("/containers/{trashed_container}")).await;

    let mut expected_series = get(&app, "/series").await;
    for series in expected_series.as_array_mut().unwrap() {
        let books_uri = format!("/series/{}/books", series["id"]);
        let mut books = get(&app, &books_uri).await;
        for book in books.as_array_mut().unwrap() {
            book["chapters"] = get(&app, &format!("{books_uri}/{}/chapters", book["id"])).await;
        }
        series["books"] = books;
    }
    let mut universes = get(&app, "/universes").await;
    for universe in universes.as_array_mut().unwrap() {
        let id = universe["id"].as_i64().unwrap();
        let mut containers = get(&app, &format!("/universes/{id}/containers")).await;
        for container in containers.as_array_mut().unwrap() {
            container["pieces"] =
                pieces(&app, &format!("/containers/{}/pieces", container["id"])).await;
        }
        universe["containers"] = containers;
        universe["loose"] = pieces(&app, &format!("/universes/{id}/pieces?container=none")).await;
    }
    assert_eq!(
        get(&app, "/tree").await,
        json!({"series":expected_series,"universes":universes})
    );
}
