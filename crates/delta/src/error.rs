use thiserror::Error;

/// Errors produced while parsing a Delta.
#[derive(Debug, Error)]
pub enum DeltaError {
    /// An op's text splits a UTF-16 surrogate pair (for example half of an emoji).
    #[error("{0}")]
    SplitSurrogate(String),
    /// An operation quill-delta only supports through registered embed handlers
    /// (composing an embed retain with an embed).
    #[error("unsupported delta operation: {0}")]
    Unsupported(String),
    /// Any other JSON or shape error.
    #[error("invalid delta: {0}")]
    Json(serde_json::Error),
}

impl From<serde_json::Error> for DeltaError {
    fn from(e: serde_json::Error) -> Self {
        DeltaError::Json(e)
    }
}

/// Wording for a Delta whose text splits a UTF-16 surrogate pair.
pub(crate) const SPLIT_SURROGATE_MSG: &str = "invalid delta: text splits a UTF-16 surrogate pair \
    (lone surrogate); ops must not cut an emoji in half";
