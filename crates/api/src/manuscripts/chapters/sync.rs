use std::num::NonZeroU64;

use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    Json,
};
use sqlx::SqlitePool;

use super::models::{MissedDeltas, SubmitDelta};
use crate::{
    chapter_revisions::{missed, Revision, RevisionError, RevisionStore},
    error::Error,
};

async fn history(
    pool: &SqlitePool,
    (series_id, book_id, chapter_id): (i64, i64, i64),
    since: i64,
) -> Result<(i64, Vec<Revision>), Error> {
    let mut transaction = pool.begin().await?;
    let head = sqlx::query_scalar!(
        "SELECT head_rev FROM chapters WHERE id = ? AND book_id = ? AND EXISTS (SELECT 1 FROM books WHERE id = ? AND series_id = ?)",
        chapter_id,
        book_id,
        book_id,
        series_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(Error::NotFound)?;
    let revisions = missed(&mut transaction, chapter_id, since, head).await?;
    transaction.commit().await?;
    Ok((head, revisions))
}

pub async fn get_deltas(
    State(pool): State<SqlitePool>,
    Path(path): Path<(i64, i64, i64)>,
    query: Result<Query<MissedDeltas>, QueryRejection>,
) -> Result<Json<Vec<Revision>>, Error> {
    let Query(query) = query.map_err(|error| Error::Validation(error.body_text()))?;
    let (_, revisions) = history(&pool, path, query.since).await?;
    Ok(Json(revisions))
}

pub async fn post_delta(
    State(pool): State<SqlitePool>,
    Path(path): Path<(i64, i64, i64)>,
    input: Result<Json<SubmitDelta>, JsonRejection>,
) -> Result<Json<Revision>, Error> {
    let Json(input) = input.map_err(|error| Error::Validation(error.body_text()))?;
    let store = RevisionStore::new(pool.clone(), NonZeroU64::new(100).unwrap());
    loop {
        let (head, revisions) = history(&pool, path, input.base_rev).await?;
        let delta = revisions
            .iter()
            .fold(input.delta.clone(), |delta, revision| {
                revision.delta.transform(&delta, true)
            });
        match store.append(path.2, head, &delta).await {
            Ok(head) => {
                return Ok(Json(Revision {
                    rev: head.head_rev,
                    delta,
                }))
            }
            Err(RevisionError::StaleHead) => tokio::task::yield_now().await,
            Err(error) => return Err(error.into()),
        }
    }
}
