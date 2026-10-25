use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::sqlite::SqlitePool;

use super::models::{Book, CreateBook, UpdateBook};
use crate::error::Error;

pub async fn list_books(
    State(pool): State<SqlitePool>,
    Path(series_id): Path<i64>,
) -> Result<Json<Vec<Book>>, Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query!("SELECT id FROM legacy_series WHERE id = ?", series_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(Error::NotFound)?;
    let books = sqlx::query_as!(
        Book,
        r#"SELECT id AS "id!", series_id AS "series_id!", title AS "title!", position AS "position!" FROM legacy_books WHERE series_id = ? ORDER BY position, id"#,
        series_id,
    )
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Json(books))
}

pub async fn create_book(
    State(pool): State<SqlitePool>,
    Path(series_id): Path<i64>,
    Json(input): Json<CreateBook>,
) -> Result<(StatusCode, Json<Book>), Error> {
    input.validate()?;
    let mut transaction = pool.begin().await?;
    let piece_id = sqlx::query_scalar!(
        "INSERT INTO pieces (universe_id, container_id, title, position, label, format, shape) SELECT universe_id, id, ?, ?, 'novel', 'prose', 'group' FROM legacy_series WHERE id = ? RETURNING id",
        input.title,
        input.position,
        series_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(Error::NotFound)?;
    let book = sqlx::query_as!(
        Book,
        r#"SELECT id AS "id!", series_id AS "series_id!", title AS "title!", position AS "position!" FROM legacy_books WHERE piece_id = ?"#,
        piece_id,
    )
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(book)))
}

pub async fn get_book(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id)): Path<(i64, i64)>,
) -> Result<Json<Book>, Error> {
    let book = sqlx::query_as!(
        Book,
        r#"SELECT id AS "id!", series_id AS "series_id!", title AS "title!", position AS "position!" FROM legacy_books WHERE series_id = ? AND id = ?"#,
        series_id,
        book_id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(book))
}

pub async fn update_book(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id)): Path<(i64, i64)>,
    Json(input): Json<UpdateBook>,
) -> Result<Json<Book>, Error> {
    input.validate()?;
    let book = sqlx::query_as!(
        Book,
        r#"UPDATE pieces SET title = COALESCE(?, title), position = COALESCE(?, position) WHERE id = (SELECT piece_id FROM legacy_books WHERE series_id = ? AND id = ?) RETURNING legacy_book_id AS "id!", container_id AS "series_id!", title, position"#,
        input.title,
        input.position,
        series_id,
        book_id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(book))
}

pub async fn delete_book(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id)): Path<(i64, i64)>,
) -> Result<StatusCode, Error> {
    let result = sqlx::query!(
        "DELETE FROM pieces WHERE id = (SELECT piece_id FROM legacy_books WHERE series_id = ? AND id = ?)",
        series_id,
        book_id,
    )
    .execute(&pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
