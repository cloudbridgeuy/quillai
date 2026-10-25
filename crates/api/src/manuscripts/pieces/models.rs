use quillai_delta::Delta;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::manuscripts::{
    reject_keys,
    series::models::{validate_position, validate_title},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Piece {
    pub id: i64,
    pub universe_id: i64,
    pub container_id: Option<i64>,
    pub parent_id: Option<i64>,
    pub title: String,
    pub position: i64,
    pub label: String,
    pub format: String,
    pub shape: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PieceDetail {
    #[serde(flatten)]
    pub piece: Piece,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Delta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_rev: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Piece>>,
}

#[derive(Debug, Deserialize)]
pub struct LoosePieces {
    pub container: Storage,
}

#[derive(Debug, Deserialize)]
pub enum Storage {
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Deserialize)]
pub struct CreatePiece {
    pub title: String,
    pub position: i64,
    pub label: String,
    pub shape: String,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChild {
    pub title: String,
    pub position: i64,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePiece {
    pub title: Option<String>,
    pub position: Option<i64>,
    pub label: Option<String>,
    pub format: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

pub const EMPTY_DELTA: &str = r#"{"ops":[]}"#;

fn validate_label(label: &str) -> Result<(), Error> {
    if !matches!(
        label,
        "piece"
            | "short_story"
            | "novella"
            | "novel"
            | "screenplay"
            | "tv_script"
            | "episode"
            | "prologue"
            | "chapter"
            | "epilogue"
            | "scene"
    ) {
        return Err(Error::Validation(format!("unknown label {label}")));
    }
    Ok(())
}

fn validate_format(format: &str) -> Result<(), Error> {
    if !matches!(format, "prose" | "script") {
        return Err(Error::Validation("format must be prose or script".into()));
    }
    Ok(())
}

impl CreatePiece {
    pub fn validate(&self) -> Result<(), Error> {
        validate_title(&self.title)?;
        validate_position(self.position)?;
        validate_label(&self.label)?;
        if !matches!(self.shape.as_str(), "leaf" | "group") {
            return Err(Error::Validation("shape must be leaf or group".into()));
        }
        self.format.as_deref().map_or(Ok(()), validate_format)
    }

    pub fn format(&self) -> &str {
        match (self.format.as_deref(), self.label.as_str()) {
            (Some(format), _) => format,
            (None, "screenplay" | "tv_script" | "episode") => "script",
            (None, _) => "prose",
        }
    }
}

impl CreateChild {
    pub fn validate(&self) -> Result<(), Error> {
        validate_title(&self.title)?;
        validate_position(self.position)?;
        self.label.as_deref().map_or(Ok(()), validate_label)
    }

    pub fn label<'a>(&'a self, format: &str) -> &'a str {
        match (self.label.as_deref(), format) {
            (Some(label), _) => label,
            (None, "script") => "scene",
            (None, _) => "chapter",
        }
    }
}

impl UpdatePiece {
    pub fn validate(&self) -> Result<(), Error> {
        reject_keys(
            &self.extra,
            &["universeId", "containerId", "parentId", "shape", "content"],
        )?;
        if let Some(title) = &self.title {
            validate_title(title)?;
        }
        if let Some(position) = self.position {
            validate_position(position)?;
        }
        if let Some(label) = &self.label {
            validate_label(label)?;
        }
        self.format.as_deref().map_or(Ok(()), validate_format)
    }
}
