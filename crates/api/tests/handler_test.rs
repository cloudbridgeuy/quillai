use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::sqlite::SqlitePool;
use tower::ServiceExt;

async fn response_body(res: axum::http::Response<Body>) -> String {
    let bytes = res
        .into_body()
        .collect()
        .await
        .expect("failed to read body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("response body is not valid UTF-8")
}

async fn json_body(res: axum::http::Response<Body>) -> serde_json::Value {
    let text = response_body(res).await;
    serde_json::from_str(&text).expect("response body is not valid JSON")
}

fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn delete_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("failed to create in-memory SQLite pool");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");
    pool
}

async fn create_user(pool: &SqlitePool, name: &str, email: &str) -> i64 {
    let app = quillai_api::app(pool.clone());
    let req = json_request("POST", "/users", json!({ "name": name, "email": email }));
    let res = app.into_service().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = json_body(res).await;
    body["id"].as_i64().unwrap()
}

#[tokio::test]
async fn list_users_empty() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool);

    let res = app
        .into_service()
        .oneshot(get_request("/users"))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(json_body(res).await, json!([]));
}

#[tokio::test]
async fn create_user_returns_201() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool);

    let req = json_request(
        "POST",
        "/users",
        json!({ "name": "Alice", "email": "alice@example.com" }),
    );
    let res = app.into_service().oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let parsed = json_body(res).await;
    assert_eq!(parsed["name"], "Alice");
    assert_eq!(parsed["email"], "alice@example.com");
    assert!(parsed["id"].is_number(), "Expected id to be a number");
}

#[tokio::test]
async fn create_user_invalid_name_returns_422() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool);

    let req = json_request(
        "POST",
        "/users",
        json!({ "name": "", "email": "alice@example.com" }),
    );
    let res = app.into_service().oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let parsed = json_body(res).await;
    assert_eq!(parsed["code"], "VALIDATION_ERROR");
    assert!(parsed["error"].as_str().unwrap().contains("name"));
}

#[tokio::test]
async fn create_user_invalid_email_returns_422() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool);

    let req = json_request("POST", "/users", json!({ "name": "Alice", "email": "bad" }));
    let res = app.into_service().oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let parsed = json_body(res).await;
    assert_eq!(parsed["code"], "VALIDATION_ERROR");
    assert!(parsed["error"].as_str().unwrap().contains("email"));
}

#[tokio::test]
async fn create_then_list() {
    let pool = test_pool().await;
    create_user(&pool, "Alice", "alice@example.com").await;

    let app = quillai_api::app(pool);
    let res = app
        .into_service()
        .oneshot(get_request("/users"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let parsed = json_body(res).await;
    let users = parsed.as_array().expect("Expected JSON array");
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["name"], "Alice");
    assert_eq!(users[0]["email"], "alice@example.com");
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool);

    let res = app
        .into_service()
        .oneshot(get_request("/nonexistent"))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_user_existing() {
    let pool = test_pool().await;
    let id = create_user(&pool, "Alice", "alice@example.com").await;

    let app = quillai_api::app(pool);
    let res = app
        .into_service()
        .oneshot(get_request(&format!("/users/{id}")))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let parsed = json_body(res).await;
    assert_eq!(parsed["id"], id);
    assert_eq!(parsed["name"], "Alice");
    assert_eq!(parsed["email"], "alice@example.com");
}

#[tokio::test]
async fn get_user_not_found() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool);

    let res = app
        .into_service()
        .oneshot(get_request("/users/999"))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let parsed = json_body(res).await;
    assert_eq!(parsed["error"], "not found");
    assert_eq!(parsed["code"], "NOT_FOUND");
}

#[tokio::test]
async fn delete_user_existing() {
    let pool = test_pool().await;
    let id = create_user(&pool, "Alice", "alice@example.com").await;

    let app = quillai_api::app(pool);
    let res = app
        .into_service()
        .oneshot(delete_request(&format!("/users/{id}")))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn delete_user_not_found() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool);

    let res = app
        .into_service()
        .oneshot(delete_request("/users/999"))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_then_get_returns_404() {
    let pool = test_pool().await;
    let id = create_user(&pool, "Alice", "alice@example.com").await;

    // DELETE the user
    let app = quillai_api::app(pool.clone());
    let res = app
        .into_service()
        .oneshot(delete_request(&format!("/users/{id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // GET should now return 404
    let app = quillai_api::app(pool);
    let res = app
        .into_service()
        .oneshot(get_request(&format!("/users/{id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_user_name_only() {
    let pool = test_pool().await;
    let id = create_user(&pool, "Alice", "alice@example.com").await;

    let app = quillai_api::app(pool);
    let req = json_request("PUT", &format!("/users/{id}"), json!({ "name": "Bob" }));
    let res = app.into_service().oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let parsed = json_body(res).await;
    assert_eq!(parsed["name"], "Bob");
    assert_eq!(parsed["email"], "alice@example.com");
}

#[tokio::test]
async fn update_user_email_only() {
    let pool = test_pool().await;
    let id = create_user(&pool, "Alice", "alice@example.com").await;

    let app = quillai_api::app(pool);
    let req = json_request(
        "PUT",
        &format!("/users/{id}"),
        json!({ "email": "new@example.com" }),
    );
    let res = app.into_service().oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let parsed = json_body(res).await;
    assert_eq!(parsed["name"], "Alice");
    assert_eq!(parsed["email"], "new@example.com");
}

#[tokio::test]
async fn update_user_both_fields() {
    let pool = test_pool().await;
    let id = create_user(&pool, "Alice", "alice@example.com").await;

    let app = quillai_api::app(pool);
    let req = json_request(
        "PUT",
        &format!("/users/{id}"),
        json!({ "name": "Bob", "email": "bob@example.com" }),
    );
    let res = app.into_service().oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let parsed = json_body(res).await;
    assert_eq!(parsed["name"], "Bob");
    assert_eq!(parsed["email"], "bob@example.com");
}

#[tokio::test]
async fn update_user_invalid_data_returns_422() {
    let pool = test_pool().await;
    let id = create_user(&pool, "Alice", "alice@example.com").await;

    let app = quillai_api::app(pool);
    let req = json_request("PUT", &format!("/users/{id}"), json!({ "name": "" }));
    let res = app.into_service().oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let parsed = json_body(res).await;
    assert_eq!(parsed["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn update_user_not_found() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool);

    let req = json_request("PUT", "/users/999", json!({ "name": "Ghost" }));
    let res = app.into_service().oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
