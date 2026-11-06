use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

use super::models::UniverseLoreEntry;
use super::states::{self, LoreState};
use super::store as lore_store;
use crate::ai::catalog::{ModelCatalog, ModelKind};
use crate::ai::config;
use crate::ai::images::{self, ImageRequest, IMAGE_MIME};
use crate::ai::lore::{parse_lore, LoreKind};
use crate::ai::models::AiAction;
use crate::error::Error;
use crate::manuscripts::timeline::{models::TimelinePoint, store as timeline_store};
use crate::manuscripts::universes::handlers::existing_universe;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateImage {
    pub timeline_point_id: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreImage {
    pub id: i64,
    pub lore_entry_id: i64,
    pub timeline_point_id: Option<i64>,
    pub prompt: String,
    pub model: String,
    pub size: String,
    pub quality: String,
    pub mime: String,
    pub cost: Option<f64>,
    pub created_at: String,
}

pub fn active_state<'a>(
    point: Option<&TimelinePoint>,
    states: &'a [LoreState],
    points: &[TimelinePoint],
) -> Option<&'a LoreState> {
    let point = point?;
    states
        .iter()
        .filter_map(|state| {
            points
                .iter()
                .find(|at| at.id == state.point_id && at.position <= point.position)
                .map(|at| (at, state))
        })
        .max_by_key(|(at, state)| (at.position, at.id, state.id))
        .map(|(_, state)| state)
}

pub fn image_prompt(entry: &UniverseLoreEntry, state: Option<&LoreState>) -> Result<String, Error> {
    let visible_text = state.map_or(entry.visible_text.as_str(), |state| &state.visible_text);
    let lore = parse_lore(json!([{
        "name": entry.name,
        "kind": entry.kind,
        "visible_text": visible_text,
        "level": entry.level,
    }]))?;
    let entry = lore
        .first()
        .ok_or_else(|| Error::Validation("lore entry is set to Leave out".into()))?;
    let preamble = match entry.kind {
        LoreKind::Character => "Head-and-shoulders portrait",
        LoreKind::Place => "Establishing view",
        LoreKind::Term | LoreKind::Note => "Illustration",
    };
    let text = entry.visible_text.trim();
    let separator = if text.is_empty() { "" } else { " " };
    Ok(format!(
        "{preamble}. Subject: {} ({}). {text}{separator}No text or lettering.",
        entry.name,
        entry.kind.as_str()
    ))
}

async fn scope(
    pool: &SqlitePool,
    universe_id: i64,
    lore_id: i64,
) -> Result<UniverseLoreEntry, Error> {
    let mut connection = pool.acquire().await?;
    existing_universe(&mut connection, universe_id).await?;
    lore_store::get(&mut connection, universe_id, lore_id).await
}

fn image_model_not_configured() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({
            "error": "AI image model is not configured: set image_model in Settings or QUILLAI_AI_IMAGE_MODEL",
            "code": Error::AiNotConfigured.error_code(),
        })),
    )
        .into_response()
}

pub async fn create_image(
    State(pool): State<SqlitePool>,
    catalog: Option<Extension<Arc<ModelCatalog>>>,
    Path((universe_id, lore_id)): Path<(i64, i64)>,
    input: Result<Json<CreateImage>, JsonRejection>,
) -> Result<Response, Error> {
    let Json(input) = input.map_err(Error::from_json_rejection)?;
    let entry = scope(&pool, universe_id, lore_id).await?;
    let prompt = {
        let mut connection = pool.acquire().await?;
        let points = timeline_store::list(&mut connection, universe_id).await?;
        let point = input
            .timeline_point_id
            .map(|id| {
                points
                    .iter()
                    .find(|point| point.id == id)
                    .ok_or_else(|| Error::Validation("timeline point does not exist".into()))
            })
            .transpose()?;
        let states = states::for_entry(&mut connection, lore_id).await?;
        image_prompt(&entry, active_state(point, &states, &points))?
    };
    let resolved = config::resolve(crate::settings::ai::load(&pool).await?, |key| {
        std::env::var(key).ok()
    });
    if resolved.model(AiAction::Image).is_none() {
        return Ok(image_model_not_configured());
    }
    let config = resolved.provider_config(AiAction::Image, None)?;
    let listed = catalog
        .as_ref()
        .and_then(|catalog| catalog.find(&config.model))
        .filter(|model| model.kind == ModelKind::Image);
    let size = listed
        .and_then(|model| model.size.as_deref())
        .unwrap_or("1024x1024");
    let quality = listed
        .and_then(|model| model.quality.as_deref())
        .unwrap_or("medium");
    let cost = listed.and_then(|model| model.price_per_image);
    let bytes = images::generate(
        &config,
        ImageRequest {
            prompt: &prompt,
            size,
            quality,
        },
    )
    .await?;
    let image = sqlx::query_as!(
        LoreImage,
        r#"INSERT INTO lore_images (lore_entry_id, timeline_point_id, prompt, model, size, quality, mime, bytes, cost) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id AS "id!", lore_entry_id, timeline_point_id, prompt, model, size, quality, mime, cost, created_at"#,
        lore_id,
        input.timeline_point_id,
        prompt,
        config.model,
        size,
        quality,
        IMAGE_MIME,
        bytes,
        cost,
    )
    .fetch_one(&pool)
    .await?;
    Ok((StatusCode::CREATED, Json(image)).into_response())
}

