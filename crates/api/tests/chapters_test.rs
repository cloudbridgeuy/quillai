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
async fn create_rejects_half_emoji_and_invalid_delta_shapes() {
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
        json!({"ops": []}),
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
    for content in [json!({}), json!([]), json!(null), json!({"ops": [5]})] {
        let (status, _) = request(
            &app,
            "POST",
            &collection,
            json!({"title": "Chapter", "position": 0, "kind": "chapter", "content": content}),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
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
    let store = quillai_api::chapter_revisions::RevisionStore::new(
        pool.clone(),
        std::num::NonZeroU64::new(100).unwrap(),
    );
    for rev in 0..4 {
        store
            .append(
                chapter["id"].as_i64().unwrap(),
                rev,
                &quillai_delta::Delta::new(),
            )
            .await
            .unwrap();
    }
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
async fn corrupt_stored_content_returns_500_sqlx_for_list_and_detail() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let collection = collection(&app).await;
    let chapter = create(&app, &collection, "chapter", 0).await;
    for content in [r#"{"ops":[{"insert":"\uD83D"}]}"#, "{invalid"] {
        sqlx::query("PRAGMA ignore_check_constraints = ON")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE chapters SET content = ? WHERE id = ?")
            .bind(content)
            .bind(chapter["id"].as_i64().unwrap())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("PRAGMA ignore_check_constraints = OFF")
            .execute(&pool)
            .await
            .unwrap();
        for uri in [
            collection.clone(),
            format!("{collection}/{}", chapter["id"]),
        ] {
            let (status, body) = request(&app, "GET", &uri, Value::Null).await;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body["code"], "SQLX");
        }
    }
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

fn delta(value: &Value) -> quillai_delta::Delta {
    serde_json::from_value(value.clone()).unwrap()
}

async fn sync_chapter(app: &Router, content: Value) -> (String, String, i64) {
    let collection = collection(app).await;
    let (status, chapter) = request(
        app,
        "POST",
        &collection,
        json!({"title": "Chapter", "position": 0, "kind": "chapter", "content": content}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = chapter["id"].as_i64().unwrap();
    (collection.clone(), format!("{collection}/{id}"), id)
}

async fn persisted(pool: &sqlx::SqlitePool, id: i64) -> (i64, i64, String, Vec<(i64, String)>) {
    let (head, snapshot, content) = sqlx::query_as::<_, (i64, i64, String)>(
        "SELECT head_rev, snapshot_rev, content FROM chapters WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    let revisions = sqlx::query_as(
        "SELECT rev, delta FROM chapter_revisions WHERE chapter_id = ? ORDER BY rev",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .unwrap();
    (head, snapshot, content, revisions)
}

#[tokio::test]
async fn sync_in_order_and_same_base_transform_converge_and_reads_rebuild() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let initial = json!({"ops": [{"insert": "Hi 😀\n"}]});
    let (collection, chapter, id) = sync_chapter(&app, initial.clone()).await;
    let uri = format!("{chapter}/deltas");
    let first = json!({"ops": [{"insert": "A"}]});
    let second = json!({"ops": [{"insert": "B"}]});
    let (status, a) = request(&app, "POST", &uri, json!({"baseRev": 0, "delta": first})).await;
    assert_eq!(
        (status, &a),
        (StatusCode::OK, &json!({"rev": 1, "delta": first}))
    );
    let (status, b) = request(&app, "POST", &uri, json!({"baseRev": 0, "delta": second})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        b,
        json!({"rev": 2, "delta": {"ops": [{"retain": 1}, {"insert": "B"}]}})
    );
    let client_a = delta(&initial)
        .compose(&delta(&first))
        .unwrap()
        .compose(&delta(&b["delta"]))
        .unwrap();
    let client_b = delta(&initial)
        .compose(&delta(&second))
        .unwrap()
        .compose(&delta(&second).transform(&delta(&first), false))
        .unwrap();
    assert_eq!(client_a, client_b);
    let third = json!({"ops": [{"retain": 2}, {"insert": "C"}]});
    let (status, c) = request(&app, "POST", &uri, json!({"baseRev": 2, "delta": third})).await;
    assert_eq!(
        (status, &c),
        (StatusCode::OK, &json!({"rev": 3, "delta": third}))
    );
    assert_eq!(
        request(&app, "GET", &format!("{uri}?since=0"), Value::Null).await,
        (StatusCode::OK, json!([a, b, c]))
    );
    assert_eq!(
        request(&app, "GET", &format!("{uri}?since=1"), Value::Null).await,
        (StatusCode::OK, json!([b, c]))
    );
    assert_eq!(
        request(&app, "GET", &format!("{uri}?since=3"), Value::Null).await,
        (StatusCode::OK, json!([]))
    );
    let expected = serde_json::to_value(client_a.compose(&delta(&third)).unwrap()).unwrap();
    for method in ["GET", "PUT"] {
        let (status, read) = request(&app, method, &chapter, json!({"title": "Renamed"})).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(read["head_rev"], 3);
        assert_eq!(read["content"], expected);
    }
    let (status, list) = request(&app, "GET", &collection, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list[0]["head_rev"], 3);
    assert_eq!(list[0]["content"], expected);
    let stored = persisted(&pool, id).await;
    assert_eq!((stored.0, stored.1, stored.3.len()), (3, 0, 3));
    assert_eq!(serde_json::from_str::<Value>(&stored.2).unwrap(), initial);
}

#[tokio::test]
async fn sync_bad_shapes_json_and_compose_errors_return_422_without_writes() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let (_, chapter, id) = sync_chapter(&app, json!({"ops": [{"insert": "😀"}]})).await;
    let uri = format!("{chapter}/deltas");
    let before = persisted(&pool, id).await;
    for bad in [
        json!(null),
        json!(1),
        json!("bad"),
        json!([]),
        json!({}),
        json!({"ops": null}),
        json!({"ops": {}}),
        json!({"ops": [null]}),
        json!({"ops": [{}]}),
        json!({"ops": [{"insert": 5}]}),
        json!({"ops": [{"retain": -1}]}),
        json!({"ops": [{"delete": 1.5}]}),
        json!({"ops": [{"insert": "x", "delete": 1}]}),
        json!({"ops": [{"insert": "x", "attributes": []}]}),
        json!({"ops": [{"insert": "x", "unknown": true}]}),
        json!({"ops": [{"retain": 9007199254740992_u64}]}),
        json!({"ops": [{"delete": 1}]}),
        json!({"ops": [{"retain": 1}, {"insert": "x"}]}),
    ] {
        let (status, body) = request(&app, "POST", &uri, json!({"baseRev": 0, "delta": bad})).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}: {body}");
        assert_eq!(body["code"], "VALIDATION_ERROR");
        assert_eq!(persisted(&pool, id).await, before);
    }
    for bad in [
        "{",
        r#"{"baseRev":0}"#,
        r#"{"baseRev":"0","delta":{"ops":[]}}"#,
        r#"{"baseRev":0,"delta":{"ops":[{"insert":"\ud83d"}]}}"#,
    ] {
        assert_eq!(
            raw_request(&app, "POST", &uri, bad).await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(persisted(&pool, id).await, before);
    }
    let valid = json!({"ops": [{"insert": "X"}]});
    assert_eq!(
        request(&app, "POST", &uri, json!({"baseRev": 0, "delta": valid}))
            .await
            .0,
        StatusCode::OK
    );
    let before = persisted(&pool, id).await;
    let invalid_after_transform = json!({"ops": [{"delete": 1}]});
    assert_eq!(
        request(
            &app,
            "POST",
            &uri,
            json!({"baseRev": 0, "delta": invalid_after_transform})
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(persisted(&pool, id).await, before);
    let (_, embed_chapter, embed_id) =
        sync_chapter(&app, json!({"ops": [{"insert": {"image": "a"}}]})).await;
    let before = persisted(&pool, embed_id).await;
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("{embed_chapter}/deltas"),
            json!({"baseRev": 0, "delta": {"ops": [{"retain": {"image": "b"}}]}})
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(persisted(&pool, embed_id).await, before);
}

#[tokio::test]
async fn sync_retained_floor_and_gaps_and_future_revisions_return_409() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let (_, chapter, id) = sync_chapter(&app, json!({"ops": []})).await;
    let uri = format!("{chapter}/deltas");
    let store = quillai_api::chapter_revisions::RevisionStore::new(
        pool.clone(),
        std::num::NonZeroU64::new(1).unwrap(),
    );
    for rev in 0..3 {
        store
            .append(id, rev, &delta(&json!({"ops": [{"insert": "x"}]})))
            .await
            .unwrap();
    }
    assert_eq!(
        request(&app, "GET", &format!("{uri}?since=0"), Value::Null)
            .await
            .1
            .as_array()
            .unwrap()
            .len(),
        3
    );
    sqlx::query("DELETE FROM chapter_revisions WHERE chapter_id = ? AND rev = 1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let before = persisted(&pool, id).await;
    for rev in [-1, 0, 4] {
        for (method, path, body) in [
            ("GET", format!("{uri}?since={rev}"), Value::Null),
            (
                "POST",
                uri.clone(),
                json!({"baseRev": rev, "delta": {"ops": []}}),
            ),
        ] {
            let (status, body) = request(&app, method, &path, body).await;
            assert_eq!(status, StatusCode::CONFLICT);
            assert_eq!(body["code"], "CONFLICT");
            assert_eq!(persisted(&pool, id).await, before);
        }
    }
    assert_eq!(
        request(&app, "GET", &format!("{uri}?since=1"), Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    sqlx::query("DELETE FROM chapter_revisions WHERE chapter_id = ? AND rev = 2")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(&app, "GET", &format!("{uri}?since=1"), Value::Null)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(&app, "GET", &format!("{uri}?since=2"), Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &uri,
            json!({"baseRev": 2, "delta": {"ops": [{"insert": "y"}]}})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "GET", &chapter, Value::Null).await.1["content"],
        json!({"ops": [{"insert": "xyxx"}]})
    );
}

#[tokio::test]
async fn sync_parent_scope_and_missing_chapters_return_404() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let (own, _, id) = sync_chapter(&app, json!({"ops": []})).await;
    let other = collection(&app).await;
    let parts: Vec<_> = own.split('/').collect();
    for uri in [
        format!("{other}/{id}/deltas"),
        format!("/series/999/books/{}/chapters/{id}/deltas", parts[4]),
        format!("/series/{}/books/999/chapters/{id}/deltas", parts[2]),
        format!("{own}/999/deltas"),
    ] {
        assert_eq!(
            request(&app, "GET", &format!("{uri}?since=0"), Value::Null).await,
            not_found()
        );
        assert_eq!(
            request(
                &app,
                "POST",
                &uri,
                json!({"baseRev": 0, "delta": {"ops": []}})
            )
            .await,
            not_found()
        );
    }
    assert_eq!(persisted(&pool, id).await.0, 0);
}

#[tokio::test]
async fn simultaneous_sync_requests_both_append_and_converge() {
    let path = std::env::temp_dir().join(format!(
        "guz705-sync-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .unwrap();
    quillai_api::database::migrate(&pool).await.unwrap();
    let app = quillai_api::app(pool.clone());
    let initial = json!({"ops": [{"insert": "😀\n"}]});
    let (_, chapter, id) = sync_chapter(&app, initial.clone()).await;
    let uri = format!("{chapter}/deltas");
    for base in (0..20).step_by(2) {
        let first = json!({"ops": [{"insert": "A"}]});
        let second = json!({"ops": [{"insert": "B"}]});
        let previous = request(&app, "GET", &chapter, Value::Null).await.1["content"].clone();
        let (a, b) = tokio::join!(
            request(&app, "POST", &uri, json!({"baseRev": base, "delta": first})),
            request(
                &app,
                "POST",
                &uri,
                json!({"baseRev": base, "delta": second})
            ),
        );
        assert_eq!(a.0, StatusCode::OK, "{}", a.1);
        assert_eq!(b.0, StatusCode::OK, "{}", b.1);
        let a_wins = a.1["rev"].as_i64().unwrap() < b.1["rev"].as_i64().unwrap();
        let on_a = delta(&previous)
            .compose(&delta(&first))
            .unwrap()
            .compose(&delta(&first).transform(&delta(&second), a_wins))
            .unwrap();
        let on_b = delta(&previous)
            .compose(&delta(&second))
            .unwrap()
            .compose(&delta(&second).transform(&delta(&first), !a_wins))
            .unwrap();
        assert_eq!(on_a, on_b);
        let (status, read) = request(&app, "GET", &chapter, Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(read["head_rev"], base + 2);
        assert_eq!(delta(&read["content"]), on_a);
        let missed = request(&app, "GET", &format!("{uri}?since={base}"), Value::Null).await;
        assert_eq!(missed.0, StatusCode::OK);
        let mut applied = delta(&previous);
        for revision in missed.1.as_array().unwrap() {
            applied = applied.compose(&delta(&revision["delta"])).unwrap();
        }
        assert_eq!(applied, on_a);
    }
    assert_eq!(persisted(&pool, id).await.3.len(), 20);
    pool.close().await;
    std::fs::remove_file(path).unwrap();
}
