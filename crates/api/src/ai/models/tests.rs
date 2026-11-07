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
fn image_action_parses_but_text_request_rejects_it() {
    assert_eq!(
        AiAction::try_from("image".to_string()).unwrap(),
        AiAction::Image
    );
    let error = GenerateInput::try_from(GenerateRequest {
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "image".into(),
        lore_complete: None,
        document: Some(text_document("Hello")),
        selection: None,
        story_context: None,
        instruction: Some("A lighthouse".into()),
        messages: Vec::new(),
        resume_prompt: None,
        max_output_tokens: None,
    })
    .unwrap_err();
    assert!(
        matches!(&error, Error::Validation(message) if message.contains("generate, edit")),
        "{error}"
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "generate".into(),
        lore_complete: None,
        document: Some(RawDocumentSlice {
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
        }),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "edit".into(),
        lore_complete: None,
        document: Some(text_document("Hello")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "generate".into(),
        lore_complete: None,
        document: Some(RawDocumentSlice {
            delta: DeltaDocument {
                ops: vec![DeltaOp {
                    insert: Some(InsertValue::Text("Hello".into())),
                    delete: Some(1),
                    retain: None,
                    attributes: None,
                }],
            },
            text: "Hello".into(),
        }),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "edit".into(),
        lore_complete: None,
        document: Some(text_document("Hello")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "edit".into(),
        lore_complete: None,
        document: Some(text_document("Hello world")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "edit".into(),
        lore_complete: None,
        document: Some(text_document("Hello world")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "edit".into(),
        lore_complete: None,
        document: Some(text_document("Hello world")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "generate".into(),
        lore_complete: None,
        document: Some(RawDocumentSlice {
            delta: DeltaDocument {
                ops: vec![DeltaOp {
                    insert: Some(InsertValue::Text("Hello".into())),
                    delete: None,
                    retain: None,
                    attributes: None,
                }],
            },
            text: "Goodbye".into(),
        }),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "generate".into(),
        lore_complete: None,
        document: Some(RawDocumentSlice {
            delta: DeltaDocument {
                ops: vec![DeltaOp {
                    insert: Some(InsertValue::Embed(json!({ "image": "x" }))),
                    delete: None,
                    retain: None,
                    attributes: None,
                }],
            },
            text: "".into(),
        }),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "edit".into(),
        lore_complete: None,
        document: Some(text_document("Hello")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "edit".into(),
        lore_complete: None,
        document: Some(text_document("Hi 💡 there")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "generate".into(),
        lore_complete: None,
        document: Some(text_document("Hello")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "generate".into(),
        lore_complete: None,
        document: Some(text_document("Hello")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "generate".into(),
        lore_complete: None,
        document: Some(text_document("Hello")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "generate".into(),
        lore_complete: None,
        document: Some(text_document("Hello")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "generate".into(),
        lore_complete: None,
        document: Some(text_document("Hello")),
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
        model: None,
        lore: empty_lore(),
        section_overrides: empty_section_overrides(),
        action: "edit".into(),
        lore_complete: None,
        document: Some(text_document("Hello world")),
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

#[test]
fn generate_request_parses_model_without_validation() {
    let mut request = json!({
        "action": "generate",
        "document": {"delta": {"ops": [{"insert": "Hello"}]}, "text": "Hello"},
    });
    let parsed: GenerateRequest = serde_json::from_value(request.clone()).unwrap();
    assert_eq!(parsed.model, None);

    for model in [
        json!("alternate-model"),
        json!(" alternate-model "),
        json!(""),
        json!(null),
        json!(42),
        json!(true),
        json!([]),
        json!({"id": "alternate-model"}),
    ] {
        request["model"] = model.clone();
        let parsed: GenerateRequest = serde_json::from_value(request.clone()).unwrap();
        assert_eq!(parsed.model, Some(model));
    }
}
