mod common;

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use quillai_api::{ai::catalog::ModelCatalog, database::MIGRATOR};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn request(app: &Router, method: &str, path: &str, body: Value) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn json_of(response: Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

fn catalog() -> ModelCatalog {
    ModelCatalog::parse(
        &json!({"models":[
        {"id":"gen-default","label":"G","input_price_per_million":0,"output_price_per_million":0},
        {"id":"edit-default","label":"E","input_price_per_million":0,"output_price_per_million":0},
        {"id":"gen-saved","label":"GS","input_price_per_million":0,"output_price_per_million":0},
        {"id":"edit-saved","label":"ES","input_price_per_million":0,"output_price_per_million":0}
    ],"defaults":{"generate":"gen-default","edit":"edit-default"}})
        .to_string(),
    )
    .unwrap()
}

#[tokio::test]
async fn migration_copies_model_into_both_action_columns_and_down_restores_it() {
    let pool = common::test_pool().await;
    MIGRATOR.undo(&pool, 20261101000003).await.unwrap();
    sqlx::query(
        "INSERT INTO ai_settings (id, base_url, api_key, model) VALUES (1, 'u', 'k', 'old-model')",
    )
    .execute(&pool)
    .await
    .unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    let row: (String, String) =
        sqlx::query_as("SELECT generate_model, edit_model FROM ai_settings WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row, ("old-model".into(), "old-model".into()));
    sqlx::query("UPDATE ai_settings SET edit_model = 'other'")
        .execute(&pool)
        .await
        .unwrap();
    MIGRATOR.undo(&pool, 20261101000003).await.unwrap();
    let model: String = sqlx::query_scalar("SELECT model FROM ai_settings WHERE id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(model, "old-model");
}

#[tokio::test]
async fn put_and_get_store_each_action_model_independently_and_reject_unknown_ids() {
    let app =
        quillai_api::app(common::test_pool().await).layer(axum::Extension(Arc::new(catalog())));
    let put = json_of(
        request(
            &app,
            "PUT",
            "/settings/ai",
            json!({"generate_model":"gen-saved","edit_model":"edit-saved"}),
        )
        .await,
    )
    .await;
    assert_eq!(put["generate_model"], "gen-saved");
    assert_eq!(put["edit_model"], "edit-saved");
    assert!(put.get("model").is_none());
    assert_eq!(put["sources"]["generate_model"], "saved");
    assert_eq!(put["sources"]["edit_model"], "saved");
    assert!(put["sources"].get("model").is_none());
    let only_edit_cleared =
        json_of(request(&app, "PUT", "/settings/ai", json!({"edit_model":""})).await).await;
    assert_eq!(only_edit_cleared["generate_model"], "gen-saved");
    assert_eq!(only_edit_cleared["edit_model"], Value::Null);
    assert_eq!(only_edit_cleared["sources"]["edit_model"], "none");
    assert_eq!(
        json_of(request(&app, "GET", "/settings/ai", json!({})).await).await,
        only_edit_cleared
    );
    for input in [
        json!({"generate_model":"other"}),
        json!({"edit_model":"other"}),
        json!({"edit_model":" "}),
    ] {
        let response = request(&app, "PUT", "/settings/ai", input).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    let old = request(&app, "PUT", "/settings/ai", json!({"model":"gen-saved"})).await;
    assert_eq!(old.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn request_model_beats_saved_action_model_beats_catalog_default() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
    let provider = Router::new().route(
        "/chat/completions",
        axum::routing::post(move |axum::Json(body): axum::Json<Value>| {
            let sender = sender.clone();
            async move {
                sender.send(body).await.unwrap();
                "data: [DONE]\n\n"
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });
    let app =
        quillai_api::app(common::test_pool().await).layer(axum::Extension(Arc::new(catalog())));
    let doc = json!({"delta":{"ops":[{"insert":"Hello"}]},"text":"Hello"});
    let sel =
        json!({"delta":{"ops":[{"insert":"Hello"}]},"text":"Hello","range":{"index":0,"length":5}});
    let sent = |action: &'static str, model: Option<&'static str>| {
        let mut input = json!({"action":action,"document":doc,"selection":sel});
        if let Some(model) = model {
            input["model"] = json!(model);
        }
        input
    };
    async fn used(
        app: &Router,
        input: Value,
        rx: &mut tokio::sync::mpsc::Receiver<Value>,
    ) -> Value {
        let response = request(app, "POST", "/ai/generate", input).await;
        assert_eq!(response.status(), StatusCode::OK);
        to_bytes(response.into_body(), usize::MAX).await.unwrap();
        rx.recv().await.unwrap()["model"].clone()
    }
    request(
        &app,
        "PUT",
        "/settings/ai",
        json!({"base_url":base_url,"api_key":"k"}),
    )
    .await;
    assert_eq!(
        used(&app, sent("generate", None), &mut receiver).await,
        "gen-default"
    );
    assert_eq!(
        used(&app, sent("edit", None), &mut receiver).await,
        "edit-default"
    );
    request(
        &app,
        "PUT",
        "/settings/ai",
        json!({"generate_model":"gen-saved"}),
    )
    .await;
    assert_eq!(
        used(&app, sent("generate", None), &mut receiver).await,
        "gen-saved"
    );
    assert_eq!(
        used(&app, sent("edit", None), &mut receiver).await,
        "edit-default"
    );
    request(
        &app,
        "PUT",
        "/settings/ai",
        json!({"edit_model":"edit-saved"}),
    )
    .await;
    assert_eq!(
        used(&app, sent("edit", None), &mut receiver).await,
        "edit-saved"
    );
    assert_eq!(
        used(&app, sent("generate", None), &mut receiver).await,
        "gen-saved"
    );
    assert_eq!(
        used(&app, sent("edit", Some("gen-default")), &mut receiver).await,
        "gen-default"
    );
    server.abort();
}
