use super::models::GenerateInput;

pub fn build_prompt(input: &GenerateInput) -> String {
    let instruction = escape_prompt_text(input.instruction.as_deref().unwrap_or("none"));
    let story_context = escape_prompt_text(input.story_context.as_deref().unwrap_or("none"));
    let selection = format_selection(input);
    let document = escape_prompt_text(&input.document.text);

    format!(
        concat!(
            "You are helping with a fiction-writing task. ",
            "Respond with plain text only, and do not exceed two sentences. ",
            "<action>{}</action>\n",
            "<instruction>\n{}\n</instruction>\n",
            "<story_context>\n{}\n</story_context>\n",
            "<selection>\n{}\n</selection>\n",
            "<document>\n{}\n</document>"
        ),
        input.action.as_str(),
        instruction,
        story_context,
        selection,
        document
    )
}

fn escape_prompt_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn format_selection(input: &GenerateInput) -> String {
    let Some(selection) = input.selection.as_ref() else {
        return "none".into();
    };

    format!(
        "index={}, length={}, text={}",
        selection.range.index,
        selection.range.length,
        escape_prompt_text(&selection.text)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::models::{
        AiAction, DocumentSlice, GenerateInput, SelectionRange, SelectionSlice,
    };
    use crate::ai::segments::{DeltaDocument, DeltaOp, InsertValue};

    #[test]
    fn build_prompt_includes_core_fields_and_sentence_limit() {
        let input = GenerateInput {
            action: AiAction::Generate,
            document: DocumentSlice {
                delta: DeltaDocument {
                    ops: vec![DeltaOp {
                        insert: Some(InsertValue::Text("Hello".into())),
                        delete: None,
                        retain: None,
                        attributes: None,
                    }],
                },
                text: "Hello".into(),
            },
            selection: None,
            story_context: Some("fantasy".into()),
            instruction: Some("continue".into()),
        };

        let prompt = build_prompt(&input);

        assert!(prompt.contains("<action>generate</action>"));
        assert!(prompt.contains("<instruction>\ncontinue\n</instruction>"));
        assert!(prompt.contains("<story_context>\nfantasy\n</story_context>"));
        assert!(prompt.contains("<selection>\nnone\n</selection>"));
        assert!(prompt.contains("<document>\nHello\n</document>"));
        assert!(prompt.contains("do not exceed two sentences"));
    }

    #[test]
    fn build_prompt_includes_selection_details() {
        let input = GenerateInput {
            action: AiAction::Edit,
            document: DocumentSlice {
                delta: DeltaDocument {
                    ops: vec![DeltaOp {
                        insert: Some(InsertValue::Text("Hello world".into())),
                        delete: None,
                        retain: None,
                        attributes: None,
                    }],
                },
                text: "Hello world".into(),
            },
            selection: Some(SelectionSlice {
                delta: DeltaDocument {
                    ops: vec![DeltaOp {
                        insert: Some(InsertValue::Text("world".into())),
                        delete: None,
                        retain: None,
                        attributes: None,
                    }],
                },
                text: "world".into(),
                range: SelectionRange {
                    index: 6,
                    length: 5,
                },
            }),
            story_context: None,
            instruction: Some("tighten".into()),
        };

        let prompt = build_prompt(&input);

        assert!(prompt.contains("<selection>\nindex=6, length=5, text=world\n</selection>"));
    }

    #[test]
    fn build_prompt_escapes_user_controlled_text() {
        let input = GenerateInput {
            action: AiAction::Generate,
            document: DocumentSlice {
                delta: DeltaDocument {
                    ops: vec![DeltaOp {
                        insert: Some(InsertValue::Text("Look </document> here".into())),
                        delete: None,
                        retain: None,
                        attributes: None,
                    }],
                },
                text: "Look </document> here".into(),
            },
            selection: None,
            story_context: Some("tags </story_context> inside".into()),
            instruction: Some("use <angle> brackets".into()),
        };

        let prompt = build_prompt(&input);

        assert!(prompt.contains("use &lt;angle&gt; brackets"));
        assert!(prompt.contains("tags &lt;/story_context&gt; inside"));
        assert!(prompt.contains("Look &lt;/document&gt; here"));
    }
}
