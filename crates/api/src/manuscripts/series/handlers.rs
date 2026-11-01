use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::sqlite::SqlitePool;

use super::models::{CreateSeries, Series, UpdateSeries};
use crate::error::Error;

pub async fn list_series(State(pool): State<SqlitePool>) -> Result<Json<Vec<Series>>, Error> {
    let series = sqlx::query_as!(
        Series,
        r#"SELECT id AS "id!", title AS "title!", position AS "position!" FROM legacy_series ORDER BY position, id"#
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
    let mut transaction = pool.begin().await?;
    let universe_id = sqlx::query_scalar!(
        "INSERT INTO universes (title, position) VALUES (?, ?) RETURNING id",
        input.title,
        input.position,
    )
    .fetch_one(&mut *transaction)
    .await?;
    let series = sqlx::query_as!(
        Series,
        r#"INSERT INTO containers (universe_id, title, position, kind) VALUES (?, ?, ?, 'book_series') RETURNING id AS "id!", title, position"#,
        universe_id,
        input.title,
        input.position,
    )
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(series)))
}

pub async fn get_series(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Series>, Error> {
    let series = sqlx::query_as!(
        Series,
        r#"SELECT id AS "id!", title AS "title!", position AS "position!" FROM legacy_series WHERE id = ?"#,
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
        "UPDATE containers SET title = COALESCE(?, title), position = COALESCE(?, position) WHERE id = ? AND id IN (SELECT id FROM legacy_series) RETURNING id, title, position",
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
    Query(query): Query<crate::manuscripts::lore::handlers::DeleteQuery>,
) -> Result<StatusCode, Error> {
    if query.permanent {
        let mut transaction = pool.begin().await?;
        let universe_id = sqlx::query_scalar!(
            "DELETE FROM containers WHERE id = ? AND kind = 'book_series' RETURNING universe_id",
            id
        )
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(Error::NotFound)?;
        sqlx::query!(
            "DELETE FROM universes WHERE id = ? AND NOT EXISTS (SELECT 1 FROM containers WHERE universe_id = ?) AND NOT EXISTS (SELECT 1 FROM pieces WHERE universe_id = ?)",
            universe_id,
            universe_id,
            universe_id,
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    if sqlx::query("UPDATE containers SET deleted_at = CURRENT_TIMESTAMP WHERE id = ? AND id IN (SELECT id FROM legacy_series)")
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

pub async fn restore_series(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<Series>, Error> {
    let mut transaction = pool.begin().await?;
    let universe_id = sqlx::query_scalar::<_, i64>("SELECT universe_id FROM containers WHERE id = ? AND kind = 'book_series' AND (deleted_at IS NOT NULL OR universe_id IN (SELECT id FROM universes WHERE deleted_at IS NOT NULL))").bind(id).fetch_optional(&mut *transaction).await?.ok_or(Error::NotFound)?;
    crate::manuscripts::pieces::trash::restore_universe(&mut transaction, universe_id).await?;
    crate::manuscripts::pieces::trash::restore_container(&mut transaction, id).await?;
    transaction.commit().await?;
    get_series(State(pool), Path(id)).await
}
