use serde::Deserialize;

use crate::error::Error;

pub const MAX_LORE_BYTES: usize = 262_144;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoreEntry {
    pub name: String,
    pub kind: LoreKind,
    pub visible_text: String,
    pub level: LoreLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreKind {
    Character,
    Place,
    Term,
    Note,
}

impl LoreKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Place => "place",
            Self::Term => "term",
            Self::Note => "note",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreLevel {
    Include,
    KnownDontReveal,
    LeaveOut,
}

pub fn parse_lore(value: serde_json::Value) -> Result<Vec<LoreEntry>, Error> {
    let mut lore: Vec<LoreEntry> = serde_json::from_value(value)
        .map_err(|error| Error::Validation(format!("invalid lore: {error}")))?;
    lore.retain(|entry| entry.level != LoreLevel::LeaveOut);
    if super::prompt::lore_text(&lore).len() > MAX_LORE_BYTES {
        return Err(Error::Validation(format!(
            "lore exceeds {MAX_LORE_BYTES} UTF-8 bytes"
        )));
    }
    Ok(lore)
}
