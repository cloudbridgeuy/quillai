use std::convert::Infallible;

use axum::body::{Body, Bytes};
use axum::extract::rejection::JsonRejection;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::Json;
use futures_util::StreamExt;

use super::catalog::ModelCatalog;
use super::models::{event_to_ndjson_line, AiAction, GenerateInput, GenerateRequest, StreamEvent};
use super::overlap::Overlap;
use super::provider::{AiProvider, OpenAiCompatibleConfig, OpenAiCompatibleProvider};
use super::segments::plain_text;
use super::tagged_text::{into_delta, TaggedText};
use crate::error::Error;

pub async fn models(
    catalog: Option<axum::Extension<std::sync::Arc<ModelCatalog>>>,
) -> Result<Json<serde_json::Value>, Error> {
    let axum::Extension(catalog) = catalog.ok_or(Error::AiNotConfigured)?;
    let models: Vec<_> = catalog
        .models()
        .iter()
        .map(|model| {
            serde_json::json!({
                "id": model.id,
                "label": model.label,
                "input_price_per_million": model.input_price_per_million,
                "output_price_per_million": model.output_price_per_million,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({
        "models": models,
        "defaults": {
            "generate": catalog.resolve(AiAction::Generate).id,
            "edit": catalog.resolve(AiAction::Edit).id,
        },
    })))
}

pub async fn generate(
    axum::extract::State(pool): axum::extract::State<sqlx::SqlitePool>,
    catalog: Option<axum::Extension<std::sync::Arc<ModelCatalog>>>,
    payload: Result<Json<GenerateRequest>, JsonRejection>,
) -> Result<Response, Error> {
    let Json(mut request) = payload.map_err(Error::from_json_rejection)?;
    let requested_model = request.model.take();
    let selected_model = requested_model
        .as_ref()
        .map(|value| {
            value
                .as_str()
                .and_then(|id| catalog.as_ref()?.0.find(id))
                .ok_or_else(|| Error::Validation("model must match a configured model id".into()))
        })
        .transpose()?;
    let mut input = GenerateInput::try_from(request)?;
    input.author_guidance = Some(crate::settings::load_guidance(&pool).await?.text);
    let (config, include_usage, reasoning_effort) = match catalog.as_ref() {
        Some(axum::Extension(catalog)) => {
            let model = selected_model.unwrap_or_else(|| catalog.resolve(input.action));
            (
                OpenAiCompatibleConfig::from_env_with_model(model.id.clone())
                    .map_err(missing_config)?,
                model.include_usage,
                model.reasoning_effort,
            )
        }
        None => (
            OpenAiCompatibleConfig::from_env().map_err(missing_config)?,
            true,
            None,
        ),
    };
    let provider = OpenAiCompatibleProvider::new(config)?
        .with_include_usage(include_usage)
        .with_reasoning_effort(reasoning_effort);
    let mut overlap =
        (input.action == AiAction::Generate).then(|| Overlap::new(input.document_at_caret().0));
    let mut upstream = provider.stream(input);
    let body_stream = async_stream::stream! {
        while let Some(event) = upstream.next().await {
            match (&mut overlap, event) {
                (Some(overlap), StreamEvent::Delta { delta }) => {
                    let text = overlap.push(&plain_text(&delta), false);
                    if !text.is_empty() {
                        let delta = into_delta(&TaggedText::generated(&text));
                        yield Ok::<Bytes, Infallible>(Bytes::from(event_to_ndjson_line(&StreamEvent::Delta { delta })));
                    }
                }
                (Some(overlap), event) => {
                    let text = overlap.push("", true);
                    if !text.is_empty() {
                        let delta = into_delta(&TaggedText::generated(&text));
                        yield Ok::<Bytes, Infallible>(Bytes::from(event_to_ndjson_line(&StreamEvent::Delta { delta })));
                    }
                    yield Ok::<Bytes, Infallible>(Bytes::from(event_to_ndjson_line(&event)));
                }
                (None, event) => yield Ok::<Bytes, Infallible>(Bytes::from(event_to_ndjson_line(&event))),
            }
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(body_stream))
        .map_err(|err| Error::Internal(format!("failed to build AI response: {err}")))
}

fn missing_config(error: Error) -> Error {
    match error {
        Error::AiProviderConfig(ref detail)
            if (detail == "missing required environment variable QUILLAI_AI_MODEL"
                && std::env::var_os("QUILLAI_AI_MODEL").is_none())
                || detail == "missing required environment variable QUILLAI_AI_API_KEY" =>
        {
            Error::AiNotConfigured
        }
        error => error,
    }
}
