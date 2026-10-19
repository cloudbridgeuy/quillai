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

async fn raw_request(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, String) {
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
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

async fn request(app: &Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let (status, body) = raw_request(app, method, uri, &body.to_string()).await;
    (
        status,
        if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&body).unwrap()
        },
    )
}

async fn collection(app: &Router) -> String {
    let (status, series) = request(
        app,
        "POST",
        "/series",
        json!({"title": "Series", "position": 0}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, book) = request(
        app,
        "POST",
        &format!("/series/{}/books", series["id"]),
        json!({"title": "Book", "position": 0}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    format!("/series/{}/books/{}/chapters", series["id"], book["id"])
}

async fn create(app: &Router, collection: &str, kind: &str, position: i64) -> Value {
    let (status, chapter) = request(
        app,
        "POST",
        collection,
        json!({
            "title": "Chapter", "position": position, "kind": kind, "book_id": 999,
            "content": {"ops": [{"insert": "Hello 😀\n", "attributes": {"bold": true}}]}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(chapter["id"].as_i64().unwrap() > 0);
    assert_eq!(chapter["kind"], kind);
    assert_eq!(chapter["position"], position);
    assert_ne!(chapter["book_id"], 999);
    assert_eq!(chapter["head_rev"], 0);
    chapter
}

fn not_found() -> (StatusCode, Value) {
    (
        StatusCode::NOT_FOUND,
        json!({"error": "not found", "code": "NOT_FOUND"}),
    )
}

#[tokio::test]
async fn chapter_crud_prologue_epilogue_and_snapshot_round_trip() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let collection = collection(&app).await;
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([]))
    );
    for kind in ["chapter", "prologue", "epilogue"] {
        let mut chapter = create(&app, &collection, kind, 0).await;
        assert_eq!(
            chapter["content"],
            json!({"ops": [{"insert": "Hello 😀\n", "attributes": {"bold": true}}]})
        );
        let uri = format!("{collection}/{}", chapter["id"]);
        assert_eq!(
            request(&app, "GET", &uri, Value::Null).await,
            (StatusCode::OK, chapter.clone())
        );
        let stored: (String, i64, i64) =
            sqlx::query_as("SELECT content, head_rev, snapshot_rev FROM chapters WHERE id = ?")
                .bind(chapter["id"].as_i64().unwrap())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&stored.0).unwrap(),
            chapter["content"]
        );
        assert_eq!((stored.1, stored.2), (0, 0));
        for update in [
            json!({"title": "Renamed", "book_id": 999}),
            json!({"kind": "epilogue"}),
            json!({"position": 3}),
            json!({}),
            json!({"title": null, "position": null, "kind": null}),
        ] {
            for field in ["title", "position", "kind"] {
                if !update[field].is_null() {
                    chapter[field] = update[field].clone();
                }
            }
            assert_eq!(
                request(&app, "PUT", &uri, update).await,
                (StatusCode::OK, chapter.clone())
            );
            assert_eq!(
                request(&app, "GET", &uri, Value::Null).await,
                (StatusCode::OK, chapter.clone())
            );
        }
        assert_eq!(
            request(&app, "DELETE", &uri, Value::Null).await,
            (StatusCode::NO_CONTENT, Value::Null)
        );
        for method in ["GET", "PUT", "DELETE"] {
            assert_eq!(request(&app, method, &uri, json!({})).await, not_found());
        }
    }
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([]))
    );
}

#[tokio::test]
async fn chapters_order_by_position_then_id_after_reorder() {
    let app = quillai_api::app(test_pool().await);
    let collection = collection(&app).await;
    let late = create(&app, &collection, "epilogue", 9).await;
    let first = create(&app, &collection, "prologue", 1).await;
    let tied = create(&app, &collection, "chapter", 1).await;
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([first, tied, late]))
    );
    let (status, moved) = request(
        &app,
        "PUT",
        &format!("{collection}/{}", late["id"]),
        json!({"position": 0}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([moved, first, tied]))
    );
}

#[tokio::test]
async fn invalid_kind_and_metadata_return_422_before_lookup_without_mutation() {
    let app = quillai_api::app(test_pool().await);
    let collection = collection(&app).await;
    let chapter = create(&app, &collection, "chapter", 0).await;
    let uri = format!("{collection}/{}", chapter["id"]);
    for input in [
        json!({"title": "Chapter", "position": 0, "kind": "invalid"}),
        json!({"title": "Chapter", "position": 0, "kind": "Prologue"}),
        json!({"title": " \t\n", "position": 0, "kind": "chapter"}),
        json!({"title": "Chapter", "position": -1, "kind": "chapter"}),
    ] {
        for (method, path) in [
            ("POST", collection.as_str()),
            ("PUT", uri.as_str()),
            ("POST", "/series/999/books/999/chapters"),
            ("PUT", "/series/999/books/999/chapters/999"),
        ] {
            let (status, body) = request(&app, method, path, input.clone()).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], "VALIDATION_ERROR");
        }
    }
    assert_eq!(
        request(&app, "GET", &uri, Value::Null).await,
        (StatusCode::OK, chapter)
    );
}

