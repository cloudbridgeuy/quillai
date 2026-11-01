use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::{sqlite::SqlitePool, SqliteConnection};

use super::models::{
    Chapter, ChapterRow, ChapterSummary, CreateChapter, ListChapters, UpdateChapter,
};
use crate::chapter_revisions::load;
use crate::error::Error;

async fn rebuilt(connection: &mut SqliteConnection, row: ChapterRow) -> Result<Chapter, Error> {
    let head = load(connection, row.piece_id)
        .await
        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    let mut chapter = Chapter::try_from(row)?;
    chapter.content = head.delta;
    chapter.head_rev = head.head_rev;
    Ok(chapter)
}

async fn chapter_row(
    connection: &mut SqliteConnection,
    piece_id: i64,
) -> Result<ChapterRow, Error> {
    Ok(sqlx::query_as!(
        ChapterRow,
        r#"SELECT id AS "id!", piece_id AS "piece_id!", book_id AS "book_id!", title AS "title!", position AS "position!", kind AS "kind!", content AS "content!", head_rev AS "head_rev!" FROM legacy_chapters WHERE piece_id = ?"#,
        piece_id,
    )
    .fetch_one(connection)
    .await?)
}

pub async fn list_chapters(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id)): Path<(i64, i64)>,
    query: Result<Query<ListChapters>, QueryRejection>,
) -> Result<Response, Error> {
    let Query(query) = query.map_err(|error| Error::Validation(error.body_text()))?;
    let mut transaction = pool.begin().await?;
    sqlx::query!(
        "SELECT id FROM legacy_books WHERE series_id = ? AND id = ?",
        series_id,
        book_id
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(Error::NotFound)?;
    if query.content.is_none() {
        let chapters = sqlx::query_as!(
            ChapterSummary,
            r#"SELECT id AS "id!", title AS "title!", position AS "position!", kind AS "kind!", head_rev AS "head_rev!" FROM legacy_chapters WHERE series_id = ? AND book_id = ? ORDER BY position, id"#,
            series_id,
            book_id,
        )
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        return Ok(Json(chapters).into_response());
    }
    let rows = sqlx::query_as!(
        ChapterRow,
        r#"SELECT id AS "id!", piece_id AS "piece_id!", book_id AS "book_id!", title AS "title!", position AS "position!", kind AS "kind!", content AS "content!", head_rev AS "head_rev!" FROM legacy_chapters WHERE series_id = ? AND book_id = ? ORDER BY position, id"#,
        series_id,
        book_id,
    )
    .fetch_all(&mut *transaction)
    .await?;
    let mut chapters = Vec::with_capacity(rows.len());
    for row in rows {
        chapters.push(rebuilt(&mut transaction, row).await?);
    }
    transaction.commit().await?;
    Ok(Json(chapters).into_response())
}

pub async fn create_chapter(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id)): Path<(i64, i64)>,
    input: Result<Json<CreateChapter>, JsonRejection>,
) -> Result<(StatusCode, Json<Chapter>), Error> {
    let Json(input) = input.map_err(|error| Error::Validation(error.body_text()))?;
    input.validate()?;
    let content = input.content.to_json();
    let mut transaction = pool.begin().await?;
    let piece_id = sqlx::query_scalar!(
        r#"INSERT INTO pieces (universe_id, parent_id, title, position, label, format, shape, content, head_rev, snapshot_rev) SELECT universe_id, piece_id, ?, ?, ?, 'prose', 'leaf', ?, 0, 0 FROM legacy_books WHERE series_id = ? AND id = ? RETURNING id AS "id!""#,
        input.title,
        input.position,
        input.kind,
        content,
        series_id,
        book_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(Error::NotFound)?;
    let chapter = chapter_row(&mut transaction, piece_id).await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(chapter.try_into()?)))
}

pub async fn get_chapter(
    State(pool): State<SqlitePool>,
    Path((series_id, book_id, chapter_id)): Path<(i64, i64, i64)>,
) -> Result<Json<Chapter>, Error> {
    let mut transaction = pool.begin().await?;
    let chapter = sqlx::query_as!(
        ChapterRow,
        r#"SELECT id AS "id!", piece_id AS "piece_id!", book_id AS "book_id!", title AS "title!", position AS "position!", kind AS "kind!", content AS "content!", head_rev AS "head_rev!" FROM legacy_chapters WHERE id = ? AND book_id = ? AND series_id = ?"#,
        chapter_id,
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
    let piece_id = sqlx::query_scalar!(
        "UPDATE pieces SET title = COALESCE(?, title), position = COALESCE(?, position), label = COALESCE(?, label) WHERE id = (SELECT piece_id FROM legacy_chapters WHERE id = ? AND book_id = ? AND series_id = ?) RETURNING id",
        input.title,
        input.position,
        input.kind,
        chapter_id,
        book_id,
        series_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(Error::NotFound)?;
    let chapter = chapter_row(&mut transaction, piece_id).await?;
    let chapter = rebuilt(&mut transaction, chapter).await?;
    transaction.commit().await?;
    Ok(Json(chapter))
}

async fn trash_piece(
    pool: &SqlitePool,
    (series_id, book_id, chapter_id): (i64, i64, i64),
) -> Result<i64, Error> {
    sqlx::query_scalar::<_, i64>("SELECT p.id FROM pieces p JOIN pieces b ON b.id = p.parent_id JOIN containers c ON c.id = b.container_id WHERE p.legacy_chapter_id = ? AND b.legacy_book_id = ? AND c.id = ? AND c.kind = 'book_series' AND b.shape = 'group' AND b.label = 'novel' AND b.format = 'prose' AND p.shape = 'leaf' AND p.label IN ('prologue', 'chapter', 'epilogue')")
        .bind(chapter_id).bind(book_id).bind(series_id).fetch_optional(pool).await?.ok_or(Error::NotFound)
}

pub async fn delete_chapter(
    State(pool): State<SqlitePool>,
    Path(path): Path<(i64, i64, i64)>,
    Query(query): Query<crate::manuscripts::lore::handlers::DeleteQuery>,
) -> Result<StatusCode, Error> {
    let id = trash_piece(&pool, path).await?;
    crate::manuscripts::pieces::trash::delete(&pool, id, query.permanent).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_chapter(
    State(pool): State<SqlitePool>,
    Path(path): Path<(i64, i64, i64)>,
) -> Result<Json<Chapter>, Error> {
    let id = trash_piece(&pool, path).await?;
    crate::manuscripts::pieces::trash::restore(&pool, id).await?;
    get_chapter(State(pool), Path(path)).await
}
