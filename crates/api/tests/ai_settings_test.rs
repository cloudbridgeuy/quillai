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

async fn body(response: Response) -> Value {
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

fn catalog() -> ModelCatalog {
    ModelCatalog::parse(&json!({"models":[
        {"id":"saved-model","label":"Saved","input_price_per_million":0,"output_price_per_million":0},
        {"id":"explicit-model","label":"Explicit","input_price_per_million":1,"output_price_per_million":2}
    ],"defaults":{"generate":"explicit-model","edit":"saved-model"}}).to_string()).unwrap()
}

#[tokio::test]
async fn settings_save_redact_omit_and_clear_each_field() {
    let pool = common::test_pool().await;
    let app = quillai_api::app(pool.clone());
    let initial = body(request(&app, "GET", "/settings/ai", json!({})).await).await;
    assert_eq!(initial["key_set"], false);
    let saved = json!({"base_url":"https://example.com/v1","api_key":"secret","generate_model":"saved","edit_model":"saved"});
    let view = body(request(&app, "PUT", "/settings/ai", saved.clone()).await).await;
    assert_eq!(view["base_url"], saved["base_url"]);
    assert_eq!(view["generate_model"], "saved");
    assert_eq!(view["edit_model"], "saved");
    assert_eq!(view["key_set"], true);
    assert_eq!(
        view["sources"],
        json!({"base_url":"saved","api_key":"saved","generate_model":"saved","edit_model":"saved","image_model":"none","lore_complete_model":"none"})
    );
    let response = request(&app, "GET", "/settings/ai", json!({})).await;
    assert_eq!(response.status(), StatusCode::OK);
    let raw = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert!(!String::from_utf8_lossy(&raw).contains("secret"));
    let get: Value = serde_json::from_slice(&raw).unwrap();
    assert!(get.get("api_key").is_none());
    assert_eq!(get, view);
    assert_eq!(
        body(request(&app, "PUT", "/settings/ai", json!({})).await).await,
        view
    );
    let changed = body(
        request(
            &app,
            "PUT",
            "/settings/ai",
            json!({"generate_model":"changed"}),
        )
        .await,
    )
    .await;
    assert_eq!(changed["generate_model"], "changed");
    assert_eq!(changed["edit_model"], "saved");
    assert_eq!(changed["base_url"], view["base_url"]);
    assert_eq!(changed["key_set"], true);
    assert_eq!(
        quillai_api::settings::ai::load(&pool)
            .await
            .unwrap()
            .api_key
            .as_deref(),
        Some("secret")
    );
    for field in ["base_url", "api_key", "generate_model", "edit_model"] {
        body(request(&app, "PUT", "/settings/ai", saved.clone()).await).await;
        let cleared = body(request(&app, "PUT", "/settings/ai", json!({field:""})).await).await;
        assert_eq!(cleared["sources"][field], initial["sources"][field]);
        assert_eq!(cleared["key_set"], field != "api_key");
        for visible in ["base_url", "generate_model", "edit_model"] {
            assert_eq!(
                &cleared[visible],
                if field == visible {
                    &initial[visible]
                } else {
                    &view[visible]
                }
            );
        }
        assert_eq!(
            body(request(&app, "GET", "/settings/ai", json!({})).await).await,
            cleared
        );
    }
}

#[tokio::test]
async fn saved_model_defaults_and_generation_update_without_restart() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
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
    for file_catalog in [false, true] {
        let app = quillai_api::app(common::test_pool().await);
        let app = if file_catalog {
            app.layer(axum::Extension(Arc::new(catalog())))
        } else {
            app
        };
        if file_catalog {
            let listing = body(request(&app, "GET", "/ai/models", json!({})).await).await;
            assert_eq!(
                listing["defaults"],
                json!({"generate":"explicit-model","edit":"saved-model","lore_complete":"explicit-model"})
            );
        }
        body(
            request(
                &app,
                "PUT",
                "/settings/ai",
                json!({"base_url":base_url,"api_key":"secret","generate_model":"saved-model","edit_model":"saved-model"}),
            )
            .await,
        )
        .await;
        let listing = body(request(&app, "GET", "/ai/models", json!({})).await).await;
        assert_eq!(
            listing["defaults"],
            json!({"generate":"saved-model","edit":"saved-model","lore_complete":if file_catalog { "explicit-model" } else { "saved-model" }})
        );
        assert_eq!(
            listing["models"].as_array().unwrap().len(),
            if file_catalog { 2 } else { 1 }
        );
        for explicit in if file_catalog {
            vec![None, Some("explicit-model")]
        } else {
            vec![None]
        } {
            let mut input = json!({"action":"generate","document":{"delta":{"ops":[{"insert":"Hello"}]},"text":"Hello"}});
            if let Some(model) = explicit {
                input["model"] = json!(model);
            }
            let response = request(&app, "POST", "/ai/generate", input).await;
            assert_eq!(response.status(), StatusCode::OK);
            to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert_eq!(
                receiver.recv().await.unwrap()["model"],
                explicit.unwrap_or("saved-model")
            );
        }
        if file_catalog {
            body(
                request(
                    &app,
                    "PUT",
                    "/settings/ai",
                    json!({"generate_model":"","edit_model":""}),
                )
                .await,
            )
            .await;
            let listing = body(request(&app, "GET", "/ai/models", json!({})).await).await;
            assert_eq!(
                listing["defaults"],
                json!({"generate":"explicit-model","edit":"saved-model","lore_complete":"explicit-model"})
            );
        }
    }
    server.abort();
}

#[tokio::test]
async fn invalid_urls_and_models_outside_file_catalog_return_422() {
    let app =
        quillai_api::app(common::test_pool().await).layer(axum::Extension(Arc::new(catalog())));
    for input in [
        json!({"base_url":"file:///tmp/x"}),
        json!({"base_url":"invalid"}),
        json!({"generate_model":" "}),
        json!({"edit_model":"other"}),
    ] {
        let response = request(&app, "PUT", "/settings/ai", input).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn image_model_saves_and_must_be_an_image_catalog_entry() {
    let catalog = ModelCatalog::parse(&json!({"models":[
        {"id":"text-model","label":"Text","input_price_per_million":0,"output_price_per_million":0},
        {"id":"image-model","label":"Image","kind":"image","price_per_image":0.011,"size":"1024x1024","quality":"medium"}
    ],"defaults":{"generate":"text-model","edit":"text-model","image":"image-model"}}).to_string()).unwrap();
    let app = quillai_api::app(common::test_pool().await).layer(axum::Extension(Arc::new(catalog)));
    let view = body(
        request(
            &app,
            "PUT",
            "/settings/ai",
            json!({"image_model":"image-model"}),
        )
        .await,
    )
    .await;
    assert_eq!(view["image_model"], "image-model");
    assert_eq!(view["sources"]["image_model"], "saved");
    assert_eq!(
        body(request(&app, "GET", "/settings/ai", json!({})).await).await,
        view
    );
    for update in [
        json!({"image_model":"text-model"}),
        json!({"generate_model":"image-model"}),
    ] {
        let response = request(&app, "PUT", "/settings/ai", update).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    let models = body(request(&app, "GET", "/ai/models", json!({})).await).await;
    assert_eq!(models["defaults"]["image"], "image-model");
    assert_eq!(models["models"][1]["kind"], "image");
    assert_eq!(models["models"][1]["price_per_image"], 0.011);
    let cleared = body(request(&app, "PUT", "/settings/ai", json!({"image_model":""})).await).await;
    assert!(cleared["image_model"].is_null());
}