pub async fn list_images(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id)): Path<(i64, i64)>,
) -> Result<Json<Vec<LoreImage>>, Error> {
    scope(&pool, universe_id, lore_id).await?;
    Ok(Json(
        sqlx::query_as!(
            LoreImage,
            r#"SELECT id AS "id!", lore_entry_id, timeline_point_id, prompt, model, size, quality, mime, cost, created_at FROM lore_images WHERE lore_entry_id = ? ORDER BY id"#,
            lore_id,
        )
        .fetch_all(&pool)
        .await?,
    ))
}

pub async fn get_image(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id, image_id)): Path<(i64, i64, i64)>,
) -> Result<Response, Error> {
    scope(&pool, universe_id, lore_id).await?;
    let image = sqlx::query!(
        "SELECT mime, bytes FROM lore_images WHERE id = ? AND lore_entry_id = ?",
        image_id,
        lore_id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok((
        [
            (header::CONTENT_TYPE, image.mime),
            (
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable".into(),
            ),
        ],
        image.bytes,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::types::Json as SqlJson;

    fn entry(level: &str) -> UniverseLoreEntry {
        UniverseLoreEntry {
            id: 1,
            universe_id: 1,
            kind: "character".into(),
            name: "Mara".into(),
            keywords: SqlJson(vec![]),
            aliases: SqlJson(vec![]),
            visible_text: "A tall smuggler.".into(),
            hidden_notes: "SECRET twin".into(),
            pinned: false,
            level: level.into(),
        }
    }

    fn point(id: i64, position: i64) -> TimelinePoint {
        TimelinePoint {
            id,
            universe_id: 1,
            name: format!("p{id}"),
            position,
        }
    }

    fn state(id: i64, point_id: i64, text: &str) -> LoreState {
        LoreState {
            id,
            name: format!("s{id}"),
            point_id,
            visible_text: text.into(),
            hidden_notes: "SECRET state".into(),
        }
    }

    #[test]
    fn active_state_is_latest_at_or_before_the_point() {
        let points = [point(1, 10), point(2, 20), point(3, 30)];
        let states = [
            state(1, 1, "young"),
            state(2, 2, "scarred"),
            state(3, 3, "old"),
        ];
        let pick = |point| active_state(point, &states, &points).map(|state| state.id);
        assert_eq!(pick(Some(&points[1])), Some(2));
        let (between, before) = (point(9, 15), point(9, 5));
        assert_eq!(pick(Some(&between)), Some(1));
        assert_eq!(pick(Some(&before)), None);
        assert_eq!(pick(None), None);
    }

    #[test]
    fn image_prompt_uses_visible_text_and_never_hidden_notes() {
        let prompt = image_prompt(&entry("include"), None).unwrap();
        assert!(prompt.contains("Subject: Mara (character)"), "{prompt}");
        assert!(prompt.contains("A tall smuggler."), "{prompt}");
        assert!(prompt.contains("No text or lettering."), "{prompt}");
        let scarred = state(2, 2, "Scarred and grey.");
        let prompt = image_prompt(&entry("known_dont_reveal"), Some(&scarred)).unwrap();
        assert!(
            prompt.contains("Scarred and grey.") && !prompt.contains("smuggler"),
            "{prompt}"
        );
        assert!(!prompt.contains("SECRET"), "{prompt}");
        assert!(matches!(
            image_prompt(&entry("leave_out"), None),
            Err(Error::Validation(_))
        ));
    }
}
