use std::num::NonZeroU64;

use quillai_delta::{Delta, DeltaError};
use serde::Serialize;
use sqlx::{SqliteConnection, SqlitePool};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RevisionError {
    #[error("chapter not found")]
    MissingChapter,
    #[error("stale chapter head")]
    StaleHead,
    #[error("chapter revision overflow")]
    RevisionOverflow,
    #[error("incomplete chapter revision history")]
    IncompleteHistory,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Delta(#[from] DeltaError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, PartialEq)]
pub struct ChapterHead {
    pub head_rev: i64,
    pub delta: Delta,
}

#[derive(Debug, Serialize)]
pub struct Revision {
    pub rev: i64,
    pub delta: Delta,
}

pub fn rebuild(snapshot: &Delta, revisions: &[Delta]) -> Result<Delta, DeltaError> {
    revisions
        .iter()
        .try_fold(snapshot.clone(), |document, revision| {
            document.compose(revision)
        })
}

pub struct RevisionStore {
    pool: SqlitePool,
    snapshot_every: NonZeroU64,
}

impl RevisionStore {
    pub fn new(pool: SqlitePool, snapshot_every: NonZeroU64) -> Self {
        Self {
            pool,
            snapshot_every,
        }
    }

    pub async fn load(&self, chapter_id: i64) -> Result<ChapterHead, RevisionError> {
        let mut transaction = self.pool.begin().await?;
        let head = load(&mut transaction, chapter_id).await?;
        transaction.commit().await?;
        Ok(head)
    }

    pub async fn append(
        &self,
        chapter_id: i64,
        expected_head: i64,
        revision: &Delta,
    ) -> Result<ChapterHead, RevisionError> {
        let mut transaction = self.pool.begin().await?;
        let claimed = sqlx::query!(
            "UPDATE chapters SET head_rev = head_rev WHERE id = ? AND head_rev = ?",
            chapter_id,
            expected_head
        )
        .execute(&mut *transaction)
        .await?;
        if claimed.rows_affected() == 0 {
            let exists = sqlx::query_scalar!("SELECT id FROM chapters WHERE id = ?", chapter_id)
                .fetch_optional(&mut *transaction)
                .await?;
            return Err(if exists.is_some() {
                RevisionError::StaleHead
            } else {
                RevisionError::MissingChapter
            });
        }
        let head_rev = expected_head
            .checked_add(1)
            .ok_or(RevisionError::RevisionOverflow)?;
        let current = load(&mut transaction, chapter_id).await?;
        let delta = rebuild(&current.delta, std::slice::from_ref(revision))?;
        let revision_json = serde_json::to_string(revision)?;
        sqlx::query!(
            "INSERT INTO chapter_revisions (chapter_id, rev, delta) VALUES (?, ?, ?)",
            chapter_id,
            head_rev,
            revision_json
        )
        .execute(&mut *transaction)
        .await?;
        if head_rev
            .cast_unsigned()
            .is_multiple_of(self.snapshot_every.get())
        {
            let snapshot = serde_json::to_string(&delta)?;
            sqlx::query!(
                "UPDATE chapters SET head_rev = ?, snapshot_rev = ?, content = ? WHERE id = ?",
                head_rev,
                head_rev,
                snapshot,
                chapter_id
            )
            .execute(&mut *transaction)
            .await?;
        } else {
            sqlx::query!(
                "UPDATE chapters SET head_rev = ? WHERE id = ?",
                head_rev,
                chapter_id
            )
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(ChapterHead { head_rev, delta })
    }
}

pub(crate) async fn load(
    connection: &mut SqliteConnection,
    chapter_id: i64,
) -> Result<ChapterHead, RevisionError> {
    let chapter = sqlx::query!(
        "SELECT content, snapshot_rev, head_rev FROM chapters WHERE id = ?",
        chapter_id
    )
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(RevisionError::MissingChapter)?;
    let rows = sqlx::query!(
        "SELECT rev, delta FROM chapter_revisions WHERE chapter_id = ? AND rev > ? ORDER BY rev",
        chapter_id,
        chapter.snapshot_rev
    )
    .fetch_all(&mut *connection)
    .await?;
    let mut last_rev = chapter.snapshot_rev;
    let mut revisions = Vec::with_capacity(rows.len());
    for row in rows {
        if last_rev.checked_add(1) != Some(row.rev) {
            return Err(RevisionError::IncompleteHistory);
        }
        last_rev = row.rev;
        revisions.push(Delta::from_json(&row.delta)?);
    }
    if last_rev != chapter.head_rev {
        return Err(RevisionError::IncompleteHistory);
    }
    Ok(ChapterHead {
        head_rev: chapter.head_rev,
        delta: rebuild(&Delta::from_json(&chapter.content)?, &revisions)?,
    })
}

pub(crate) async fn missed(
    connection: &mut SqliteConnection,
    chapter_id: i64,
    since: i64,
    head: i64,
) -> Result<Vec<Revision>, RevisionError> {
    if since < 0 || since > head {
        return Err(RevisionError::StaleHead);
    }
    let rows = sqlx::query!(
        "SELECT rev, delta FROM chapter_revisions WHERE chapter_id = ? AND rev > ? ORDER BY rev",
        chapter_id,
        since
    )
    .fetch_all(connection)
    .await?;
    let mut last_rev = since;
    let mut revisions = Vec::with_capacity(rows.len());
    for row in rows {
        if last_rev.checked_add(1) != Some(row.rev) {
            return Err(RevisionError::IncompleteHistory);
        }
        last_rev = row.rev;
        revisions.push(Revision {
            rev: row.rev,
            delta: Delta::from_json(&row.delta)?,
        });
    }
    if last_rev != head {
        return Err(RevisionError::IncompleteHistory);
    }
    Ok(revisions)
}
