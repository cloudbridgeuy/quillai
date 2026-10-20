use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use sqlx::{sqlite::SqlitePool, types::Json as SqlJson};

use super::models::{LoreEntry, LoreInput};
use crate::error::Error;

fn parse_input(
    input: Result<Json<LoreInput>, JsonRejection>,
    creating: bool,
) -> Result<LoreInput, Error> {
    let Json(input) = input.map_err(|rejection| match rejection {
        JsonRejection::JsonDataError(error) => Error::Validation(error.body_text()),
        rejection => Error::from_json_rejection(rejection),
    })?;
    input.validate(creating)?;
    Ok(input)
}

pub async fn list_lore(
    State(pool): State<SqlitePool>,
    Path(series_id): Path<i64>,
) -> Result<Json<Vec<LoreEntry>>, Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query!("SELECT id FROM series WHERE id = ?", series_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(Error::NotFound)?;
    let entries = sqlx::query_as!(
        LoreEntry,
        r#"SELECT id AS "id!", series_id, kind, name, keywords AS "keywords: SqlJson<Vec<String>>", aliases AS "aliases: SqlJson<Vec<String>>", visible_text, hidden_notes, pinned AS "pinned: bool", level FROM lore_entries WHERE series_id = ? ORDER BY name, id"#,
        series_id,
    )
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Json(entries))
}

pub async fn create_lore(
    State(pool): State<SqlitePool>,
    Path(series_id): Path<i64>,
    input: Result<Json<LoreInput>, JsonRejection>,
) -> Result<(StatusCode, Json<LoreEntry>), Error> {
    let input = parse_input(input, true)?;
    let keywords = input.keywords.map(SqlJson);
    let aliases = input.aliases.map(SqlJson);
    let entry = sqlx::query_as!(
        LoreEntry,
        r#"INSERT INTO lore_entries (series_id, kind, name, keywords, aliases, visible_text, hidden_notes, pinned, level) SELECT id, ?, ?, COALESCE(?, '[]'), COALESCE(?, '[]'), COALESCE(?, ''), COALESCE(?, ''), COALESCE(?, 0), COALESCE(?, 'include') FROM series WHERE id = ? RETURNING id, series_id, kind, name, keywords AS "keywords: SqlJson<Vec<String>>", aliases AS "aliases: SqlJson<Vec<String>>", visible_text, hidden_notes, pinned AS "pinned: bool", level"#,
        input.kind,
        input.name,
        keywords,
        aliases,
        input.visible_text,
        input.hidden_notes,
        input.pinned,
        input.level,
        series_id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok((StatusCode::CREATED, Json(entry)))
}

pub async fn get_lore(
    State(pool): State<SqlitePool>,
    Path((series_id, lore_id)): Path<(i64, i64)>,
) -> Result<Json<LoreEntry>, Error> {
    let entry = sqlx::query_as!(
        LoreEntry,
        r#"SELECT id, series_id, kind, name, keywords AS "keywords: SqlJson<Vec<String>>", aliases AS "aliases: SqlJson<Vec<String>>", visible_text, hidden_notes, pinned AS "pinned: bool", level FROM lore_entries WHERE series_id = ? AND id = ?"#,
        series_id,
        lore_id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(entry))
}

pub async fn update_lore(
    State(pool): State<SqlitePool>,
    Path((series_id, lore_id)): Path<(i64, i64)>,
    input: Result<Json<LoreInput>, JsonRejection>,
) -> Result<Json<LoreEntry>, Error> {
    let input = parse_input(input, false)?;
    let keywords = input.keywords.map(SqlJson);
    let aliases = input.aliases.map(SqlJson);
    let entry = sqlx::query_as!(
        LoreEntry,
        r#"UPDATE lore_entries SET kind = COALESCE(?, kind), name = COALESCE(?, name), keywords = COALESCE(?, keywords), aliases = COALESCE(?, aliases), visible_text = COALESCE(?, visible_text), hidden_notes = COALESCE(?, hidden_notes), pinned = COALESCE(?, pinned), level = COALESCE(?, level) WHERE series_id = ? AND id = ? RETURNING id, series_id, kind, name, keywords AS "keywords: SqlJson<Vec<String>>", aliases AS "aliases: SqlJson<Vec<String>>", visible_text, hidden_notes, pinned AS "pinned: bool", level"#,
        input.kind,
        input.name,
        keywords,
        aliases,
        input.visible_text,
        input.hidden_notes,
        input.pinned,
        input.level,
        series_id,
        lore_id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(entry))
}

pub async fn delete_lore(
    State(pool): State<SqlitePool>,
    Path((series_id, lore_id)): Path<(i64, i64)>,
) -> Result<StatusCode, Error> {
    let result = sqlx::query!(
        "DELETE FROM lore_entries WHERE series_id = ? AND id = ?",
        series_id,
        lore_id,
    )
    .execute(&pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
