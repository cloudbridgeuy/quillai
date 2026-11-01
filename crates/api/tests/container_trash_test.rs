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

async fn create(app: &Router, uri: &str, body: Value) -> Value {
    let (status, value) = request(app, "POST", uri, body).await;
    assert_eq!(status, StatusCode::CREATED, "{uri}: {value}");
    value
}

async fn expect(app: &Router, method: &str, path: &str, expected: StatusCode) {
    assert_eq!(
        request(app, method, path, Value::Null).await.0,
        expected,
        "{method} {path}"
    );
}

#[tokio::test]
async fn placement_into_deleted_container_preserves_source() {
    let pool = common::test_pool().await;
    let app = quillai_api::app(pool);
    let universe = create(&app, "/universes", json!({"title":"World", "position":0})).await;
    let universe_path = format!("/universes/{}", universe["id"]);
    let container = create(
        &app,
        &format!("{universe_path}/containers"),
        json!({"title":"Collection", "position":0, "kind":"collection"}),
    )
    .await;
    let piece = create(
        &app,
        &format!("{universe_path}/pieces"),
        json!({"title":"Loose", "position":3, "label":"short_story", "shape":"leaf"}),
    )
    .await;
    let piece_path = format!("/pieces/{}", piece["id"]);
    let before = request(&app, "GET", &piece_path, Value::Null).await;
    expect(
        &app,
        "DELETE",
        &format!("/containers/{}", container["id"]),
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("{piece_path}/placement"),
            json!({"containerId":container["id"], "position":0})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(request(&app, "GET", &piece_path, Value::Null).await, before);
}

#[tokio::test]
async fn permanent_series_delete_cleans_only_empty_universes() {
    for retained in ["none", "piece", "container"] {
        let pool = common::test_pool().await;
        let app = quillai_api::app(pool.clone());
        let series = create(&app, "/series", json!({"title":"Series", "position":0})).await;
        let universe_id: i64 =
            sqlx::query_scalar("SELECT universe_id FROM containers WHERE id = ?")
                .bind(series["id"].as_i64().unwrap())
                .fetch_one(&pool)
                .await
                .unwrap();
        let universe_path = format!("/universes/{universe_id}");
        create(
            &app,
            &format!("{universe_path}/lore"),
            json!({"kind":"note", "name":"Lore"}),
        )
        .await;
        let retained_path = match retained {
            "piece" => {
                let piece = create(
                    &app,
                    &format!("{universe_path}/pieces"),
                    json!({"title":"Loose", "position":0, "label":"short_story", "shape":"leaf"}),
                )
                .await;
                Some(format!("/pieces/{}", piece["id"]))
            }
            "container" => {
                let container = create(
                    &app,
                    &format!("{universe_path}/containers"),
                    json!({"title":"Other", "position":1, "kind":"collection"}),
                )
                .await;
                Some(format!("/containers/{}", container["id"]))
            }
            _ => None,
        };
        if let Some(path) = retained_path {
            expect(&app, "DELETE", &path, StatusCode::NO_CONTENT).await;
        }
        expect(
            &app,
            "DELETE",
            &format!("/series/{}?permanent=true", series["id"]),
            StatusCode::NO_CONTENT,
        )
        .await;
        let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM universes WHERE id = ?), (SELECT COUNT(*) FROM lore_entries WHERE universe_id = ?)")
            .bind(universe_id).bind(universe_id).fetch_one(&pool).await.unwrap();
        let expected = i64::from(retained != "none");
        assert_eq!(counts, (expected, expected), "{retained}");
    }
}

