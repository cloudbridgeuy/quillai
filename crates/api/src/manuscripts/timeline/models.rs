use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::manuscripts::series::models::{validate_position, validate_title};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePoint {
    pub id: i64,
    pub universe_id: i64,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreatePoint {
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePoint {
    pub name: Option<String>,
    pub position: Option<i64>,
}

impl CreatePoint {
    pub fn validate(&self) -> Result<(), Error> {
        validate_title(&self.name)
            .map_err(|_| Error::Validation("name must not be empty".into()))?;
        validate_position(self.position)
    }
}

impl UpdatePoint {
    pub fn validate(&self) -> Result<(), Error> {
        if let Some(name) = &self.name {
            validate_title(name).map_err(|_| Error::Validation("name must not be empty".into()))?;
        }
        self.position.map_or(Ok(()), validate_position)
    }
}
