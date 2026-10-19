use quillai_delta::Delta;
use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Serialize)]
pub struct Chapter {
    pub id: i64,
    pub book_id: i64,
    pub title: String,
    pub position: i64,
    pub kind: String,
    pub content: Delta,
    pub head_rev: i64,
}

pub struct ChapterRow {
    pub id: i64,
    pub book_id: i64,
    pub title: String,
    pub position: i64,
    pub kind: String,
    pub content: String,
    pub head_rev: i64,
}

impl TryFrom<ChapterRow> for Chapter {
    type Error = Error;

    fn try_from(row: ChapterRow) -> Result<Self, Error> {
        let content =
            Delta::from_json(&row.content).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
        Ok(Self {
            id: row.id,
            book_id: row.book_id,
            title: row.title,
            position: row.position,
            kind: row.kind,
            content,
            head_rev: row.head_rev,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateChapter {
    pub title: String,
    pub position: i64,
    pub kind: String,
    #[serde(default)]
    pub content: Delta,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChapter {
    pub title: Option<String>,
    pub position: Option<i64>,
    pub kind: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn validate(title: Option<&str>, position: Option<i64>, kind: Option<&str>) -> Result<(), Error> {
    if title.is_some_and(|title| title.trim().is_empty()) {
        return Err(Error::Validation("title must not be empty".into()));
    }
    if position.is_some_and(|position| position < 0) {
        return Err(Error::Validation("position must not be negative".into()));
    }
    if kind.is_some_and(|kind| !matches!(kind, "chapter" | "prologue" | "epilogue")) {
        return Err(Error::Validation(
            "kind must be chapter, prologue, or epilogue".into(),
        ));
    }
    Ok(())
}

impl CreateChapter {
    pub fn validate(&self) -> Result<(), Error> {
        validate(Some(&self.title), Some(self.position), Some(&self.kind))
    }
}

impl UpdateChapter {
    pub fn validate(&self) -> Result<(), Error> {
        if self.extra.contains_key("content") {
            return Err(Error::Validation(
                "content cannot be updated through PUT".into(),
            ));
        }
        validate(self.title.as_deref(), self.position, self.kind.as_deref())
    }
}
