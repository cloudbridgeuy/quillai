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

async fn create_series(app: &Router) -> i64 {
    let (status, body) = request(
        app,
        "POST",
        "/series",
        json!({"title": "Series", "position": 0}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    body["id"].as_i64().unwrap()
}

async fn create_lore(app: &Router, series_id: i64, input: Value) -> Value {
    let (status, body) = request(app, "POST", &format!("/series/{series_id}/lore"), input).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert!(body["id"].as_i64().unwrap() > 0);
    assert_eq!(body["series_id"], series_id);
    body
}

#[tokio::test]
async fn lore_crud_defaults_partial_updates_and_storage() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let series_id = create_series(&app).await;
    let collection = format!("/series/{series_id}/lore");
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([]))
    );
    let mut entry = create_lore(&app, series_id, json!({"kind": "character", "name": "Ada"})).await;
    let id = entry["id"].as_i64().unwrap();
    let uri = format!("{collection}/{id}");
    assert_eq!(
        entry,
        json!({"id": id, "series_id": series_id, "kind": "character", "name": "Ada",
            "keywords": [], "aliases": [], "visible_text": "", "hidden_notes": "",
            "pinned": false, "level": "include"})
    );
    let fields = [
        "kind",
        "name",
        "keywords",
        "aliases",
        "visible_text",
        "hidden_notes",
        "pinned",
        "level",
    ];
    for update in [
        json!({"kind": "place", "name": "Library", "keywords": ["books", "📚"],
            "aliases": ["Archive"], "visible_text": "Public lore", "hidden_notes": "Author secret",
            "pinned": true, "level": "known_dont_reveal"}),
        json!({}),
        json!({"kind": null, "name": null, "keywords": null, "aliases": null,
            "visible_text": null, "hidden_notes": null, "pinned": null, "level": null}),
        json!({"kind": "term", "level": "leave_out", "pinned": false}),
        json!({"kind": "note", "level": "include", "keywords": [], "aliases": [],
            "visible_text": "", "hidden_notes": ""}),
    ] {
        for field in fields {
            if !update[field].is_null() {
                entry[field] = update[field].clone();
            }
        }
        assert_eq!(
            request(&app, "PUT", &uri, update).await,
            (StatusCode::OK, entry.clone())
        );
        assert_eq!(
            request(&app, "GET", &uri, Value::Null).await,
            (StatusCode::OK, entry.clone())
        );
        assert_eq!(
            request(&app, "GET", &collection, Value::Null).await,
            (StatusCode::OK, json!([entry]))
        );
        let row: (String, String, i64) =
            sqlx::query_as("SELECT keywords, aliases, pinned FROM lore_entries WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&row.0).unwrap(),
            entry["keywords"]
        );
        assert_eq!(
            serde_json::from_str::<Value>(&row.1).unwrap(),
            entry["aliases"]
        );
        assert_eq!(row.2, i64::from(entry["pinned"].as_bool().unwrap()));
    }
    assert_eq!(
        request(&app, "DELETE", &uri, Value::Null).await,
        (StatusCode::NO_CONTENT, Value::Null)
    );
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([]))
    );
    assert_eq!(
        request(&app, "GET", &uri, Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn create_accepts_all_fields_and_null_optional_defaults() {
    let app = quillai_api::app(test_pool().await);
    let series_id = create_series(&app).await;
    let input = json!({"kind": "place", "name": "Archive", "keywords": ["books"],
        "aliases": ["Library"], "visible_text": "Open daily", "hidden_notes": "Secret door",
        "pinned": true, "level": "known_dont_reveal"});
    let entry = create_lore(&app, series_id, input.clone()).await;
    for (field, value) in input.as_object().unwrap() {
        assert_eq!(&entry[field], value);
    }
    let defaults = create_lore(
        &app,
        series_id,
        json!({"kind": "note", "name": "Defaults",
        "keywords": null, "aliases": null, "visible_text": null, "hidden_notes": null,
        "pinned": null, "level": null}),
    )
    .await;
    assert_eq!(defaults["keywords"], json!([]));
    assert_eq!(defaults["aliases"], json!([]));
    assert_eq!(defaults["visible_text"], "");
    assert_eq!(defaults["hidden_notes"], "");
    assert_eq!(defaults["pinned"], false);
    assert_eq!(defaults["level"], "include");
}

#[tokio::test]
async fn missing_parents_details_and_wrong_series_return_json_404() {
    let app = quillai_api::app(test_pool().await);
    let first = create_series(&app).await;
    let other = create_series(&app).await;
    let entry = create_lore(&app, first, json!({"kind": "note", "name": "Private"})).await;
    let expected = (
        StatusCode::NOT_FOUND,
        json!({"error": "not found", "code": "NOT_FOUND"}),
    );
    for method in ["GET", "POST"] {
        assert_eq!(
            request(
                &app,
                method,
                "/series/999/lore",
                json!({"kind": "note", "name": "Valid"})
            )
            .await,
            expected
        );
    }
    for path in [
        format!("/series/{other}/lore/{}", entry["id"]),
        format!("/series/{first}/lore/999"),
        format!("/series/999/lore/{}", entry["id"]),
    ] {
        for method in ["GET", "PUT", "DELETE"] {
            assert_eq!(request(&app, method, &path, json!({})).await, expected);
        }
    }
    assert_eq!(
        request(&app, "GET", &format!("/series/{other}/lore"), Value::Null).await,
        (StatusCode::OK, json!([]))
    );
    assert_eq!(
        request(&app, "GET", &format!("/series/{first}/lore"), Value::Null).await,
        (StatusCode::OK, json!([entry]))
    );
}

#[tokio::test]
async fn invalid_fields_return_json_422_without_mutation() {
    let app = quillai_api::app(test_pool().await);
    let series_id = create_series(&app).await;
    let collection = format!("/series/{series_id}/lore");
    let entry = create_lore(
        &app,
        series_id,
        json!({"kind": "note", "name": "Unchanged"}),
    )
    .await;
    let uri = format!("{collection}/{}", entry["id"]);
    let mut invalid = vec![json!(null), json!([]), json!("text")];
    for (field, values) in [
        (
            "kind",
            vec![json!("unknown"), json!(""), json!(1), json!(true)],
        ),
        (
            "name",
            vec![
                json!(""),
                json!(" \t\n\r\u{000b}\u{000c}"),
                json!(1),
                json!([]),
            ],
        ),
        (
            "level",
            vec![json!("unknown"), json!(""), json!(1), json!(false)],
        ),
        (
            "pinned",
            vec![json!(0), json!(1), json!("false"), json!([]), json!({})],
        ),
        (
            "visible_text",
            vec![json!(0), json!(true), json!([]), json!({})],
        ),
        (
            "hidden_notes",
            vec![json!(0), json!(true), json!([]), json!({})],
        ),
        (
            "keywords",
            vec![
                json!("word"),
                json!({}),
                json!(false),
                json!([1]),
                json!([null]),
                json!([true]),
                json!([{}]),
                json!([[]]),
                json!([""]),
                json!(["ok", " \t\n"]),
            ],
        ),
        (
            "aliases",
            vec![
                json!("word"),
                json!({}),
                json!(false),
                json!([1]),
                json!([null]),
                json!([true]),
                json!([{}]),
                json!([[]]),
                json!([""]),
                json!(["ok", " \t\n"]),
            ],
        ),
    ] {
        for value in values {
            let mut input = json!({"kind": "note", "name": "Valid"});
            input[field] = value;
            invalid.push(input);
        }
    }
    for input in invalid {
        for (method, path) in [("POST", collection.as_str()), ("PUT", uri.as_str())] {
            let (status, body) = request(&app, method, path, input.clone()).await;
            assert_eq!(
                status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{method} {input}: {body}"
            );
            assert_eq!(body["code"], "VALIDATION_ERROR");
            assert!(body["error"]
                .as_str()
                .unwrap()
                .starts_with("validation error:"));
        }
    }
    for input in [
        json!({}),
        json!({"kind": "note"}),
        json!({"name": "Valid"}),
        json!({"kind": null, "name": "Valid"}),
        json!({"kind": "note", "name": null}),
    ] {
        let (status, body) = request(&app, "POST", &collection, input).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["code"], "VALIDATION_ERROR");
    }
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([entry]))
    );
}

#[tokio::test]
async fn order_by_name_then_id_and_cascade_delete() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let series_id = create_series(&app).await;
    let other_id = create_series(&app).await;
    let late = create_lore(&app, series_id, json!({"kind": "place", "name": "Zed"})).await;
    let first = create_lore(&app, series_id, json!({"kind": "term", "name": "Ada"})).await;
    let tied = create_lore(&app, series_id, json!({"kind": "character", "name": "Ada"})).await;
    let other = create_lore(&app, other_id, json!({"kind": "note", "name": "Other"})).await;
    let collection = format!("/series/{series_id}/lore");
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([first, tied, late]))
    );
    let (status, moved) = request(
        &app,
        "PUT",
        &format!("{collection}/{}", late["id"]),
        json!({"name": "Ada"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([moved, first, tied]))
    );
    assert_eq!(
        request(&app, "DELETE", &format!("/series/{series_id}"), Value::Null).await,
        (StatusCode::NO_CONTENT, Value::Null)
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lore_entries WHERE id <> ?")
        .bind(other["id"].as_i64().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("/series/{other_id}/lore"),
            Value::Null
        )
        .await,
        (StatusCode::OK, json!([other]))
    );
}