#[tokio::test]
async fn container_and_universe_trash_lifecycle() {
    let pool = common::test_pool().await;
    let app = quillai_api::app(pool.clone());
    let universe = create(&app, "/universes", json!({"title":"World", "position":0})).await;
    let containers = format!("/universes/{}/containers", universe["id"]);
    let series = create(&app, "/series", json!({"title":"Series", "position":0})).await;
    let books = format!("/series/{}/books", series["id"]);
    let book = create(&app, &books, json!({"title":"Book", "position":0})).await;
    let collection_body = json!({"title":"Collection", "position":0, "kind":"collection"});
    let collection = create(&app, &containers, collection_body).await;
    create(&app, &books, json!({"title":"Sibling", "position":5})).await;
    create(
        &app,
        &containers,
        json!({"title":"Sibling", "position":5, "kind":"collection"}),
    )
    .await;
    let series_universe: i64 =
        sqlx::query_scalar("SELECT universe_id FROM containers WHERE id = ?")
            .bind(series["id"].as_i64().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    create(
        &app,
        &format!("/universes/{series_universe}/containers"),
        json!({"title":"Sibling", "position":5, "kind":"book_series"}),
    )
    .await;
    create(&app, "/universes", json!({"title":"Sibling", "position":5})).await;
    for path in [
        format!("{books}/{}", book["id"]),
        format!("/series/{}", series["id"]),
        format!("/containers/{}", collection["id"]),
        format!("/universes/{}", universe["id"]),
    ] {
        expect(&app, "DELETE", &path, StatusCode::NO_CONTENT).await;
        expect(&app, "DELETE", &path, StatusCode::NOT_FOUND).await;
        let restore = format!("{path}/restore");
        let (status, restored) = request(&app, "POST", &restore, Value::Null).await;
        assert_eq!(status, StatusCode::OK, "{path}: {restored}");
        assert_eq!(restored["position"], 6, "{path}");
        expect(&app, "POST", &restore, StatusCode::NOT_FOUND).await;
        expect(&app, "DELETE", &path, StatusCode::NO_CONTENT).await;
        expect(
            &app,
            "DELETE",
            &format!("{path}?permanent=true"),
            StatusCode::NO_CONTENT,
        )
        .await;
        expect(&app, "POST", &restore, StatusCode::NOT_FOUND).await;
    }
}

#[tokio::test]
async fn restoring_chapter_restores_ancestors_but_not_deleted_siblings() {
    let pool = common::test_pool().await;
    let app = quillai_api::app(pool.clone());
    let series = create(&app, "/series", json!({"title":"Series", "position":0})).await;
    let series_path = format!("/series/{}", series["id"]);
    let books = format!("{series_path}/books");
    let book = create(&app, &books, json!({"title":"Book", "position":0})).await;
    let book_path = format!("{books}/{}", book["id"]);
    let chapters = format!("{book_path}/chapters");
    let chapter_body =
        json!({"title":"Chapter", "position":0, "kind":"chapter", "content":{"ops":[]}});
    let chapter = create(&app, &chapters, chapter_body.clone()).await;
    let deleted = create(&app, &chapters, chapter_body).await;
    let chapter_path = format!("{chapters}/{}", chapter["id"]);
    let deleted_path = format!("{chapters}/{}", deleted["id"]);
    let (universe_id, book_piece, chapter_piece): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT universe_id FROM containers WHERE id = ?), (SELECT id FROM pieces WHERE legacy_book_id = ?), (SELECT id FROM pieces WHERE legacy_chapter_id = ?)")
        .bind(series["id"].as_i64().unwrap()).bind(book["id"].as_i64().unwrap()).bind(chapter["id"].as_i64().unwrap()).fetch_one(&pool).await.unwrap();
    let universe_path = format!("/universes/{universe_id}");
    create(&app, &books, json!({"title":"Sibling", "position":5})).await;
    create(
        &app,
        &format!("{universe_path}/containers"),
        json!({"title":"Sibling", "position":5, "kind":"book_series"}),
    )
    .await;
    create(&app, "/universes", json!({"title":"Sibling", "position":5})).await;
    let lore_path = format!("{universe_path}/lore");
    let lore = create(&app, &lore_path, json!({"kind":"note", "name":"Lore"})).await;
    expect(&app, "DELETE", &deleted_path, StatusCode::NO_CONTENT).await;
    expect(&app, "DELETE", &book_path, StatusCode::NO_CONTENT).await;
    expect(&app, "GET", &chapter_path, StatusCode::NOT_FOUND).await;
    expect(
        &app,
        "GET",
        &format!("/pieces/{chapter_piece}"),
        StatusCode::NOT_FOUND,
    )
    .await;
    expect(&app, "DELETE", &series_path, StatusCode::NO_CONTENT).await;
    for path in [
        &books,
        &format!("/containers/{}/pieces", series["id"]),
        &format!("{series_path}/lore"),
    ] {
        expect(&app, "GET", path, StatusCode::NOT_FOUND).await;
    }
    expect(&app, "GET", &lore_path, StatusCode::OK).await;
    expect(&app, "DELETE", &universe_path, StatusCode::NO_CONTENT).await;
    let timestamps: (Option<String>, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT (SELECT deleted_at FROM universes WHERE id = ?), (SELECT deleted_at FROM containers WHERE id = ?), (SELECT deleted_at FROM pieces WHERE id = ?), (SELECT deleted_at FROM pieces WHERE id = ?)")
        .bind(universe_id).bind(series["id"].as_i64().unwrap()).bind(book_piece).bind(chapter_piece).fetch_one(&pool).await.unwrap();
    assert!(
        timestamps.0.is_some()
            && timestamps.1.is_some()
            && timestamps.2.is_some()
            && timestamps.3.is_none()
    );
    for path in [
        series_path.clone(),
        books.clone(),
        book_path.clone(),
        chapters.clone(),
        chapter_path.clone(),
        format!("/pieces/{book_piece}"),
        format!("/pieces/{chapter_piece}"),
        format!("{universe_path}/pieces?container=none"),
        format!("{universe_path}/containers"),
        lore_path.clone(),
        format!("{lore_path}/{}", lore["id"]),
    ] {
        expect(&app, "GET", &path, StatusCode::NOT_FOUND).await;
    }
    let piece_path = format!("/pieces/{chapter_piece}");
    let child_path = format!("/pieces/{book_piece}/children");
    let lore_entry = format!("{lore_path}/{}", lore["id"]);
    for (method, path, body) in [
        ("PUT", piece_path.clone(), json!({"title":"Changed"})),
        (
            "POST",
            format!("{piece_path}/placement"),
            json!({"containerId":null,"position":0}),
        ),
        ("POST", child_path, json!({"title":"New", "position":0})),
        (
            "POST",
            lore_path.clone(),
            json!({"kind":"note", "name":"New"}),
        ),
        ("PUT", lore_entry.clone(), json!({"name":"Changed"})),
    ] {
        assert_eq!(
            request(&app, method, &path, body).await.0,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    expect(
        &app,
        "POST",
        &format!("{chapter_path}/restore"),
        StatusCode::OK,
    )
    .await;
    let (_, restored_chapter) = request(&app, "GET", &chapter_path, Value::Null).await;
    assert_eq!(restored_chapter["position"], 0);
    for path in [&universe_path, &series_path, &book_path] {
        let (status, restored) = request(&app, "GET", path, Value::Null).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(restored["position"], 6, "{path}");
    }
    expect(&app, "GET", &deleted_path, StatusCode::NOT_FOUND).await;
    expect(&app, "GET", &lore_entry, StatusCode::OK).await;
}