#[tokio::test]
async fn put_content_key_returns_422_even_when_null_or_parent_missing() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let collection = collection(&app).await;
    let chapter = create(&app, &collection, "chapter", 0).await;
    let uri = format!("{collection}/{}", chapter["id"]);
    for content in [json!({"ops": []}), Value::Null, json!("bad"), json!(0)] {
        for path in [uri.as_str(), "/series/999/books/999/chapters/999"] {
            let (status, body) = request(
                &app,
                "PUT",
                path,
                json!({"title": "Changed", "content": content}),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], "VALIDATION_ERROR");
        }
    }
    assert_eq!(
        request(&app, "GET", &uri, Value::Null).await,
        (StatusCode::OK, chapter)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM chapter_revisions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn create_rejects_half_emoji_and_accepts_delta_parser_shapes() {
    let app = quillai_api::app(test_pool().await);
    let collection = collection(&app).await;
    for path in [collection.as_str(), "/series/999/books/999/chapters"] {
        let (status, _) = raw_request(&app, "POST", path, r#"{"title":"Chapter","kind":"chapter","position":0,"content":{"ops":[{"insert":"\uD83D"}]}}"#).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([]))
    );
    for content in [
        json!({"ops": [{"retain": 0}, {"insert": ""}]}),
        json!([{ "insert": "Array" }]),
        json!({}),
    ] {
        let expected = quillai_delta::Delta::from_json(&content.to_string()).unwrap();
        let (status, chapter) = request(
            &app,
            "POST",
            &collection,
            json!({"title": "Chapter", "position": 0, "kind": "chapter", "content": content}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(chapter["content"], serde_json::to_value(expected).unwrap());
    }
    let (status, chapter) = request(
        &app,
        "POST",
        &collection,
        json!({"title": "Empty", "position": 0, "kind": "chapter"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(chapter["content"], json!({"ops": []}));
}

#[tokio::test]
async fn missing_and_wrong_parents_and_chapters_return_404() {
    let app = quillai_api::app(test_pool().await);
    let own = collection(&app).await;
    let other = collection(&app).await;
    let chapter = create(&app, &own, "chapter", 0).await;
    let parts: Vec<_> = own.split('/').collect();
    let wrong_series = format!("/series/999/books/{}/chapters", parts[4]);
    let wrong_book = format!("/series/{}/books/999/chapters", parts[2]);
    for path in [
        "/series/999/books/999/chapters",
        wrong_series.as_str(),
        wrong_book.as_str(),
    ] {
        for method in ["GET", "POST"] {
            assert_eq!(
                request(
                    &app,
                    method,
                    path,
                    json!({"title": "Chapter", "position": 0, "kind": "chapter"})
                )
                .await,
                not_found()
            );
        }
    }
    for path in [
        other.as_str(),
        wrong_series.as_str(),
        wrong_book.as_str(),
        own.as_str(),
    ] {
        let id = if path == own {
            json!(999)
        } else {
            chapter["id"].clone()
        };
        for method in ["GET", "PUT", "DELETE"] {
            assert_eq!(
                request(
                    &app,
                    method,
                    &format!("{path}/{id}"),
                    json!({"title": "Wrong"})
                )
                .await,
                not_found()
            );
        }
    }
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("{own}/{}", chapter["id"]),
            Value::Null
        )
        .await,
        (StatusCode::OK, chapter)
    );
}

#[tokio::test]
async fn reads_return_chapters_head_rev_not_snapshot_rev() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let collection = collection(&app).await;
    let mut chapter = create(&app, &collection, "chapter", 0).await;
    sqlx::query("UPDATE chapters SET head_rev = 4 WHERE id = ?")
        .bind(chapter["id"].as_i64().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    chapter["head_rev"] = json!(4);
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("{collection}/{}", chapter["id"]),
            Value::Null
        )
        .await,
        (StatusCode::OK, chapter.clone())
    );
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([chapter]))
    );
}

#[tokio::test]
async fn unrelated_database_failures_remain_500() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let collection = collection(&app).await;
    pool.close().await;
    for (method, uri) in [
        ("GET", collection.clone()),
        ("POST", collection.clone()),
        ("GET", format!("{collection}/1")),
        ("PUT", format!("{collection}/1")),
        ("DELETE", format!("{collection}/1")),
    ] {
        let (status, body) = request(
            &app,
            method,
            &uri,
            json!({"title": "Chapter", "position": 0, "kind": "chapter"}),
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "SQLX");
    }
}
