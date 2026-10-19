use std::convert::Infallible;

use axum::body::{Body, Bytes};
use axum::extract::rejection::JsonRejection;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::Json;
use futures_util::StreamExt;

use super::models::{event_to_ndjson_line, GenerateInput, GenerateRequest};
use super::provider::{AiProvider, OpenAiCompatibleProvider};
use crate::error::Error;

pub async fn generate(
    axum::extract::State(pool): axum::extract::State<sqlx::SqlitePool>,
    payload: Result<Json<GenerateRequest>, JsonRejection>,
) -> Result<Response, Error> {
    let Json(request) = payload.map_err(Error::from_json_rejection)?;
    let mut input = GenerateInput::try_from(request)?;
    input.author_guidance = Some(crate::settings::load_guidance(&pool).await?.text);
    let provider = OpenAiCompatibleProvider::from_env()?;
    let body_stream = provider
        .stream(input)
        .map(|event| Ok::<Bytes, Infallible>(Bytes::from(event_to_ndjson_line(&event))));

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(body_stream))
        .map_err(|err| Error::Internal(format!("failed to build AI response: {err}")))
}
