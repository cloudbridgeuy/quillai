use axum::{
    extract::{rejection::JsonRejection, Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::{sqlite::SqlitePool, SqliteConnection};

use super::models::{LoreEntry, LoreInput};
use super::store;
use crate::error::Error;

pub fn parse_input(
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

async fn series_universe(connection: &mut SqliteConnection, series_id: i64) -> Result<i64, Error> {
    sqlx::query_scalar!(
        r#"SELECT universe_id AS "universe_id!" FROM legacy_series WHERE id = ?"#,
        series_id
    )
    .fetch_optional(connection)
    .await?
    .ok_or(Error::NotFound)
}

pub async fn list_lore(
    State(pool): State<SqlitePool>,
    Path(series_id): Path<i64>,
) -> Result<Json<Vec<LoreEntry>>, Error> {
    let mut transaction = pool.begin().await?;
    let universe_id = series_universe(&mut transaction, series_id).await?;
    let entries = store::list(&mut transaction, universe_id).await?;
    transaction.commit().await?;
    Ok(Json(
        entries
            .into_iter()
            .map(|entry| LoreEntry::in_series(entry, series_id))
            .collect(),
    ))
}

pub async fn create_lore(
    State(pool): State<SqlitePool>,
    Path(series_id): Path<i64>,
    input: Result<Json<LoreInput>, JsonRejection>,
) -> Result<(StatusCode, Json<LoreEntry>), Error> {
    let input = parse_input(input, true)?;
    let mut transaction = pool.begin().await?;
    let universe_id = series_universe(&mut transaction, series_id).await?;
    let entry = store::create(&mut transaction, universe_id, input).await?;
    transaction.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(LoreEntry::in_series(entry, series_id)),
    ))
}

pub async fn get_lore(
    State(pool): State<SqlitePool>,
    Path((series_id, lore_id)): Path<(i64, i64)>,
) -> Result<Json<LoreEntry>, Error> {
    let mut transaction = pool.begin().await?;
    let universe_id = series_universe(&mut transaction, series_id).await?;
    let entry = store::get(&mut transaction, universe_id, lore_id).await?;
    transaction.commit().await?;
    Ok(Json(LoreEntry::in_series(entry, series_id)))
}

pub async fn update_lore(
    State(pool): State<SqlitePool>,
    Path((series_id, lore_id)): Path<(i64, i64)>,
    input: Result<Json<LoreInput>, JsonRejection>,
) -> Result<Json<LoreEntry>, Error> {
    let input = parse_input(input, false)?;
    let mut transaction = pool.begin().await?;
    let universe_id = series_universe(&mut transaction, series_id).await?;
    let entry = store::update(&mut transaction, universe_id, lore_id, input).await?;
    transaction.commit().await?;
    Ok(Json(LoreEntry::in_series(entry, series_id)))
}

#[derive(serde::Deserialize, Default)]
pub struct DeleteQuery {
    #[serde(default)]
    pub permanent: bool,
}

pub async fn delete_lore(
    State(pool): State<SqlitePool>,
    Path((series_id, lore_id)): Path<(i64, i64)>,
    Query(query): Query<DeleteQuery>,
) -> Result<StatusCode, Error> {
    let mut transaction = pool.begin().await?;
    let universe_id = series_universe(&mut transaction, series_id).await?;
    if query.permanent {
        store::permanent_delete(&mut transaction, universe_id, lore_id).await?;
    } else {
        store::delete(&mut transaction, universe_id, lore_id).await?;
    }
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_lore(
    State(pool): State<SqlitePool>,
    Path((series_id, lore_id)): Path<(i64, i64)>,
) -> Result<Json<LoreEntry>, Error> {
    let mut transaction = pool.begin().await?;
    let universe_id = series_universe(&mut transaction, series_id).await?;
    let entry = store::restore(&mut transaction, universe_id, lore_id).await?;
    transaction.commit().await?;
    Ok(Json(LoreEntry::in_series(entry, series_id)))
}
