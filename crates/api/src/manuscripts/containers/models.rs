use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::manuscripts::{
    reject_keys,
    series::models::{validate_position, validate_title},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub id: i64,
    pub universe_id: i64,
    pub title: String,
    pub position: i64,
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateContainer {
    pub title: String,
    pub position: i64,
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContainer {
    pub title: Option<String>,
    pub position: Option<i64>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl CreateContainer {
    pub fn validate(&self) -> Result<(), Error> {
        validate_title(&self.title)?;
        validate_position(self.position)?;
        if !matches!(
            self.kind.as_str(),
            "book_series" | "tv_series" | "collection"
        ) {
            return Err(Error::Validation(
                "kind must be book_series, tv_series, or collection".into(),
            ));
        }
        Ok(())
    }
}

impl UpdateContainer {
    pub fn validate(&self) -> Result<(), Error> {
        reject_keys(&self.extra, &["kind", "universeId"])?;
        if let Some(title) = &self.title {
            validate_title(title)?;
        }
        if let Some(position) = self.position {
            validate_position(position)?;
        }
        Ok(())
    }
}
