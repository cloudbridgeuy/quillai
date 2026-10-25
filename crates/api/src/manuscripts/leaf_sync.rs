use std::num::NonZeroU64;

use quillai_delta::Delta;
use sqlx::SqlitePool;

use crate::{
    chapter_revisions::{missed, Revision, RevisionError, RevisionStore},
    error::Error,
};

const SNAPSHOT_EVERY: NonZeroU64 = NonZeroU64::new(100).unwrap();

pub async fn history(
    pool: &SqlitePool,
    piece_id: i64,
    since: i64,
) -> Result<(i64, Vec<Revision>), Error> {
    let mut transaction = pool.begin().await?;
    let head = sqlx::query_scalar!(
        r#"SELECT head_rev AS "head_rev!" FROM pieces WHERE id = ? AND shape = 'leaf'"#,
        piece_id,
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(Error::NotFound)?;
    let revisions = missed(&mut transaction, piece_id, since, head).await?;
    transaction.commit().await?;
    Ok((head, revisions))
}

pub async fn submit(
    pool: &SqlitePool,
    piece_id: i64,
    base_rev: i64,
    delta: &Delta,
) -> Result<Revision, Error> {
    let store = RevisionStore::new(pool.clone(), SNAPSHOT_EVERY);
    loop {
        let (head, revisions) = history(pool, piece_id, base_rev).await?;
        let delta = revisions.iter().fold(delta.clone(), |delta, revision| {
            revision.delta.transform(&delta, true)
        });
        match store.append(piece_id, head, &delta).await {
            Ok(head) => {
                return Ok(Revision {
                    rev: head.head_rev,
                    delta,
                })
            }
            Err(RevisionError::StaleHead) => tokio::task::yield_now().await,
            Err(error) => return Err(error.into()),
        }
    }
}
