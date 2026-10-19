use std::collections::BTreeMap;

use serde::Deserialize;

use super::models::ConversationRole;
use crate::error::Error;

pub const MAX_OVERRIDE_SECTION_BYTES: usize = 65_536;
pub const MAX_OVERRIDE_TOTAL_BYTES: usize = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptSectionId {
    System,
    History,
    Instruction,
    StoryContext,
    Selection,
    Document,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptHistoryMessage {
    pub role: ConversationRole,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SectionOverrides {
    pub text: BTreeMap<PromptSectionId, String>,
    pub history: Option<Vec<PromptHistoryMessage>>,
}

impl TryFrom<serde_json::Value> for SectionOverrides {
    type Error = Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let parse_error = |error| Error::Validation(format!("invalid sectionOverrides: {error}"));
        let entries: BTreeMap<PromptSectionId, serde_json::Value> =
            serde_json::from_value(value).map_err(parse_error)?;
        let mut overrides = Self::default();
        let mut total = 0;
        for (id, value) in entries {
            let bytes = if id == PromptSectionId::History {
                let messages: Vec<PromptHistoryMessage> =
                    serde_json::from_value(value).map_err(parse_error)?;
                let bytes = messages.iter().map(|message| message.text.len()).sum();
                overrides.history = Some(messages);
                bytes
            } else {
                let text: String = serde_json::from_value(value).map_err(parse_error)?;
                let bytes = text.len();
                overrides.text.insert(id, text);
                bytes
            };
            if bytes > MAX_OVERRIDE_SECTION_BYTES {
                return Err(Error::Validation(format!(
                    "sectionOverrides section exceeds {MAX_OVERRIDE_SECTION_BYTES} UTF-8 bytes"
                )));
            }
            total += bytes;
        }
        if total > MAX_OVERRIDE_TOTAL_BYTES {
            return Err(Error::Validation(format!(
                "sectionOverrides total exceeds {MAX_OVERRIDE_TOTAL_BYTES} UTF-8 bytes"
            )));
        }
        Ok(overrides)
    }
}
