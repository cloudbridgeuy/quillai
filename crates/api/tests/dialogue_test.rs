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

fn catalog(dialogue_default: Option<&str>) -> ModelCatalog {
    let mut defaults =
        json!({"generate":"gen-default","edit":"edit-default","image":"image-default"});
    if let Some(id) = dialogue_default {
        defaults["dialogue"] = json!(id);
    }
    ModelCatalog::parse(
        &json!({"models":[
            {"id":"gen-default","label":"G","input_price_per_million":0,"output_price_per_million":0},
            {"id":"edit-default","label":"E","input_price_per_million":0,"output_price_per_million":0},
            {"id":"dialogue-default","label":"D","input_price_per_million":0,"output_price_per_million":0},
            {"id":"dialogue-saved","label":"DS","input_price_per_million":0,"output_price_per_million":0},
            {"id":"image-default","label":"I","kind":"image","price_per_image":0.01,"size":"1024x1024","quality":"medium"}
        ],"defaults":defaults})
        .to_string(),
    )
    .unwrap()
}

async fn world(dialogue_default: Option<&str>) -> (Router, Receiver<Value>) {
    let (sender, receiver) = tokio::sync::mpsc::channel(8);
    let provider = Router::new().route(
        "/chat/completions",
        axum::routing::post(move |axum::Json(body): axum::Json<Value>| {
            let sender = sender.clone();
            async move {
                sender.send(body).await.unwrap();
                "data: {\"choices\":[{\"delta\":{\"content\":\"\\\"Hi.\\\"\"}}]}\n\ndata: [DONE]\n\n"
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });
    let app = quillai_api::app(common::test_pool().await)
        .layer(axum::Extension(Arc::new(catalog(dialogue_default))));
    request(
        &app,
        "PUT",
        "/settings/ai",
        json!({"base_url":base_url,"api_key":"k"}),
    )
    .await;
    (app, receiver)
}

fn exchange(dialogue: Value) -> Value {
    json!({
        "action":"dialogue",
        "document":{"delta":{"ops":[{"insert":"She waited.\n"}]},"text":"She waited.\n"},
        "lore":[
            {"name":"Mara","kind":"character","visible_text":"mara-voice","level":"include"},
            {"name":"Ilan","kind":"character","visible_text":"ilan-secret","level":"known_dont_reveal"},
            {"name":"Keep","kind":"place","visible_text":"keep-text","level":"include"},
            {"name":"Ghost","kind":"character","visible_text":"ghost-text","level":"leave_out"}
        ],
        "dialogue":dialogue
    })
}

async fn sent(app: &Router, input: Value, receiver: &mut Receiver<Value>) -> Value {
    let response = request(app, "POST", "/ai/generate", input).await;
    assert_eq!(response.status(), StatusCode::OK);
    to_bytes(response.into_body(), usize::MAX).await.unwrap();
    receiver.recv().await.unwrap()
}

async fn assert_422(input: Value, expected: &str) {
    let (app, mut receiver) = world(None).await;
    let response = request(&app, "POST", "/ai/generate", input).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error = json_of(response).await.to_string();
    assert!(error.contains(expected), "unexpected error {error}");
    assert!(receiver.try_recv().is_err());
}

#[tokio::test]
async fn missing_dialogue_is_422() {
    let mut input = exchange(json!(null));
    input.as_object_mut().unwrap().remove("dialogue");
    assert_422(input, "dialogue is required").await;
}

#[tokio::test]
async fn empty_speakers_is_422() {
    assert_422(exchange(json!({"speakers":[]})), "speakers").await;
}

#[tokio::test]
async fn more_than_six_speakers_is_422() {
    assert_422(exchange(json!({"speakers":vec!["Mara";7]})), "6").await;
}

#[tokio::test]
async fn duplicate_speakers_is_422() {
    assert_422(exchange(json!({"speakers":["Mara","Mara"]})), "duplicate").await;
}

#[tokio::test]
async fn speaker_that_is_not_a_character_in_lore_is_422() {
    assert_422(exchange(json!({"speakers":["Keep"]})), "Keep").await;
    assert_422(exchange(json!({"speakers":["Nobody"]})), "Nobody").await;
}

#[tokio::test]
async fn brief_over_cap_is_422_and_at_cap_is_accepted() {
    assert_422(
        exchange(json!({"speakers":["Mara"],"brief":"a".repeat(4_097)})),
        "4096",
    )
    .await;
    let (app, mut receiver) = world(None).await;
    sent(
        &app,
        exchange(json!({"speakers":["Mara"],"brief":"é".repeat(2_048)})),
        &mut receiver,
    )
    .await;
}

#[tokio::test]
async fn lore_over_cap_is_422() {
    let mut input = exchange(json!({"speakers":["Mara"]}));
    input["lore"][0]["visible_text"] = json!("a".repeat(262_145));
    assert_422(input, "262144").await;
}

#[tokio::test]
async fn hidden_notes_and_leave_out_entries_never_reach_the_prompt() {
    let mut hidden = exchange(json!({"speakers":["Mara"]}));
    hidden["lore"][0]["hidden_notes"] = json!("secret-hidden-note");
    assert_422(hidden, "hidden_notes").await;
    assert_422(exchange(json!({"speakers":["Ghost"]})), "Ghost").await;

    let (app, mut receiver) = world(None).await;
    let body = sent(
        &app,
        exchange(json!({"speakers":["Mara","Ilan"],"brief":"the debt"})),
        &mut receiver,
    )
    .await;
    assert_eq!(body["max_tokens"], 1024);
    let user = body["messages"].as_array().unwrap().last().unwrap()["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(user.contains("<action>dialogue</action>"));
    assert!(user.contains("<speaker>Mara</speaker>\n<speaker>Ilan</speaker>"));
    assert!(user.contains("mara-voice") && user.contains("ilan-secret"));
    assert!(user.contains("Known, but MUST NOT be revealed in prose."));
    let raw = body.to_string();
    for leaked in ["ghost-text", "secret-hidden-note"] {
        assert!(!raw.contains(leaked), "{leaked} leaked into {raw}");
    }
}

#[tokio::test]
async fn dialogue_model_falls_back_to_generate_then_uses_catalog_and_saved_defaults() {
    let (app, mut receiver) = world(None).await;
    let speak = || exchange(json!({"speakers":["Mara"]}));
    let models = json_of(request(&app, "GET", "/ai/models", json!({})).await).await;
    assert_eq!(models["defaults"]["dialogue"], "gen-default");
    assert_eq!(
        sent(&app, speak(), &mut receiver).await["model"],
        "gen-default"
    );
    let view = json_of(
        request(
            &app,
            "PUT",
            "/settings/ai",
            json!({"dialogue_model":"dialogue-saved"}),
        )
        .await,
    )
    .await;
    assert_eq!(view["dialogue_model"], "dialogue-saved");
    assert_eq!(view["sources"]["dialogue_model"], "saved");
    assert_eq!(
        sent(&app, speak(), &mut receiver).await["model"],
        "dialogue-saved"
    );
    let models = json_of(request(&app, "GET", "/ai/models", json!({})).await).await;
    assert_eq!(models["defaults"]["dialogue"], "dialogue-saved");
    assert_eq!(models["defaults"]["generate"], "gen-default");
    let rejected = request(
        &app,
        "PUT",
        "/settings/ai",
        json!({"dialogue_model":"image-default"}),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let (app, mut receiver) = world(Some("dialogue-default")).await;
    assert_eq!(
        sent(&app, speak(), &mut receiver).await["model"],
        "dialogue-default"
    );
}

#[tokio::test]
async fn migration_adds_nullable_dialogue_model_and_keeps_saved_models() {
    let pool = common::test_pool().await;
    quillai_api::database::MIGRATOR
        .undo(&pool, 20261107000000)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ai_settings (id, generate_model, edit_model, image_model, lore_complete_model) VALUES (1, 'g', 'e', 'i', 'l')")
        .execute(&pool)
        .await
        .unwrap();
    quillai_api::database::MIGRATOR.run(&pool).await.unwrap();
    let row: (String, String, String, String, Option<String>) = sqlx::query_as(
        "SELECT generate_model, edit_model, image_model, lore_complete_model, dialogue_model FROM ai_settings WHERE id = 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row, ("g".into(), "e".into(), "i".into(), "l".into(), None));
}
