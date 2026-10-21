use std::convert::Infallible;

use axum::body::{Body, Bytes};
use axum::extract::rejection::JsonRejection;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::Json;
use futures_util::StreamExt;

use super::catalog::ModelCatalog;
use super::models::{event_to_ndjson_line, GenerateInput, GenerateRequest};
use super::provider::{AiProvider, OpenAiCompatibleConfig, OpenAiCompatibleProvider};
use crate::error::Error;

pub async fn generate(
    axum::extract::State(pool): axum::extract::State<sqlx::SqlitePool>,
    catalog: Option<axum::Extension<std::sync::Arc<ModelCatalog>>>,
    payload: Result<Json<GenerateRequest>, JsonRejection>,
) -> Result<Response, Error> {
    let Json(request) = payload.map_err(Error::from_json_rejection)?;
    let mut input = GenerateInput::try_from(request)?;
    input.author_guidance = Some(crate::settings::load_guidance(&pool).await?.text);
    let (config, include_usage) = match catalog {
        Some(axum::Extension(catalog)) => {
            let model = catalog.resolve(input.action);
            (
                OpenAiCompatibleConfig::from_env_with_model(model.id.clone())?,
                model.include_usage,
            )
        }
        None => (OpenAiCompatibleConfig::from_env()?, true),
    };
    let provider = OpenAiCompatibleProvider::new(config)?.with_include_usage(include_usage);
    let body_stream = provider
        .stream(input)
        .map(|event| Ok::<Bytes, Infallible>(Bytes::from(event_to_ndjson_line(&event))));

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(body_stream))
        .map_err(|err| Error::Internal(format!("failed to build AI response: {err}")))
}
