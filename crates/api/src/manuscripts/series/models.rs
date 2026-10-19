use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Series {
    pub id: i64,
    pub title: String,
    pub position: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateSeries {
    pub title: String,
    pub position: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSeries {
    pub title: Option<String>,
    pub position: Option<i64>,
}

fn validate_title(title: &str) -> Result<(), Error> {
    if title.trim().is_empty() {
        return Err(Error::Validation("title must not be empty".into()));
    }
    Ok(())
}

fn validate_position(position: i64) -> Result<(), Error> {
    if position < 0 {
        return Err(Error::Validation("position must not be negative".into()));
    }
    Ok(())
}

impl CreateSeries {
    pub fn validate(&self) -> Result<(), Error> {
        validate_title(&self.title)?;
        validate_position(self.position)
    }
}

impl UpdateSeries {
    pub fn validate(&self) -> Result<(), Error> {
        if let Some(title) = &self.title {
            validate_title(title)?;
        }
        if let Some(position) = self.position {
            validate_position(position)?;
        }
        Ok(())
    }
}
