use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, SqliteConnection};

use super::store as lore_store;
use crate::error::Error;
use crate::manuscripts::timeline::store as timeline_store;
use crate::manuscripts::universes::handlers::existing_universe;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoreState {
    pub id: i64,
    pub name: String,
    pub point_id: i64,
    pub visible_text: String,
    pub hidden_notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateState {
    pub name: String,
    pub point_id: i64,
    pub visible_text: Option<String>,
    pub hidden_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    pub name: Option<String>,
    pub point_id: Option<i64>,
    pub visible_text: Option<String>,
    pub hidden_notes: Option<String>,
}

fn blank(name: &str) -> Result<(), Error> {
    if name.trim().is_empty() {
        return Err(Error::Validation("name must not be blank".into()));
    }
    Ok(())
}

fn body<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, Error> {
    match input {
        Ok(Json(value)) => Ok(value),
        Err(JsonRejection::JsonDataError(error)) => Err(Error::Validation(error.body_text())),
        Err(rejection) => Err(Error::from_json_rejection(rejection)),
    }
}

/// The unique index on (entry, point) among active states is the single
/// source of truth for duplicates; its violation becomes 409.
fn conflict(error: sqlx::Error) -> Error {
    match &error {
        sqlx::Error::Database(db) if db.is_unique_violation() => Error::Conflict,
        _ => error.into(),
    }
}

/// Active states of one entry, ordered by point position, point id, state id.
pub async fn for_entry(
    connection: &mut SqliteConnection,
    lore_id: i64,
) -> Result<Vec<LoreState>, Error> {
    Ok(sqlx::query_as!(
        LoreState,
        r#"SELECT s.id AS "id!", s.name, s.point_id, s.visible_text, s.hidden_notes FROM lore_entry_states s JOIN timeline_points p ON p.id = s.point_id WHERE s.lore_entry_id = ? AND s.deleted_at IS NULL ORDER BY p.position, p.id, s.id"#,
        lore_id,
    )
    .fetch_all(connection)
    .await?)
}

/// Active states of every active entry in a universe, keyed by entry id.
pub async fn for_universe(
    connection: &mut SqliteConnection,
    universe_id: i64,
) -> Result<std::collections::HashMap<i64, Vec<LoreState>>, Error> {
    let rows = sqlx::query!(
        r#"SELECT s.lore_entry_id, s.id AS "id!", s.name, s.point_id, s.visible_text, s.hidden_notes FROM lore_entry_states s JOIN timeline_points p ON p.id = s.point_id JOIN lore_entries e ON e.id = s.lore_entry_id WHERE e.universe_id = ? AND e.deleted_at IS NULL AND s.deleted_at IS NULL ORDER BY p.position, p.id, s.id"#,
        universe_id,
    )
    .fetch_all(connection)
    .await?;
    let mut map: std::collections::HashMap<i64, Vec<LoreState>> = Default::default();
    for row in rows {
        map.entry(row.lore_entry_id).or_default().push(LoreState {
            id: row.id,
            name: row.name,
            point_id: row.point_id,
            visible_text: row.visible_text,
            hidden_notes: row.hidden_notes,
        });
    }
    Ok(map)
}

async fn get_active(
    connection: &mut SqliteConnection,
    lore_id: i64,
    state_id: i64,
) -> Result<LoreState, Error> {
    sqlx::query_as!(
        LoreState,
        r#"SELECT id AS "id!", name, point_id, visible_text, hidden_notes FROM lore_entry_states WHERE id = ? AND lore_entry_id = ? AND deleted_at IS NULL"#,
        state_id,
        lore_id,
    )
    .fetch_optional(connection)
    .await?
    .ok_or(Error::NotFound)
}

/// Require an active universe and an active entry inside it; returns the universe id.
async fn scope(
    connection: &mut SqliteConnection,
    universe_id: i64,
    lore_id: i64,
) -> Result<i64, Error> {
    existing_universe(connection, universe_id).await?;
    lore_store::get(connection, universe_id, lore_id).await?;
    Ok(universe_id)
}

pub async fn list_states(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id)): Path<(i64, i64)>,
) -> Result<Json<Vec<LoreState>>, Error> {
    let mut tx = pool.begin().await?;
    scope(&mut tx, universe_id, lore_id).await?;
    let states = for_entry(&mut tx, lore_id).await?;
    tx.commit().await?;
    Ok(Json(states))
}

pub async fn create_state(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id)): Path<(i64, i64)>,
    input: Result<Json<CreateState>, JsonRejection>,
) -> Result<(StatusCode, Json<LoreState>), Error> {
    let input = body(input)?;
    blank(&input.name)?;
    let mut tx = pool.begin().await?;
    let universe_id = scope(&mut tx, universe_id, lore_id).await?;
    timeline_store::check_for_piece(&mut tx, input.point_id, universe_id).await?;
    let state = sqlx::query_as!(
        LoreState,
        r#"INSERT INTO lore_entry_states (lore_entry_id, name, point_id, visible_text, hidden_notes) VALUES (?, ?, ?, COALESCE(?, ''), COALESCE(?, '')) RETURNING id AS "id!", name, point_id, visible_text, hidden_notes"#,
        lore_id,
        input.name,
        input.point_id,
        input.visible_text,
        input.hidden_notes,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(state)))
}

pub async fn update_state(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id, state_id)): Path<(i64, i64, i64)>,
    input: Result<Json<UpdateState>, JsonRejection>,
) -> Result<Json<LoreState>, Error> {
    let input = body(input)?;
    if let Some(name) = &input.name {
        blank(name)?;
    }
    let mut tx = pool.begin().await?;
    let universe_id = scope(&mut tx, universe_id, lore_id).await?;
    get_active(&mut tx, lore_id, state_id).await?;
    if let Some(point_id) = input.point_id {
        timeline_store::check_for_piece(&mut tx, point_id, universe_id).await?;
    }
    let state = sqlx::query_as!(
        LoreState,
        r#"UPDATE lore_entry_states SET name = COALESCE(?, name), point_id = COALESCE(?, point_id), visible_text = COALESCE(?, visible_text), hidden_notes = COALESCE(?, hidden_notes) WHERE id = ? RETURNING id AS "id!", name, point_id, visible_text, hidden_notes"#,
        input.name,
        input.point_id,
        input.visible_text,
        input.hidden_notes,
        state_id,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict)?;
    tx.commit().await?;
    Ok(Json(state))
}

pub async fn delete_state(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id, state_id)): Path<(i64, i64, i64)>,
) -> Result<StatusCode, Error> {
    let mut tx = pool.begin().await?;
    scope(&mut tx, universe_id, lore_id).await?;
    get_active(&mut tx, lore_id, state_id).await?;
    sqlx::query!(
        "UPDATE lore_entry_states SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?",
        state_id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_state(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id, state_id)): Path<(i64, i64, i64)>,
) -> Result<Json<LoreState>, Error> {
    let mut tx = pool.begin().await?;
    scope(&mut tx, universe_id, lore_id).await?;
    let result = sqlx::query!(
        "UPDATE lore_entry_states SET deleted_at = NULL WHERE id = ? AND lore_entry_id = ? AND deleted_at IS NOT NULL",
        state_id,
        lore_id,
    )
    .execute(&mut *tx)
    .await
    .map_err(conflict)?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    let state = get_active(&mut tx, lore_id, state_id).await?;
    tx.commit().await?;
    Ok(Json(state))
}
