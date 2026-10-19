use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::sqlite::SqlitePool;

use super::models::{CreateSeries, Series, UpdateSeries};
use crate::error::Error;

pub async fn list_series(State(pool): State<SqlitePool>) -> Result<Json<Vec<Series>>, Error> {
    let series = sqlx::query_as!(
        Series,
        "SELECT id, title, position FROM series ORDER BY position, id"
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(series))
}

pub async fn create_series(
    State(pool): State<SqlitePool>,
    Json(input): Json<CreateSeries>,
) -> Result<(StatusCode, Json<Series>), Error> {
    input.validate()?;
    let result = sqlx::query!(
        "INSERT INTO series (title, position) VALUES (?, ?)",
        input.title,
        input.position,
    )
    .execute(&pool)
    .await?;

    let series = Series {
        id: result.last_insert_rowid(),
        title: input.title,
        position: input.position,
    };
    Ok((StatusCode::CREATED, Json(series)))
}

pub async fn get_series(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Series>, Error> {
    let series = sqlx::query_as!(
        Series,
        "SELECT id, title, position FROM series WHERE id = ?",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(series))
}

pub async fn update_series(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateSeries>,
) -> Result<Json<Series>, Error> {
    input.validate()?;
    let series = sqlx::query_as!(
        Series,
        "UPDATE series SET title = COALESCE(?, title), position = COALESCE(?, position) WHERE id = ? RETURNING id, title, position",
        input.title,
        input.position,
        id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(series))
}

pub async fn delete_series(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, Error> {
    let result = sqlx::query!("DELETE FROM series WHERE id = ?", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
