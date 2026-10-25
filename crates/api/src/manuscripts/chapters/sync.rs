use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    Json,
};
use sqlx::SqlitePool;

use super::models::{MissedDeltas, SubmitDelta};
use crate::{chapter_revisions::Revision, error::Error, manuscripts::leaf_sync};

async fn chapter_piece(
    pool: &SqlitePool,
    (series_id, book_id, chapter_id): (i64, i64, i64),
) -> Result<i64, Error> {
    sqlx::query_scalar!(
        r#"SELECT piece_id AS "piece_id!" FROM legacy_chapters WHERE id = ? AND book_id = ? AND series_id = ?"#,
        chapter_id,
        book_id,
        series_id,
    )
    .fetch_optional(pool)
    .await?
    .ok_or(Error::NotFound)
}

pub async fn get_deltas(
    State(pool): State<SqlitePool>,
    Path(path): Path<(i64, i64, i64)>,
    query: Result<Query<MissedDeltas>, QueryRejection>,
) -> Result<Json<Vec<Revision>>, Error> {
    let Query(query) = query.map_err(|error| Error::Validation(error.body_text()))?;
    let piece_id = chapter_piece(&pool, path).await?;
    let (_, revisions) = leaf_sync::history(&pool, piece_id, query.since).await?;
    Ok(Json(revisions))
}

pub async fn post_delta(
    State(pool): State<SqlitePool>,
    Path(path): Path<(i64, i64, i64)>,
    input: Result<Json<SubmitDelta>, JsonRejection>,
) -> Result<Json<Revision>, Error> {
    let Json(input) = input.map_err(|error| Error::Validation(error.body_text()))?;
    let piece_id = chapter_piece(&pool, path).await?;
    let revision = leaf_sync::submit(&pool, piece_id, input.base_rev, &input.delta).await?;
    Ok(Json(revision))
}
