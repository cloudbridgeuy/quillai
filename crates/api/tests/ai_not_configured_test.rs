use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

mod common;

use common::test_pool;

fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn json_body(response: axum::http::Response<Body>) -> serde_json::Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn generate_without_model_or_key_is_not_configured() {
    std::env::remove_var("QUILLAI_AI_MODEL");
    std::env::remove_var("QUILLAI_AI_API_KEY");
    let app = quillai_api::app(test_pool().await);
    let catalog = quillai_api::ai::catalog::ModelCatalog::parse(&json!({
        "models": [{"id":"test-model","label":"Test","input_price_per_million":0.0,"output_price_per_million":0.0}],
        "defaults": {"generate":"test-model","edit":"test-model"}
    }).to_string()).unwrap();
    for (model, use_catalog) in [(None, false), (Some("test-model"), false), (None, true)] {
        if let Some(model) = model {
            std::env::set_var("QUILLAI_AI_MODEL", model);
        }
        let router = if use_catalog {
            app.clone()
                .layer(axum::Extension(std::sync::Arc::new(catalog.clone())))
        } else {
            app.clone()
        };
        let response = router
            .oneshot(json_request(
                "POST",
                "/ai/generate",
                json!({
                    "action": "generate",
                    "document": json!({"delta": {"ops": [{"insert": "Hello"}]}, "text": "Hello"}),
                }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json_body(response).await["code"], "AI_NOT_CONFIGURED");
    }
    std::env::set_var("QUILLAI_AI_API_KEY", "test-key");
    for model in ["", "   "] {
        std::env::set_var("QUILLAI_AI_MODEL", model);
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/ai/generate",
                json!({
                    "action": "generate",
                    "document": {"delta": {"ops": [{"insert": "Hello"}]}, "text": "Hello"},
                }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            json_body(response).await["code"],
            "AI_PROVIDER_CONFIG_ERROR"
        );
    }
    std::env::remove_var("QUILLAI_AI_MODEL");
    std::env::remove_var("QUILLAI_AI_API_KEY");
}
