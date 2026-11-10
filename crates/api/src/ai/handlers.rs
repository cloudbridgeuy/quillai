use std::convert::Infallible;

use axum::body::{Body, Bytes};
use axum::extract::rejection::JsonRejection;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::Json;
use futures_util::StreamExt;

use super::catalog::ModelCatalog;
use super::config::{self, Source};
use super::models::{event_to_ndjson_line, AiAction, GenerateInput, GenerateRequest, StreamEvent};
use super::overlap::Overlap;
use super::provider::{AiProvider, OpenAiCompatibleProvider};
use super::segments::plain_text;
use super::tagged_text::{into_delta, TaggedText};
use crate::error::Error;

pub async fn models(
    axum::extract::State(pool): axum::extract::State<sqlx::SqlitePool>,
    catalog: Option<axum::Extension<std::sync::Arc<ModelCatalog>>>,
) -> Result<Json<serde_json::Value>, Error> {
    let saved = crate::settings::ai::load(&pool).await?;
    let saved_catalog = saved
        .generate_model
        .as_deref()
        .or(saved.edit_model.as_deref())
        .map(ModelCatalog::from_model)
        .transpose()?;
    let catalog = match (catalog.as_ref(), saved_catalog.as_ref()) {
        (Some(catalog), _) if catalog.is_file() => &*catalog.0,
        (_, Some(catalog)) => catalog,
        (Some(catalog), None) => &*catalog.0,
        (None, None) => return Err(Error::AiNotConfigured),
    };
    let preferred = |saved: Option<&str>, action| {
        saved
            .and_then(|id| catalog.find(id))
            .or_else(|| catalog.resolve(action))
            .map(|model| model.id.clone())
    };
    let models: Vec<_> = catalog
        .models()
        .iter()
        .map(|model| {
            without_nulls(serde_json::json!({
                "id": model.id,
                "label": model.label,
                "kind": model.kind,
                "input_price_per_million": model.input_price_per_million,
                "output_price_per_million": model.output_price_per_million,
                "price_per_image": model.price_per_image,
                "size": model.size,
                "quality": model.quality,
            }))
        })
        .collect();
    Ok(Json(serde_json::json!({
        "models": models,
        "defaults": without_nulls(serde_json::json!({
            "generate": preferred(saved.generate_model.as_deref(), AiAction::Generate),
            "edit": preferred(saved.edit_model.as_deref(), AiAction::Edit),
            "image": preferred(saved.image_model.as_deref(), AiAction::Image),
            "lore_complete": preferred(saved.lore_complete_model.as_deref(), AiAction::LoreComplete),
            "dialogue": preferred(saved.dialogue_model.as_deref(), AiAction::Dialogue),
        })),
    })))
}

fn without_nulls(mut value: serde_json::Value) -> serde_json::Value {
    if let Some(object) = value.as_object_mut() {
        object.retain(|_, field| !field.is_null());
    }
    value
}

pub async fn generate(
    axum::extract::State(pool): axum::extract::State<sqlx::SqlitePool>,
    catalog: Option<axum::Extension<std::sync::Arc<ModelCatalog>>>,
    payload: Result<Json<GenerateRequest>, JsonRejection>,
) -> Result<Response, Error> {
    let Json(mut request) = payload.map_err(Error::from_json_rejection)?;
    let requested_model = request.model.take();
    let saved = crate::settings::ai::load(&pool).await?;
    let resolved = config::resolve(saved, |key| std::env::var(key).ok());
    let mut input = GenerateInput::try_from(request)?;
    let action = input.action;
    let dynamic_catalog = if catalog.is_none()
        || (resolved.model_source(action) == Source::Saved
            && catalog.as_ref().is_some_and(|catalog| !catalog.is_file()))
    {
        resolved
            .model(action)
            .map(ModelCatalog::from_model)
            .transpose()?
    } else {
        None
    };
    let catalog = dynamic_catalog
        .as_ref()
        .or_else(|| catalog.as_ref().map(|catalog| &*catalog.0));
    let selected_model = requested_model
        .as_ref()
        .map(|value| {
            value
                .as_str()
                .and_then(|id| catalog.and_then(|catalog| catalog.find(id)))
                .ok_or_else(|| Error::Validation("model must match a configured model id".into()))
        })
        .transpose()?;
    input.author_guidance = Some(crate::settings::load_guidance(&pool).await?.text);
    let model = selected_model
        .or_else(|| {
            catalog.and_then(|catalog| {
                if resolved.model_source(action) == Source::Saved {
                    resolved.model(action).and_then(|id| catalog.find(id))
                } else {
                    None
                }
            })
        })
        .or_else(|| catalog.and_then(|catalog| catalog.resolve(input.action)));
    let config = resolved.provider_config(action, model.map(|model| model.id.as_str()))?;
    let include_usage = model.is_none_or(|model| model.include_usage);
    let reasoning_effort = model.and_then(|model| model.reasoning_effort);
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
