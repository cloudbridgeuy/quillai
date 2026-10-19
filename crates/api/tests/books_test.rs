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

async fn create_series(app: &Router) -> String {
    let (status, series) = request(
        app,
        "POST",
        "/series",
        json!({"title": "Series", "position": 0}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    format!("/series/{}/books", series["id"])
}

async fn create_book(app: &Router, collection: &str, title: &str, position: i64) -> Value {
    let (status, book) = request(
        app,
        "POST",
        collection,
        json!({"title": title, "position": position, "series_id": 999}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(book["id"].as_i64().unwrap() > 0);
    assert_eq!(book["title"], title);
    assert_eq!(book["position"], position);
    assert_eq!(collection, format!("/series/{}/books", book["series_id"]));
    book
}

fn not_found() -> (StatusCode, Value) {
    (
        StatusCode::NOT_FOUND,
        json!({"error": "not found", "code": "NOT_FOUND"}),
    )
}

#[tokio::test]
async fn book_crud_partial_updates_and_persistence() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let collection = create_series(&app).await;
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([]))
    );
    let mut book = create_book(&app, &collection, "Original", 0).await;
    let uri = format!("{collection}/{}", book["id"]);
    for update in [
        json!({"title": "Renamed", "series_id": 999}),
        json!({"position": 3}),
        json!({"title": "Final", "position": 1}),
        json!({}),
        json!({"title": null, "position": null}),
    ] {
        for field in ["title", "position"] {
            if !update[field].is_null() {
                book[field] = update[field].clone();
            }
        }
        assert_eq!(
            request(&app, "PUT", &uri, update).await,
            (StatusCode::OK, book.clone())
        );
        assert_eq!(
            request(&app, "GET", &uri, Value::Null).await,
            (StatusCode::OK, book.clone())
        );
    }
    let stored: (i64, String, i64) =
        sqlx::query_as("SELECT series_id, title, position FROM books WHERE id = ?")
            .bind(book["id"].as_i64().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        stored,
        (book["series_id"].as_i64().unwrap(), "Final".into(), 1)
    );
    assert_eq!(
        request(&app, "DELETE", &uri, Value::Null).await,
        (StatusCode::NO_CONTENT, Value::Null)
    );
    assert_eq!(request(&app, "GET", &uri, Value::Null).await, not_found());
    assert_eq!(
        request(&app, "DELETE", &uri, Value::Null).await,
        not_found()
    );
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([]))
    );
}

#[tokio::test]
async fn missing_parent_and_missing_book_return_json_404() {
    let app = quillai_api::app(test_pool().await);
    for method in ["GET", "POST"] {
        assert_eq!(
            request(
                &app,
                method,
                "/series/999/books",
                json!({"title": "Book", "position": 0})
            )
            .await,
            not_found()
        );
    }
    let collection = create_series(&app).await;
    for collection in [collection.as_str(), "/series/999/books"] {
        for method in ["GET", "PUT", "DELETE"] {
            assert_eq!(
                request(
                    &app,
                    method,
                    &format!("{collection}/999"),
                    json!({"title": "Book"})
                )
                .await,
                not_found()
            );
        }
    }
}

#[tokio::test]
async fn books_cannot_be_accessed_through_another_series() {
    let app = quillai_api::app(test_pool().await);
    let own = create_series(&app).await;
    let other = create_series(&app).await;
    let book = create_book(&app, &own, "Private", 0).await;
    for collection in [other.as_str(), "/series/999/books"] {
        for method in ["GET", "PUT", "DELETE"] {
            assert_eq!(
                request(
                    &app,
                    method,
                    &format!("{collection}/{}", book["id"]),
                    json!({"title": "Wrong"})
                )
                .await,
                not_found()
            );
        }
    }
    assert_eq!(
        request(&app, "GET", &other, Value::Null).await,
        (StatusCode::OK, json!([]))
    );
    assert_eq!(
        request(&app, "GET", &format!("{own}/{}", book["id"]), Value::Null).await,
        (StatusCode::OK, book)
    );
}

#[tokio::test]
async fn invalid_books_return_422_without_mutation() {
    let app = quillai_api::app(test_pool().await);
    let collection = create_series(&app).await;
    let book = create_book(&app, &collection, "Unchanged", 0).await;
    let uri = format!("{collection}/{}", book["id"]);
    for input in [
        json!({"title": "", "position": 0}),
        json!({"title": " \t\n", "position": 0}),
        json!({"title": "Valid", "position": -1}),
    ] {
        for (method, path) in [("POST", collection.as_str()), ("PUT", uri.as_str())] {
            let (status, body) = request(&app, method, path, input.clone()).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], "VALIDATION_ERROR");
            assert!(body["error"]
                .as_str()
                .unwrap()
                .starts_with("validation error:"));
        }
    }
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([book]))
    );
}

#[tokio::test]
async fn books_order_by_position_then_id_after_update() {
    let app = quillai_api::app(test_pool().await);
    let collection = create_series(&app).await;
    let late = create_book(&app, &collection, "Late", 9).await;
    let first = create_book(&app, &collection, "First", 1).await;
    let tied = create_book(&app, &collection, "Tied", 1).await;
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
async fn deleting_book_cascades_chapters() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let collection = create_series(&app).await;
    let book = create_book(&app, &collection, "Removed", 0).await;
    let chapter_id: i64 = sqlx::query_scalar(
        "INSERT INTO chapters (book_id, title, position, kind) VALUES (?, 'Chapter', 0, 'chapter') RETURNING id",
    )
    .bind(book["id"].as_i64().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        request(
            &app,
            "DELETE",
            &format!("{collection}/{}", book["id"]),
            Value::Null
        )
        .await,
        (StatusCode::NO_CONTENT, Value::Null)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM chapters WHERE id = ?")
            .bind(chapter_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn deleting_series_cascades_books() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let collection = create_series(&app).await;
    let book = create_book(&app, &collection, "Removed", 0).await;
    assert_eq!(
        request(
            &app,
            "DELETE",
            &format!("/series/{}", book["series_id"]),
            Value::Null
        )
        .await,
        (StatusCode::NO_CONTENT, Value::Null)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM books")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    for (method, uri) in [
        ("GET", collection.clone()),
        ("POST", collection.clone()),
        ("GET", format!("{collection}/{}", book["id"])),
        ("PUT", format!("{collection}/{}", book["id"])),
        ("DELETE", format!("{collection}/{}", book["id"])),
    ] {
        assert_eq!(
            request(&app, method, &uri, json!({"title": "Book", "position": 0})).await,
            not_found()
        );
    }
}

#[tokio::test]
async fn unrelated_database_failures_remain_500() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let collection = create_series(&app).await;
    pool.close().await;
    for (method, uri) in [
        ("GET", collection.clone()),
        ("POST", collection.clone()),
        ("GET", format!("{collection}/1")),
        ("PUT", format!("{collection}/1")),
        ("DELETE", format!("{collection}/1")),
    ] {
        let (status, body) =
            request(&app, method, &uri, json!({"title": "Book", "position": 0})).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "SQLX");
    }
}
