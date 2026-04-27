use serde::{Deserialize, Serialize};

use super::segments::{plain_text, sanitize_document, DeltaDocument, InsertValue};
use crate::error::Error;

pub const DEFAULT_MAX_OUTPUT_TOKENS: usize = 512;
pub const MIN_MAX_OUTPUT_TOKENS: usize = 1;
pub const MAX_MAX_OUTPUT_TOKENS: usize = 4096;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            document,
            selection,
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
mod tests {
    use super::*;
    use crate::ai::segments::{DeltaDocument, DeltaOp, InsertValue};
    use serde_json::json;

    fn text_document(text: &str) -> RawDocumentSlice {
        RawDocumentSlice {
            delta: DeltaDocument {
                ops: vec![DeltaOp {
                    insert: Some(InsertValue::Text(text.into())),
                    delete: None,
                    retain: None,
                    attributes: None,
                }],
            },
            text: text.into(),
        }
    }

    fn text_selection(text: &str, index: usize, length: usize) -> RawSelectionSlice {
        RawSelectionSlice {
            delta: DeltaDocument {
                ops: vec![DeltaOp {
                    insert: Some(InsertValue::Text(text.into())),
                    delete: None,
                    retain: None,
                    attributes: None,
                }],
            },
            text: text.into(),
            range: RawSelection { index, length },
        }
    }

    #[test]
    fn action_parses_generate() {
        assert_eq!(
            AiAction::try_from("generate".to_string()).unwrap(),
            AiAction::Generate
        );
    }

    #[test]
    fn action_rejects_unknown_value() {
        let err = AiAction::try_from("rewrite".to_string()).unwrap_err();
        assert!(err.to_string().contains("action must be one of"));
    }

    #[test]
    fn generate_request_parses_and_sanitizes() {
        let input = GenerateInput::try_from(GenerateRequest {
            action: "generate".into(),
            document: RawDocumentSlice {
                delta: DeltaDocument {
                    ops: vec![
                        DeltaOp {
                            insert: Some(InsertValue::Text("Hello".into())),
                            delete: None,
                            retain: None,
                            attributes: None,
                        },
                        DeltaOp {
                            insert: None,
                            delete: Some(1),
                            retain: None,
                            attributes: None,
                        },
                    ],
                },
                text: "Hello".into(),
            },
            selection: Some(text_selection("", 5, 0)),
            story_context: Some("  scene notes  ".into()),
            instruction: Some("   ".into()),
            messages: vec![RawConversationMessage {
                role: "user".into(),
                content: "  Add the text of the letter.  ".into(),
            }],
            resume_prompt: Some("  Continue from the cutoff.  ".into()),
            max_output_tokens: None,
        })
        .unwrap();

        assert_eq!(input.action, AiAction::Generate);
        assert_eq!(input.document.delta.ops.len(), 1);
        assert_eq!(input.document.text, "Hello");
        assert_eq!(input.story_context.as_deref(), Some("scene notes"));
        assert_eq!(input.instruction, None);
        assert_eq!(
            input.resume_prompt.as_deref(),
            Some("Continue from the cutoff.")
        );
        assert_eq!(input.max_output_tokens, DEFAULT_MAX_OUTPUT_TOKENS);
        assert_eq!(
            input.messages,
            vec![ConversationMessage {
                role: ConversationRole::User,
                content: "Add the text of the letter.".into(),
            }]
        );
        assert_eq!(input.selection.unwrap().range.length, 0);
    }

