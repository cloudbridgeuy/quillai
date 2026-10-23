pub use super::lore::{LoreEntry, LoreLevel};
pub use super::section_overrides::{PromptSectionId, SectionOverrides};
use serde::{Deserialize, Serialize};

use super::segments::{plain_text, sanitize_document, DeltaDocument, InsertValue};
use crate::error::Error;

pub const DEFAULT_MAX_OUTPUT_TOKENS: usize = 512;
pub const MIN_MAX_OUTPUT_TOKENS: usize = 1;
pub const MAX_MAX_OUTPUT_TOKENS: usize = 4096;
pub const EDIT_OUTPUT_TOKENS_BASE: usize = 128;
pub const EDIT_OUTPUT_TOKENS_PER_SELECTED_CHAR: usize = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    #[serde(default, deserialize_with = "deserialize_model")]
    pub model: Option<serde_json::Value>,
    #[serde(default = "empty_lore")]
    pub lore: serde_json::Value,
    #[serde(default = "empty_section_overrides")]
    pub section_overrides: serde_json::Value,
    pub action: String,
    pub document: RawDocumentSlice,
    pub selection: Option<RawSelectionSlice>,
    pub story_context: Option<String>,
    pub instruction: Option<String>,
    #[serde(default)]
    pub messages: Vec<RawConversationMessage>,
    pub resume_prompt: Option<String>,
    pub max_output_tokens: Option<usize>,
}

fn deserialize_model<'de, D>(deserializer: D) -> Result<Option<serde_json::Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde_json::Value::deserialize(deserializer).map(Some)
}

fn empty_section_overrides() -> serde_json::Value {
    serde_json::json!({})
}

fn empty_lore() -> serde_json::Value {
    serde_json::json!([])
}

