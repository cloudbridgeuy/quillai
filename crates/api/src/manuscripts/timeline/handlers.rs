use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use sqlx::sqlite::SqlitePool;

use super::models::{CreatePoint, TimelinePoint, UpdatePoint};
use super::store;
use crate::error::Error;
use crate::manuscripts::{json_input, universes::handlers::existing_universe};

pub async fn list_points(
    State(pool): State<SqlitePool>,
    Path(universe_id): Path<i64>,
) -> Result<Json<Vec<TimelinePoint>>, Error> {
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let points = store::list(&mut transaction, universe_id).await?;
    transaction.commit().await?;
    Ok(Json(points))
}

pub async fn create_point(
    State(pool): State<SqlitePool>,
    Path(universe_id): Path<i64>,
    input: Result<Json<CreatePoint>, JsonRejection>,
) -> Result<(StatusCode, Json<TimelinePoint>), Error> {
    let input = json_input(input)?;
    input.validate()?;
    let mut transaction = pool.begin().await?;
    existing_universe(&mut transaction, universe_id).await?;
    let point = store::create(&mut transaction, universe_id, input).await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(point)))
}

pub async fn update_point(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    input: Result<Json<UpdatePoint>, JsonRejection>,
) -> Result<Json<TimelinePoint>, Error> {
    let input = json_input(input)?;
    input.validate()?;
    let mut transaction = pool.begin().await?;
    let point = store::update(&mut transaction, id, input).await?;
    transaction.commit().await?;
    Ok(Json(point))
}

pub async fn delete_point(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, Error> {
    let mut transaction = pool.begin().await?;
    store::delete(&mut transaction, id).await?;
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_point(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<TimelinePoint>, Error> {
    let mut transaction = pool.begin().await?;
    let point = store::restore(&mut transaction, id).await?;
    transaction.commit().await?;
    Ok(Json(point))
}
