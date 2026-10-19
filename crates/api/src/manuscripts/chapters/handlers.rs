use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use sqlx::sqlite::SqlitePool;

use super::models::{Chapter, ChapterRow, CreateChapter, UpdateChapter};
use crate::chapter_revisions::load;
use crate::error::Error;

async fn rebuilt(
    connection: &mut sqlx::SqliteConnection,
    row: ChapterRow,
) -> Result<Chapter, Error> {
    let head = load(connection, row.id)
        .await
        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    let mut chapter = Chapter::try_from(row)?;
    chapter.content = head.delta;
    chapter.head_rev = head.head_rev;
    Ok(chapter)
}

pub async fn list_chapters(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id)): Path<(i64, i64)>,
) -> Result<Json<Vec<Chapter>>, Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query!(
        "SELECT id FROM books WHERE series_id = ? AND id = ?",
        series_id,
        book_id
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(Error::NotFound)?;
    let rows = sqlx::query_as!(
        ChapterRow,
        r#"SELECT id AS "id!", book_id, title, position, kind, content, head_rev FROM chapters WHERE book_id = ? ORDER BY position, id"#,
        book_id,
    )
    .fetch_all(&mut *transaction)
    .await?;
    let mut chapters = Vec::with_capacity(rows.len());
    for row in rows {
        chapters.push(rebuilt(&mut transaction, row).await?);
    }
    transaction.commit().await?;
    Ok(Json(chapters))
}

pub async fn create_chapter(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id)): Path<(i64, i64)>,
    input: Result<Json<CreateChapter>, JsonRejection>,
) -> Result<(StatusCode, Json<Chapter>), Error> {
    let Json(input) = input.map_err(|error| Error::Validation(error.body_text()))?;
    input.validate()?;
    let content = input.content.to_json();
    let chapter = sqlx::query_as!(
        ChapterRow,
        "INSERT INTO chapters (book_id, title, position, kind, content) SELECT id, ?, ?, ?, ? FROM books WHERE series_id = ? AND id = ? RETURNING id, book_id, title, position, kind, content, head_rev",
        input.title,
        input.position,
        input.kind,
        content,
        series_id,
        book_id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok((StatusCode::CREATED, Json(chapter.try_into()?)))
}

pub async fn get_chapter(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id, chapter_id)): Path<(i64, i64, i64)>,
) -> Result<Json<Chapter>, Error> {
    let mut transaction = pool.begin().await?;
    let chapter = sqlx::query_as!(
        ChapterRow,
        "SELECT id, book_id, title, position, kind, content, head_rev FROM chapters WHERE id = ? AND book_id = ? AND EXISTS (SELECT 1 FROM books WHERE id = ? AND series_id = ?)",
        chapter_id,
        book_id,
        book_id,
        series_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(Error::NotFound)?;
    let chapter = rebuilt(&mut transaction, chapter).await?;
    transaction.commit().await?;
    Ok(Json(chapter))
}

pub async fn update_chapter(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id, chapter_id)): Path<(i64, i64, i64)>,
    input: Result<Json<UpdateChapter>, JsonRejection>,
) -> Result<Json<Chapter>, Error> {
    let Json(input) = input.map_err(|error| Error::Validation(error.body_text()))?;
    input.validate()?;
    let mut transaction = pool.begin().await?;
    let chapter = sqlx::query_as!(
        ChapterRow,
        "UPDATE chapters SET title = COALESCE(?, title), position = COALESCE(?, position), kind = COALESCE(?, kind) WHERE id = ? AND book_id = ? AND EXISTS (SELECT 1 FROM books WHERE id = ? AND series_id = ?) RETURNING id, book_id, title, position, kind, content, head_rev",
        input.title,
        input.position,
        input.kind,
        chapter_id,
        book_id,
        book_id,
        series_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(Error::NotFound)?;
    let chapter = rebuilt(&mut transaction, chapter).await?;
    transaction.commit().await?;
    Ok(Json(chapter))
}

pub async fn delete_chapter(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id, chapter_id)): Path<(i64, i64, i64)>,
) -> Result<StatusCode, Error> {
    let result = sqlx::query!(
        "DELETE FROM chapters WHERE id = ? AND book_id = ? AND EXISTS (SELECT 1 FROM books WHERE id = ? AND series_id = ?)",
        chapter_id,
        book_id,
        book_id,
        series_id,
    )
    .execute(&pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
