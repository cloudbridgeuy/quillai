mod common;

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use quillai_api::ai::catalog::ModelCatalog;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
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

fn catalog(lore_complete_default: Option<&str>) -> ModelCatalog {
    let mut defaults =
        json!({"generate":"gen-default","edit":"edit-default","image":"image-default"});
    if let Some(id) = lore_complete_default {
        defaults["lore_complete"] = json!(id);
    }
    ModelCatalog::parse(
        &json!({"models":[
            {"id":"gen-default","label":"G","input_price_per_million":0,"output_price_per_million":0},
            {"id":"edit-default","label":"E","input_price_per_million":0,"output_price_per_million":0},
            {"id":"lore-default","label":"L","input_price_per_million":0,"output_price_per_million":0},
            {"id":"lore-saved","label":"LS","input_price_per_million":0,"output_price_per_million":0},
            {"id":"image-default","label":"I","kind":"image","price_per_image":0.01,"size":"1024x1024","quality":"medium"}
        ],"defaults":defaults})
        .to_string(),
    )
    .unwrap()
}

async fn world(lore_complete_default: Option<&str>) -> (Router, Receiver<Value>) {
    let (sender, receiver) = tokio::sync::mpsc::channel(8);
    let provider = Router::new().route(
        "/chat/completions",
        axum::routing::post(move |axum::Json(body): axum::Json<Value>| {
            let sender = sender.clone();
            async move {
                sender.send(body).await.unwrap();
                "data: {\"choices\":[{\"delta\":{\"content\":\" More.\"}}]}\n\ndata: [DONE]\n\n"
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });
    let app = quillai_api::app(common::test_pool().await)
        .layer(axum::Extension(Arc::new(catalog(lore_complete_default))));
    request(
        &app,
        "PUT",
        "/settings/ai",
        json!({"base_url":base_url,"api_key":"k"}),
    )
    .await;
    (app, receiver)
}

fn completion(text: &str) -> Value {
    json!({
        "action":"lore_complete",
        "loreComplete":{"name":"Mara <Vex> & co","kind":"character","text":text},
        "lore":[{"name":"OtherEntry","kind":"place","visible_text":"other-entry-text","level":"include"}],
        "hidden_notes":"secret-hidden-note",
        "keywords":["secret-keyword"],
        "aliases":["secret-alias"],
        "storyContext":"secret-story-context"
    })
}

async fn sent(app: &Router, input: Value, receiver: &mut Receiver<Value>) -> Value {
    let response = request(app, "POST", "/ai/generate", input).await;
    assert_eq!(response.status(), StatusCode::OK);
    to_bytes(response.into_body(), usize::MAX).await.unwrap();
    receiver.recv().await.unwrap()
}

#[tokio::test]
async fn prompt_has_escaped_name_kind_and_text_and_nothing_hidden() {
    let (app, mut receiver) = world(None).await;
    let body = sent(
        &app,
        completion("Born in <the> keep & raised"),
        &mut receiver,
    )
    .await;
    let user = body["messages"].as_array().unwrap().last().unwrap()["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(user.contains("You are completing a reference note about a story element."));
    assert!(user.contains("Mara &lt;Vex&gt; &amp; co"));
    assert!(user.contains("character"));
    assert!(user.contains("Born in &lt;the&gt; keep &amp; raised"));
    let raw = body.to_string();
    for leaked in [
        "secret-hidden-note",
        "secret-keyword",
        "secret-alias",
        "secret-story-context",
        "OtherEntry",
        "other-entry-text",
    ] {
        assert!(!raw.contains(leaked), "{leaked} leaked into {raw}");
    }
}

#[tokio::test]
async fn text_over_cap_is_422_before_provider_and_at_cap_is_accepted() {
    let (app, mut receiver) = world(None).await;
    let over = request(
        &app,
        "POST",
        "/ai/generate",
        completion(&"a".repeat(65_537)),
    )
    .await;
    assert_eq!(over.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error = json_of(over).await;
    assert!(
        error.to_string().contains("65536"),
        "unexpected error {error}"
    );
    assert!(receiver.try_recv().is_err());
    sent(&app, completion(&"é".repeat(32_768)), &mut receiver).await;
}

#[tokio::test]
async fn saved_lore_complete_model_beats_catalog_default_which_falls_back_to_generate() {
    let (app, mut receiver) = world(None).await;
    assert_eq!(
        sent(&app, completion("x"), &mut receiver).await["model"],
        "gen-default"
    );
    let view = json_of(
        request(
            &app,
            "PUT",
            "/settings/ai",
            json!({"lore_complete_model":"lore-saved"}),
        )
        .await,
    )
    .await;
    assert_eq!(view["lore_complete_model"], "lore-saved");
    assert_eq!(view["sources"]["lore_complete_model"], "saved");
    assert_eq!(
        sent(&app, completion("x"), &mut receiver).await["model"],
        "lore-saved"
    );
    let rejected = request(
        &app,
        "PUT",
        "/settings/ai",
        json!({"lore_complete_model":"image-default"}),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let (app, mut receiver) = world(Some("lore-default")).await;
    assert_eq!(
        sent(&app, completion("x"), &mut receiver).await["model"],
        "lore-default"
    );
}

#[tokio::test]
async fn models_route_reports_lore_complete_default() {
    let (app, _receiver) = world(None).await;
    let models = json_of(request(&app, "GET", "/ai/models", json!({})).await).await;
    assert_eq!(models["defaults"]["lore_complete"], "gen-default");
    request(
        &app,
        "PUT",
        "/settings/ai",
        json!({"lore_complete_model":"lore-saved"}),
    )
    .await;
    let models = json_of(request(&app, "GET", "/ai/models", json!({})).await).await;
    assert_eq!(models["defaults"]["lore_complete"], "lore-saved");
    assert_eq!(models["defaults"]["generate"], "gen-default");
}

#[tokio::test]
async fn migration_adds_nullable_column_and_keeps_saved_models() {
    let pool = common::test_pool().await;
    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('ai_settings')")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        columns.contains(&"lore_complete_model".to_string()),
        "{columns:?}"
    );
    quillai_api::database::MIGRATOR
        .undo(&pool, 20261106000002)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ai_settings (id, generate_model, edit_model, image_model) VALUES (1, 'g', 'e', 'i')")
        .execute(&pool)
        .await
        .unwrap();
    quillai_api::database::MIGRATOR.run(&pool).await.unwrap();
    let row: (String, String, String, Option<String>) = sqlx::query_as(
        "SELECT generate_model, edit_model, image_model, lore_complete_model FROM ai_settings WHERE id = 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row, ("g".into(), "e".into(), "i".into(), None));
}
