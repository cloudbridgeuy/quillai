use axum::{
    extract::{rejection::JsonRejection, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use sqlx::{sqlite::SqlitePool, SqliteConnection};

use crate::error::Error;
use crate::manuscripts::{
    json_input,
    lore::{
        handlers::{parse_input, DeleteQuery},
        models::{LoreInput, UniverseLoreEntry},
        states::{self, LoreState},
        store,
    },
    series::models::{CreateSeries, Series as Universe, UpdateSeries},
};

/// Universe lore entry with its active states, ordered by point position.
#[derive(Debug, Serialize)]
pub struct LoreWithStates {
    #[serde(flatten)]
    entry: UniverseLoreEntry,
    states: Vec<LoreState>,
}

async fn with_states(
    connection: &mut SqliteConnection,
    entry: UniverseLoreEntry,
) -> Result<LoreWithStates, Error> {
    let states = states::for_entry(connection, entry.id).await?;
    Ok(LoreWithStates { entry, states })
}

pub(crate) async fn existing_universe(
    connection: &mut SqliteConnection,
    id: i64,
) -> Result<(), Error> {
    sqlx::query_scalar!(
        "SELECT id FROM universes WHERE id = ? AND deleted_at IS NULL",
        id
    )
    .fetch_optional(connection)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(())
}

pub async fn list_universes(State(pool): State<SqlitePool>) -> Result<Json<Vec<Universe>>, Error> {
    let universes = sqlx::query_as!(
        Universe,
        "SELECT id, title, position FROM universes WHERE deleted_at IS NULL ORDER BY position, id"
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(universes))
}

pub async fn create_universe(
    State(pool): State<SqlitePool>,
    input: Result<Json<CreateSeries>, JsonRejection>,
) -> Result<(StatusCode, Json<Universe>), Error> {
    let input = json_input(input)?;
    input.validate()?;
    let universe = sqlx::query_as!(
        Universe,
        "INSERT INTO universes (title, position) VALUES (?, ?) RETURNING id, title, position",
        input.title,
        input.position,
    )
    .fetch_one(&pool)
    .await?;
    Ok((StatusCode::CREATED, Json(universe)))
}

pub async fn get_universe(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Universe>, Error> {
    let universe = sqlx::query_as!(
        Universe,
        "SELECT id, title, position FROM universes WHERE id = ? AND deleted_at IS NULL",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(universe))
}

pub async fn update_universe(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    input: Result<Json<UpdateSeries>, JsonRejection>,
) -> Result<Json<Universe>, Error> {
    let input = json_input(input)?;
    input.validate()?;
    let universe = sqlx::query_as!(
        Universe,
        "UPDATE universes SET title = COALESCE(?, title), position = COALESCE(?, position) WHERE id = ? AND deleted_at IS NULL RETURNING id, title, position",
        input.title,
        input.position,
        id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(universe))
}

pub async fn delete_universe(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Query(query): Query<DeleteQuery>,
) -> Result<StatusCode, Error> {
    let sql = if query.permanent {
        "DELETE FROM universes WHERE id = ?"
    } else {
        "UPDATE universes SET deleted_at = CURRENT_TIMESTAMP WHERE id = ? AND deleted_at IS NULL"
    };
    if sqlx::query(sql)
        .bind(id)
        .execute(&pool)
        .await?
        .rows_affected()
        == 0
    {
        return Err(Error::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_universe(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Universe>, Error> {
    let mut transaction = pool.begin().await?;
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM universes WHERE id = ? AND deleted_at IS NOT NULL",
    )
    .bind(id)
    .fetch_optional(&mut *transaction)
    .await?;
    if exists.is_none() {
        return Err(Error::NotFound);
    }
    crate::manuscripts::pieces::trash::restore_universe(&mut transaction, id).await?;
    transaction.commit().await?;
    get_universe(State(pool), Path(id)).await
}

pub async fn list_lore(
    State(pool): State<SqlitePool>,
    Path(universe_id): Path<i64>,
) -> Result<Json<Vec<LoreWithStates>>, Error> {
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let entries = store::list(&mut transaction, universe_id).await?;
    let mut all = states::for_universe(&mut transaction, universe_id).await?;
    transaction.commit().await?;
    Ok(Json(
        entries
            .into_iter()
            .map(|entry| {
                let states = all.remove(&entry.id).unwrap_or_default();
                LoreWithStates { entry, states }
            })
            .collect(),
    ))
}

pub async fn create_lore(
    State(pool): State<SqlitePool>,
    Path(universe_id): Path<i64>,
    input: Result<Json<LoreInput>, JsonRejection>,
) -> Result<(StatusCode, Json<UniverseLoreEntry>), Error> {
    let input = parse_input(input, true)?;
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let entry = store::create(&mut transaction, universe_id, input).await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

pub async fn get_lore(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id)): Path<(i64, i64)>,
) -> Result<Json<LoreWithStates>, Error> {
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let entry = store::get(&mut transaction, universe_id, lore_id).await?;
    let entry = with_states(&mut transaction, entry).await?;
    transaction.commit().await?;
    Ok(Json(entry))
}

pub async fn update_lore(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id)): Path<(i64, i64)>,
    input: Result<Json<LoreInput>, JsonRejection>,
) -> Result<Json<LoreWithStates>, Error> {
    let input = parse_input(input, false)?;
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let entry = store::update(&mut transaction, universe_id, lore_id, input).await?;
    let entry = with_states(&mut transaction, entry).await?;
    transaction.commit().await?;
    Ok(Json(entry))
}

pub async fn delete_lore(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id)): Path<(i64, i64)>,
    Query(query): Query<DeleteQuery>,
) -> Result<StatusCode, Error> {
    let mut connection = pool.acquire().await?;
    existing_universe(&mut connection, universe_id).await?;
    if query.permanent {
        store::permanent_delete(&mut connection, universe_id, lore_id).await?;
    } else {
        store::delete(&mut connection, universe_id, lore_id).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_lore(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id)): Path<(i64, i64)>,
) -> Result<Json<LoreWithStates>, Error> {
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let entry = store::restore(&mut transaction, universe_id, lore_id).await?;
    let entry = with_states(&mut transaction, entry).await?;
    transaction.commit().await?;
    Ok(Json(entry))
}
