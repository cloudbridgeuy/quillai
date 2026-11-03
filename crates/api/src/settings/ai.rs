use crate::{
    ai::{
        catalog::ModelCatalog,
        config::{self, SavedSettings, Sources},
    },
    error::Error,
};
use axum::{
    extract::{rejection::JsonRejection, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    base_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
}

#[derive(Serialize)]
pub struct View {
    base_url: Option<String>,
    model: Option<String>,
    key_set: bool,
    sources: Sources,
}

pub async fn load(pool: &SqlitePool) -> Result<SavedSettings, Error> {
    let row = sqlx::query!("SELECT base_url, api_key, model FROM ai_settings WHERE id = 1")
        .fetch_optional(pool)
        .await?;
    Ok(row
        .map(|row| SavedSettings {
            base_url: row.base_url,
            api_key: row.api_key,
            model: row.model,
        })
        .unwrap_or_default())
}

fn view(saved: SavedSettings) -> View {
    let resolved = config::resolve(saved, |key| std::env::var(key).ok());
    View {
        base_url: resolved.base_url,
        model: resolved.model,
        key_set: resolved
            .api_key
            .as_ref()
            .is_some_and(|key| !key.trim().is_empty()),
        sources: resolved.sources,
    }
}

pub async fn get(State(pool): State<SqlitePool>) -> Result<Json<View>, Error> {
    Ok(Json(view(load(&pool).await?)))
}

pub async fn put(
    State(pool): State<SqlitePool>,
    catalog: Option<Extension<Arc<ModelCatalog>>>,
    payload: Result<Json<Update>, JsonRejection>,
) -> Result<Json<View>, Error> {
    let Json(input) = payload.map_err(Error::from_json_rejection)?;
    if let Some(url) = input.base_url.as_deref().filter(|s| !s.is_empty()) {
        let parsed =
            reqwest::Url::parse(url).map_err(|_| Error::Validation("invalid base_url".into()))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host().is_none() {
            return Err(Error::Validation("invalid base_url".into()));
        }
    }
    if let Some(model) = input.model.as_deref().filter(|s| !s.is_empty()) {
        if model.trim().is_empty()
            || catalog
                .as_ref()
                .is_some_and(|catalog| catalog.is_file() && catalog.find(model).is_none())
        {
            return Err(Error::Validation(
                "model must match a configured model id".into(),
            ));
        }
    }
    let base_set = input.base_url.is_some();
    let key_set = input.api_key.is_some();
    let model_set = input.model.is_some();
    let base = input.base_url.filter(|s| !s.is_empty());
    let key = input.api_key.filter(|s| !s.is_empty());
    let model = input.model.filter(|s| !s.is_empty());
    sqlx::query!("INSERT INTO ai_settings (id, base_url, api_key, model) VALUES (1, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET base_url = CASE WHEN ? THEN excluded.base_url ELSE ai_settings.base_url END, api_key = CASE WHEN ? THEN excluded.api_key ELSE ai_settings.api_key END, model = CASE WHEN ? THEN excluded.model ELSE ai_settings.model END", base, key, model, base_set, key_set, model_set).execute(&pool).await?;
    Ok(Json(view(load(&pool).await?)))
}
