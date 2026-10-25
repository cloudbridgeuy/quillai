use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::error::Error;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LoreEntry {
    pub id: i64,
    pub series_id: i64,
    pub kind: String,
    pub name: String,
    pub keywords: Json<Vec<String>>,
    pub aliases: Json<Vec<String>>,
    pub visible_text: String,
    pub hidden_notes: String,
    pub pinned: bool,
    pub level: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UniverseLoreEntry {
    pub id: i64,
    pub universe_id: i64,
    pub kind: String,
    pub name: String,
    pub keywords: Json<Vec<String>>,
    pub aliases: Json<Vec<String>>,
    pub visible_text: String,
    pub hidden_notes: String,
    pub pinned: bool,
    pub level: String,
}

impl LoreEntry {
    pub fn in_series(entry: UniverseLoreEntry, series_id: i64) -> Self {
        Self {
            id: entry.id,
            series_id,
            kind: entry.kind,
            name: entry.name,
            keywords: entry.keywords,
            aliases: entry.aliases,
            visible_text: entry.visible_text,
            hidden_notes: entry.hidden_notes,
            pinned: entry.pinned,
            level: entry.level,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoreInput {
    pub kind: Option<String>,
    pub name: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub aliases: Option<Vec<String>>,
    pub visible_text: Option<String>,
    pub hidden_notes: Option<String>,
    pub pinned: Option<bool>,
    pub level: Option<String>,
}

impl LoreInput {
    pub fn validate(&self, creating: bool) -> Result<(), Error> {
        if creating && (self.kind.is_none() || self.name.is_none()) {
            return Err(Error::Validation("kind and name are required".into()));
        }
        if let Some(kind) = &self.kind {
            if !["character", "place", "term", "note"].contains(&kind.as_str()) {
                return Err(Error::Validation("invalid lore kind".into()));
            }
        }
        if self
            .name
            .as_ref()
            .is_some_and(|name| name.trim().is_empty())
        {
            return Err(Error::Validation("name must not be blank".into()));
        }
        for (field, values) in [("keywords", &self.keywords), ("aliases", &self.aliases)] {
            if values
                .as_ref()
                .is_some_and(|values| values.iter().any(|value| value.trim().is_empty()))
            {
                return Err(Error::Validation(format!(
                    "{field} must contain nonblank strings"
                )));
            }
        }
        if let Some(level) = &self.level {
            if !["include", "known_dont_reveal", "leave_out"].contains(&level.as_str()) {
                return Err(Error::Validation("invalid lore level".into()));
            }
        }
        Ok(())
    }
}