    #[test]
    fn edit_requires_non_empty_selection() {
        let err = GenerateInput::try_from(GenerateRequest {
            action: "edit".into(),
            document: text_document("Hello"),
            selection: Some(text_selection("", 0, 0)),
            story_context: None,
            instruction: Some("tighten".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap_err();

        assert!(err.to_string().contains("selection is required"));
    }

    #[test]
    fn malformed_document_is_rejected_at_boundary() {
        let err = GenerateInput::try_from(GenerateRequest {
            action: "generate".into(),
            document: RawDocumentSlice {
                delta: DeltaDocument {
                    ops: vec![DeltaOp {
                        insert: Some(InsertValue::Text("Hello".into())),
                        delete: Some(1),
                        retain: None,
                        attributes: None,
                    }],
                },
                text: "Hello".into(),
            },
            selection: None,
            story_context: None,
            instruction: None,
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("must not mix insert, delete, and retain"));
    }

    #[test]
    fn selection_must_fit_sanitized_document_text_span() {
        let err = GenerateInput::try_from(GenerateRequest {
            action: "edit".into(),
            document: text_document("Hello"),
            selection: Some(text_selection("o!", 4, 2)),
            story_context: None,
            instruction: Some("tighten".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("selection must fall within the sanitized document text span"));
    }

    #[test]
    fn selection_text_must_match_document_span() {
        let err = GenerateInput::try_from(GenerateRequest {
            action: "edit".into(),
            document: text_document("Hello world"),
            selection: Some(text_selection("there", 6, 5)),
            story_context: None,
            instruction: Some("tighten".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("selection text must match the transmitted document text span"));
    }

    #[test]
    fn selected_text_uses_selection_slice_text() {
        let input = GenerateInput::try_from(GenerateRequest {
            action: "edit".into(),
            document: text_document("Hello world"),
            selection: Some(text_selection("world", 6, 5)),
            story_context: None,
            instruction: Some("tighten".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap();

        assert_eq!(input.selected_text(), Some("world"));
    }

    #[test]
    fn selection_delta_text_must_match_selection_text() {
        let err = GenerateInput::try_from(GenerateRequest {
            action: "edit".into(),
            document: text_document("Hello world"),
            selection: Some(RawSelectionSlice {
                delta: DeltaDocument {
                    ops: vec![DeltaOp {
                        insert: Some(InsertValue::Text("there".into())),
                        delete: None,
                        retain: None,
                        attributes: None,
                    }],
                },
                text: "world".into(),
                range: RawSelection {
                    index: 6,
                    length: 5,
                },
            }),
            story_context: None,
            instruction: Some("tighten".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("selection delta text must match the transmitted selection text"));
    }

    #[test]
    fn document_text_must_match_sanitized_delta_text() {
        let err = GenerateInput::try_from(GenerateRequest {
            action: "generate".into(),
            document: RawDocumentSlice {
                delta: DeltaDocument {
                    ops: vec![DeltaOp {
                        insert: Some(InsertValue::Text("Hello".into())),
                        delete: None,
                        retain: None,
                        attributes: None,
                    }],
                },
                text: "Goodbye".into(),
            },
            selection: None,
            story_context: None,
            instruction: None,
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("document text must match the sanitized delta text"));
    }

    #[test]
    fn document_and_selection_slices_reject_embeds() {
        let document_err = GenerateInput::try_from(GenerateRequest {
            action: "generate".into(),
            document: RawDocumentSlice {
                delta: DeltaDocument {
                    ops: vec![DeltaOp {
                        insert: Some(InsertValue::Embed(json!({ "image": "x" }))),
                        delete: None,
                        retain: None,
                        attributes: None,
                    }],
                },
                text: "".into(),
            },
            selection: None,
            story_context: None,
            instruction: None,
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap_err();

        assert!(document_err
            .to_string()
            .contains("AI document slices currently support text inserts only"));

        let selection_err = GenerateInput::try_from(GenerateRequest {
            action: "edit".into(),
            document: text_document("Hello"),
            selection: Some(RawSelectionSlice {
                delta: DeltaDocument {
                    ops: vec![DeltaOp {
                        insert: Some(InsertValue::Embed(json!({ "image": "x" }))),
                        delete: None,
                        retain: None,
                        attributes: None,
                    }],
                },
                text: "".into(),
                range: RawSelection {
                    index: 0,
                    length: 1,
                },
            }),
            story_context: None,
            instruction: Some("tighten".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap_err();

        assert!(selection_err
            .to_string()
            .contains("AI selection slices currently support text inserts only"));
    }

    #[test]
    fn utf16_selection_indices_accept_emoji_spans() {
        let input = GenerateInput::try_from(GenerateRequest {
            action: "edit".into(),
            document: text_document("Hi 💡 there"),
            selection: Some(text_selection("💡", 3, 2)),
            story_context: None,
            instruction: Some("tighten".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap();

        assert_eq!(input.selected_text(), Some("💡"));
    }

    #[test]
    fn ndjson_line_serializes_event() {
        let line = event_to_ndjson_line(&StreamEvent::Delta {
            delta: DeltaDocument {
                ops: vec![DeltaOp {
                    insert: Some(InsertValue::Text("Hello".into())),
                    delete: None,
                    retain: None,
                    attributes: None,
                }],
            },
        });

        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(
            value,
            json!({ "type": "delta", "delta": { "ops": [{ "insert": "Hello" }] } })
        );
    }

    #[test]
    fn blank_message_content_is_rejected() {
        let err = GenerateInput::try_from(GenerateRequest {
            action: "generate".into(),
            document: text_document("Hello"),
            selection: None,
            story_context: None,
            instruction: Some("continue".into()),
            messages: vec![RawConversationMessage {
                role: "assistant".into(),
                content: "   ".into(),
            }],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("messages content must not be blank"));
    }

    #[test]
    fn invalid_message_role_is_rejected() {
        let err = GenerateInput::try_from(GenerateRequest {
            action: "generate".into(),
            document: text_document("Hello"),
            selection: None,
            story_context: None,
            instruction: Some("continue".into()),
            messages: vec![RawConversationMessage {
                role: "system".into(),
                content: "Nope".into(),
            }],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("messages role must be one of: user, assistant"));
    }

    #[test]
    fn token_limit_defaults_and_bounds_are_enforced() {
        let defaulted = GenerateInput::try_from(GenerateRequest {
            action: "generate".into(),
            document: text_document("Hello"),
            selection: None,
            story_context: None,
            instruction: Some("continue".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: None,
        })
        .unwrap();

        assert_eq!(defaulted.max_output_tokens, DEFAULT_MAX_OUTPUT_TOKENS);

        let too_large = GenerateInput::try_from(GenerateRequest {
            action: "generate".into(),
            document: text_document("Hello"),
            selection: None,
            story_context: None,
            instruction: Some("continue".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: Some(MAX_MAX_OUTPUT_TOKENS + 1),
        })
        .unwrap_err();

        assert!(too_large
            .to_string()
            .contains("maxOutputTokens must be at most"));

        let too_small = GenerateInput::try_from(GenerateRequest {
            action: "generate".into(),
            document: text_document("Hello"),
            selection: None,
            story_context: None,
            instruction: Some("continue".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: Some(MIN_MAX_OUTPUT_TOKENS - 1),
        })
        .unwrap_err();

        assert!(too_small
            .to_string()
            .contains("maxOutputTokens must be at least"));
    }

    #[test]
    fn edit_requests_drop_generate_only_thread_fields() {
        let input = GenerateInput::try_from(GenerateRequest {
            action: "edit".into(),
            document: text_document("Hello world"),
            selection: Some(text_selection("world", 6, 5)),
            story_context: None,
            instruction: Some("tighten".into()),
            messages: vec![RawConversationMessage {
                role: "assistant".into(),
                content: "previous output".into(),
            }],
            resume_prompt: Some("resume".into()),
            max_output_tokens: Some(600),
        })
        .unwrap();

        assert!(input.messages.is_empty());
        assert_eq!(input.resume_prompt, None);
        assert_eq!(input.max_output_tokens, 600);
        assert_eq!(input.effective_prompt_text(), Some("tighten"));
    }

    #[test]
    fn ndjson_line_serializes_done_finish_reason_in_camel_case() {
        let line = event_to_ndjson_line(&StreamEvent::Done {
            finish_reason: FinishReason::Length,
        });

        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(value, json!({ "type": "done", "finishReason": "length" }));
    }
}
