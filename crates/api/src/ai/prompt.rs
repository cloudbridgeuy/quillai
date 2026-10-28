use super::models::{AiAction, GenerateInput, LoreEntry, LoreLevel, PromptSectionId};

const GENERATE_RULES: &str = "You are helping with a fiction-writing task. Respond with plain text only, and do not exceed two sentences. Start exactly where the text before the caret ends. Do not repeat or rephrase it. ";
const EDIT_RULES: &str = concat!(
    "You are editing a passage of fiction. Rewrite only the selected text, following the instruction. ",
    "Change only what the instruction asks; keep every other word, sentence, and paragraph break unchanged. ",
    "Do not add new paragraphs unless the selected text already has them. ",
    "Do not add commentary, explanations, notes, or text before or after the selection. ",
    "Reply with only the replacement text for the selection, wrapped in <replacement></replacement>. "
);

pub fn build_prompt(input: &GenerateInput) -> String {
    let instruction = escape_prompt_text(section_text(
        input,
        PromptSectionId::Instruction,
        input.effective_prompt_text().unwrap_or("none"),
    ));
    let story_context = escape_prompt_text(section_text(
        input,
        PromptSectionId::StoryContext,
        input.story_context.as_deref().unwrap_or("none"),
    ));
    let selection = format_selection(input);
    let lore = lore_text(&input.lore);
    let lore = if lore.is_empty() {
        String::new()
    } else {
        format!("<lore>\n{}\n</lore>\n", escape_prompt_text(&lore))
    };
    let document = escape_prompt_text(section_text(
        input,
        PromptSectionId::Document,
        &input.document.text,
    ));

    let caret = if input.action == AiAction::Generate {
        let (before, after) = input.document_at_caret();
        format!(
            "\n<before_caret>\n{}\n</before_caret>\n<after_caret>\n{}\n</after_caret>",
            escape_prompt_text(before),
            escape_prompt_text(after)
        )
    } else {
        String::new()
    };

    format!(
        concat!(
            "{}",
            "<action>{}</action>\n",
            "<instruction>\n{}\n</instruction>\n",
            "<story_context>\n{}\n</story_context>\n",
            "{}",
            "<selection>\n{}\n</selection>\n",
            "<document>\n{}\n</document>{}"
        ),
        match input.action {
            AiAction::Generate => GENERATE_RULES,
            AiAction::Edit => EDIT_RULES,
        },
        input.action.as_str(),
        instruction,
        story_context,
        lore,
        selection,
        document,
        caret
    )
}

pub fn lore_text(entries: &[LoreEntry]) -> String {
    entries
        .iter()
        .filter(|entry| entry.level != LoreLevel::LeaveOut)
        .map(|entry| {
            format!(
                "Name: {}\nKind: {}\n{}Text: {}",
                entry.name,
                entry.kind.as_str(),
                if entry.level == LoreLevel::KnownDontReveal {
                    "Known, but MUST NOT be revealed in prose.\n"
                } else {
                    ""
                },
                entry.visible_text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn escape_prompt_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn section_text<'a>(input: &'a GenerateInput, id: PromptSectionId, fallback: &'a str) -> &'a str {
    input
        .section_overrides
        .text
        .get(&id)
        .map(String::as_str)
        .unwrap_or(fallback)
}

fn format_selection(input: &GenerateInput) -> String {
    let Some(selection) = input.selection.as_ref() else {
        return escape_prompt_text(section_text(input, PromptSectionId::Selection, "none"));
    };

    format!(
        "index={}, length={}, text={}",
        selection.range.index,
        selection.range.length,
        escape_prompt_text(section_text(
            input,
            PromptSectionId::Selection,
            &selection.text
        ))
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
            lore: vec![],
            author_guidance: None,
            section_overrides: Default::default(),
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
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: 512,
        };

        let prompt = build_prompt(&input);
        assert!(prompt
            .contains("<before_caret>\nHello\n</before_caret>\n<after_caret>\n\n</after_caret>"));

        assert!(prompt.contains("<action>generate</action>"));
        assert!(prompt.contains("<instruction>\ncontinue\n</instruction>"));
        assert!(prompt.contains("<story_context>\nfantasy\n</story_context>"));
        assert!(prompt.contains("<selection>\nnone\n</selection>"));
        assert!(prompt.contains("<document>\nHello\n</document>"));
        assert!(prompt.starts_with(&format!("{GENERATE_RULES}<action>generate</action>\n")));
        assert!(prompt.contains(
            "Start exactly where the text before the caret ends. Do not repeat or rephrase it."
        ));
        assert!(!prompt.contains("replacement"));
    }

    #[test]
    fn build_prompt_includes_selection_details() {
        let input = GenerateInput {
            lore: vec![],
            author_guidance: None,
            section_overrides: Default::default(),
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
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: 512,
        };

        let prompt = build_prompt(&input);

        assert!(prompt.contains("<selection>\nindex=6, length=5, text=world\n</selection>"));
        assert!(prompt.starts_with(&format!("{EDIT_RULES}<action>edit</action>\n")));
        assert!(prompt.contains("wrapped in <replacement></replacement>"));
        assert!(prompt.contains("Change only what the instruction asks"));
        assert!(prompt.contains("Do not add commentary"));
        assert!(!prompt.contains("two sentences"));
        assert!(!prompt.contains("<before_caret>"));
        assert!(!prompt.contains("<after_caret>"));

        let mut input = input;
        input.action = AiAction::Generate;
        input.document.text = "A😀B".into();
        input.selection.as_mut().unwrap().range = SelectionRange {
            index: 3,
            length: 0,
        };
        let prompt = build_prompt(&input);
        assert!(prompt
            .contains("<before_caret>\nA😀\n</before_caret>\n<after_caret>\nB\n</after_caret>"));
    }

    #[test]
    fn build_prompt_uses_resume_prompt_as_effective_generate_instruction() {
        let input = GenerateInput {
            lore: vec![],
            author_guidance: None,
            section_overrides: Default::default(),
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
            story_context: None,
            instruction: Some("continue".into()),
            messages: vec![],
            resume_prompt: Some("resume from cutoff".into()),
            max_output_tokens: 512,
        };

        let prompt = build_prompt(&input);

        assert!(prompt.contains("<instruction>\nresume from cutoff\n</instruction>"));
        assert!(!prompt.contains("<instruction>\ncontinue\n</instruction>"));
    }

    #[test]
    fn build_prompt_escapes_user_controlled_text() {
        let input = GenerateInput {
            lore: vec![],
            author_guidance: None,
            section_overrides: Default::default(),
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
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: 512,
        };

        let prompt = build_prompt(&input);

        assert!(prompt.contains("use &lt;angle&gt; brackets"));
        assert!(prompt.contains("tags &lt;/story_context&gt; inside"));
        assert!(prompt.contains("Look &lt;/document&gt; here"));
    }
}