#[derive(Debug, Deserialize)]
pub struct RawConversationMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawDocumentSlice {
    pub delta: DeltaDocument,
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSelectionSlice {
    pub delta: DeltaDocument,
    pub text: String,
    pub range: RawSelection,
}

#[derive(Debug, Deserialize)]
pub struct RawSelection {
    pub index: usize,
    pub length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiAction {
    Generate,
    Edit,
}

impl AiAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generate => "generate",
            Self::Edit => "edit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationRole {
    User,
    Assistant,
}

impl ConversationRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationMessage {
    pub role: ConversationRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionRange {
    pub index: usize,
    pub length: usize,
}

impl SelectionRange {
    pub fn end(self) -> Result<usize, Error> {
        self.index
            .checked_add(self.length)
            .ok_or_else(|| Error::Validation("selection range exceeds supported bounds".into()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentSlice {
    pub delta: DeltaDocument,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionSlice {
    pub delta: DeltaDocument,
    pub text: String,
    pub range: SelectionRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenerateInput {
    pub lore: Vec<LoreEntry>,
    pub author_guidance: Option<String>,
    pub section_overrides: SectionOverrides,
    pub action: AiAction,
    pub document: DocumentSlice,
    pub selection: Option<SelectionSlice>,
    pub story_context: Option<String>,
    pub instruction: Option<String>,
    pub messages: Vec<ConversationMessage>,
    pub resume_prompt: Option<String>,
    pub max_output_tokens: usize,
}

impl GenerateInput {
    pub fn selected_text(&self) -> Option<&str> {
        self.selection
            .as_ref()
            .map(|selection| selection.text.as_str())
    }

    pub fn effective_prompt_text(&self) -> Option<&str> {
        match self.action {
            AiAction::Generate => self
                .resume_prompt
                .as_deref()
                .or(self.instruction.as_deref()),
            AiAction::Edit => self.instruction.as_deref(),
        }
    }

    pub fn provider_max_tokens(&self) -> usize {
        match (self.action, &self.selection) {
            (AiAction::Edit, Some(selection)) => self.max_output_tokens.min(
                EDIT_OUTPUT_TOKENS_BASE
                    + EDIT_OUTPUT_TOKENS_PER_SELECTED_CHAR * selection.text.chars().count(),
            ),
            _ => self.max_output_tokens,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    Delta {
        delta: DeltaDocument,
    },
    Usage {
        #[serde(rename = "promptTokens")]
        prompt_tokens: u64,
        #[serde(rename = "completionTokens")]
        completion_tokens: u64,
    },
    Done {
        #[serde(rename = "finishReason")]
        finish_reason: FinishReason,
    },
    Error {
        error: String,
    },
    Cancelled,
}

impl TryFrom<String> for AiAction {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "generate" => Ok(Self::Generate),
            "edit" => Ok(Self::Edit),
            _ => Err(Error::Validation(
                "action must be one of: generate, edit".into(),
            )),
        }
    }
}

impl TryFrom<RawSelection> for SelectionRange {
    type Error = Error;

    fn try_from(value: RawSelection) -> Result<Self, Self::Error> {
        Ok(Self {
            index: value.index,
            length: value.length,
        })
    }
}

impl TryFrom<RawConversationMessage> for ConversationMessage {
    type Error = Error;

    fn try_from(value: RawConversationMessage) -> Result<Self, Self::Error> {
        let role = match value.role.as_str() {
            "user" => ConversationRole::User,
            "assistant" => ConversationRole::Assistant,
            _ => {
                return Err(Error::Validation(
                    "messages role must be one of: user, assistant".into(),
                ));
            }
        };

        let content = normalize_message_content(value.content)?;

        Ok(Self { role, content })
    }
}

impl TryFrom<RawDocumentSlice> for DocumentSlice {
    type Error = Error;

    fn try_from(value: RawDocumentSlice) -> Result<Self, Self::Error> {
        let delta = sanitize_document(value.delta)?;
        validate_text_only_delta(&delta, "document")?;
        let expected_text = plain_text(&delta);

        if value.text != expected_text {
            return Err(Error::Validation(
                "document text must match the sanitized delta text".into(),
            ));
        }

        Ok(Self {
            delta,
            text: value.text,
        })
    }
}

impl TryFrom<RawSelectionSlice> for SelectionSlice {
    type Error = Error;

    fn try_from(value: RawSelectionSlice) -> Result<Self, Self::Error> {
        let delta = sanitize_document(value.delta)?;
        validate_text_only_delta(&delta, "selection")?;

        Ok(Self {
            delta,
            text: value.text,
            range: SelectionRange::try_from(value.range)?,
        })
    }
}

impl TryFrom<GenerateRequest> for GenerateInput {
    type Error = Error;

    fn try_from(request: GenerateRequest) -> Result<Self, Self::Error> {
        let GenerateRequest {
            model: _,
            lore,
            section_overrides,
            action,
            document,
            selection,
            story_context,
            instruction,
            messages,
            resume_prompt,
            max_output_tokens,
        } = request;

        let action = AiAction::try_from(action)?;
        let lore = super::lore::parse_lore(lore)?;
        let section_overrides = SectionOverrides::try_from(section_overrides)?;
        let document = DocumentSlice::try_from(document)?;
        let selection = selection.map(SelectionSlice::try_from).transpose()?;
        let messages = if action == AiAction::Generate {
            messages
                .into_iter()
                .map(ConversationMessage::try_from)
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        validate_selection(selection.as_ref(), &document)?;

        if action == AiAction::Edit
            && selection
                .as_ref()
                .is_none_or(|selection| selection.range.length == 0)
        {
            return Err(Error::Validation(
                "selection is required when action is edit".into(),
            ));
        }

        Ok(Self {
            action,
            lore,
            author_guidance: None,
            document,
            selection,
            section_overrides,
            story_context: normalize_optional_text(story_context),
            instruction: normalize_optional_text(instruction),
            messages,
            resume_prompt: if action == AiAction::Generate {
                normalize_optional_text(resume_prompt)
            } else {
                None
            },
            max_output_tokens: normalize_max_output_tokens(max_output_tokens)?,
        })
    }
}

pub fn event_to_ndjson_line(event: &StreamEvent) -> String {
    format!(
        "{}\n",
        serde_json::to_string(event).expect("AI stream event serialization should not fail")
    )
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_max_output_tokens(value: Option<usize>) -> Result<usize, Error> {
    let value = value.unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS);
    if value < MIN_MAX_OUTPUT_TOKENS {
        return Err(Error::Validation(format!(
            "maxOutputTokens must be at least {MIN_MAX_OUTPUT_TOKENS}"
        )));
    }

    if value > MAX_MAX_OUTPUT_TOKENS {
        return Err(Error::Validation(format!(
            "maxOutputTokens must be at most {MAX_MAX_OUTPUT_TOKENS}"
        )));
    }

    Ok(value)
}

fn normalize_message_content(content: String) -> Result<String, Error> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(Error::Validation(
            "messages content must not be blank".into(),
        ));
    }

    Ok(trimmed.to_string())
}

fn validate_selection(
    selection: Option<&SelectionSlice>,
    document: &DocumentSlice,
) -> Result<(), Error> {
    let Some(selection) = selection else {
        return Ok(());
    };

    let text_len = utf16_len(&document.text);
    let end = selection.range.end()?;

    if end > text_len {
        return Err(Error::Validation(format!(
            "selection must fall within the sanitized document text span (0..{text_len})"
        )));
    }

    let expected_selection = slice_utf16_span(&document.text, selection.range)?;
    if expected_selection != selection.text {
        return Err(Error::Validation(
            "selection text must match the transmitted document text span".into(),
        ));
    }

    let selection_delta_text = plain_text(&selection.delta);
    if selection_delta_text != selection.text {
        return Err(Error::Validation(
            "selection delta text must match the transmitted selection text".into(),
        ));
    }

    Ok(())
}

fn validate_text_only_delta(delta: &DeltaDocument, kind: &str) -> Result<(), Error> {
    if delta
        .ops
        .iter()
        .any(|op| matches!(op.insert, Some(InsertValue::Embed(_))))
    {
        return Err(Error::Validation(format!(
            "AI {kind} slices currently support text inserts only"
        )));
    }

    Ok(())
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn slice_utf16_span(text: &str, selection: SelectionRange) -> Result<String, Error> {
    let end = selection.end()?;
    let units = text.encode_utf16().collect::<Vec<_>>();
    let slice = units.get(selection.index..end).ok_or_else(|| {
        Error::Validation("selection must fall within the sanitized document text span".into())
    })?;

    String::from_utf16(slice).map_err(|err| {
        Error::Validation(format!(
            "selection must align to valid UTF-16 boundaries: {err}"
        ))
    })
}

#[cfg(test)]
#[path = "models/tests.rs"]
mod tests;
