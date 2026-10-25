use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use sqlx::{sqlite::SqlitePool, SqliteConnection};

use crate::error::Error;
use crate::manuscripts::{
    json_input,
    lore::{
        handlers::parse_input,
        models::{LoreInput, UniverseLoreEntry},
        store,
    },
    series::models::{CreateSeries, Series as Universe, UpdateSeries},
};

pub(crate) async fn existing_universe(
    connection: &mut SqliteConnection,
    id: i64,
) -> Result<(), Error> {
    sqlx::query_scalar!("SELECT id FROM universes WHERE id = ?", id)
        .fetch_optional(connection)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(())
}

pub async fn list_universes(State(pool): State<SqlitePool>) -> Result<Json<Vec<Universe>>, Error> {
    let universes = sqlx::query_as!(
        Universe,
        "SELECT id, title, position FROM universes ORDER BY position, id"
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
        "SELECT id, title, position FROM universes WHERE id = ?",
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
        "UPDATE universes SET title = COALESCE(?, title), position = COALESCE(?, position) WHERE id = ? RETURNING id, title, position",
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
) -> Result<StatusCode, Error> {
    let result = sqlx::query!("DELETE FROM universes WHERE id = ?", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_lore(
    State(pool): State<SqlitePool>,
    Path(universe_id): Path<i64>,
) -> Result<Json<Vec<UniverseLoreEntry>>, Error> {
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let entries = store::list(&mut transaction, universe_id).await?;
    transaction.commit().await?;
    Ok(Json(entries))
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
) -> Result<Json<UniverseLoreEntry>, Error> {
    let mut connection = pool.acquire().await?;
    Ok(Json(
        store::get(&mut connection, universe_id, lore_id).await?,
    ))
}

pub async fn update_lore(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id)): Path<(i64, i64)>,
    input: Result<Json<LoreInput>, JsonRejection>,
) -> Result<Json<UniverseLoreEntry>, Error> {
    let input = parse_input(input, false)?;
    let mut connection = pool.acquire().await?;
    Ok(Json(
        store::update(&mut connection, universe_id, lore_id, input).await?,
    ))
}

pub async fn delete_lore(
    State(pool): State<SqlitePool>,
    Path((universe_id, lore_id)): Path<(i64, i64)>,
) -> Result<StatusCode, Error> {
    let mut connection = pool.acquire().await?;
    store::delete(&mut connection, universe_id, lore_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
