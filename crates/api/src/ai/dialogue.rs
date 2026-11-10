use std::collections::HashSet;

use serde::Deserialize;

use super::lore::{LoreEntry, LoreKind};
use super::models::GenerateInput;
use super::prompt::{escape_prompt_text, lore_text};
use crate::error::Error;

pub const DIALOGUE_MAX_OUTPUT_TOKENS: usize = 1_024;
const MAX_SPEAKERS: usize = 6;
const MAX_BRIEF_BYTES: usize = 4_096;

pub const DIALOGUE_RULES: &str = concat!(
    "You are writing a dialogue exchange in a work of fiction. Respond with plain text only. ",
    "Write between 4 and 12 spoken lines for the listed speakers, each spoken line its own paragraph; short action beats between lines are allowed. ",
    "Start exactly where the text before the caret ends. Do not repeat or rephrase it. ",
    "Match the dialogue marks used in the text before the caret; if it has none, use straight double quotes. ",
    "Give each speaker a voice that fits their lore. A speaker may act on lore that must not be revealed, but must never state it. "
);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueInput {
    pub speakers: Vec<String>,
    #[serde(default)]
    pub brief: String,
}

pub fn parse(value: Option<serde_json::Value>, lore: &[LoreEntry]) -> Result<DialogueInput, Error> {
    let value = value
        .ok_or_else(|| Error::Validation("dialogue is required when action is dialogue".into()))?;
    let dialogue: DialogueInput = serde_json::from_value(value)
        .map_err(|error| Error::Validation(format!("invalid dialogue: {error}")))?;
    if dialogue.speakers.is_empty() || dialogue.speakers.len() > MAX_SPEAKERS {
        return Err(Error::Validation(format!(
            "dialogue speakers must name 1 to {MAX_SPEAKERS} characters"
        )));
    }
    let mut seen = HashSet::new();
    for speaker in &dialogue.speakers {
        if !seen.insert(speaker) {
            return Err(Error::Validation(format!(
                "dialogue speakers has a duplicate: {speaker}"
            )));
        }
        if !lore
            .iter()
            .any(|entry| entry.kind == LoreKind::Character && &entry.name == speaker)
        {
            return Err(Error::Validation(format!(
                "dialogue speaker {speaker} is not a character in lore"
            )));
        }
    }
    if dialogue.brief.len() > MAX_BRIEF_BYTES {
        return Err(Error::Validation(format!(
            "dialogue brief exceeds {MAX_BRIEF_BYTES} UTF-8 bytes"
        )));
    }
    Ok(dialogue)
}

pub fn build_prompt(input: &GenerateInput, dialogue: &DialogueInput) -> String {
    let speakers = dialogue
        .speakers
        .iter()
        .map(|speaker| format!("<speaker>{}</speaker>", escape_prompt_text(speaker)))
        .collect::<Vec<_>>()
        .join("\n");
    let brief = dialogue.brief.trim();
    let lore = lore_text(&input.lore);
    let lore = if lore.is_empty() {
        String::new()
    } else {
        format!("<lore>\n{}\n</lore>\n", escape_prompt_text(&lore))
    };
    format!(
        "{DIALOGUE_RULES}<action>dialogue</action>\n<speakers>\n{speakers}\n</speakers>\n<brief>\n{}\n</brief>\n<story_context>\n{}\n</story_context>\n{lore}<before_caret>\n{}\n</before_caret>",
        escape_prompt_text(if brief.is_empty() { "none" } else { brief }),
        escape_prompt_text(input.story_context.as_deref().unwrap_or("none")),
        escape_prompt_text(input.document_at_caret().0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::models::GenerateRequest;
    use serde_json::json;

    fn input(dialogue: serde_json::Value) -> Result<GenerateInput, Error> {
        let request: GenerateRequest = serde_json::from_value(json!({
            "action": "dialogue",
            "document": {"delta": {"ops": [{"insert": "Before. After\n"}]}, "text": "Before. After\n"},
            "selection": {"delta": {"ops": []}, "text": "", "range": {"index": 7, "length": 0}},
            "storyContext": "Rainy night",
            "maxOutputTokens": 99,
            "lore": [
                {"name": "Mara <V>", "kind": "character", "visible_text": "Gruff.", "level": "include"},
                {"name": "Ilan", "kind": "character", "visible_text": "Owes Mara a debt.", "level": "known_dont_reveal"},
                {"name": "Keep", "kind": "place", "visible_text": "Stone tower.", "level": "include"},
                {"name": "Ghost", "kind": "character", "visible_text": "ghost-text", "level": "leave_out"}
            ],
            "dialogue": dialogue,
        }))
        .unwrap();
        GenerateInput::try_from(request)
    }

    fn rejected(dialogue: serde_json::Value) -> String {
        match input(dialogue) {
            Err(Error::Validation(message)) => message,
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn accepts_character_speakers_and_fixes_max_output_tokens() {
        let parsed = input(json!({"speakers": ["Mara <V>", "Ilan"], "brief": " argue "})).unwrap();
        assert_eq!(parsed.max_output_tokens, DIALOGUE_MAX_OUTPUT_TOKENS);
        assert_eq!(
            parsed.dialogue,
            Some(DialogueInput {
                speakers: vec!["Mara <V>".into(), "Ilan".into()],
                brief: " argue ".into()
            })
        );
    }

    #[test]
    fn rejects_bad_speakers_and_long_brief() {
        assert!(rejected(json!({"speakers": []})).contains("speakers"));
        assert!(rejected(json!({"speakers": vec!["Mara <V>"; 7]})).contains("6"));
        assert!(rejected(json!({"speakers": ["Ilan", "Ilan"]})).contains("duplicate"));
        assert!(rejected(json!({"speakers": ["Keep"]})).contains("Keep"));
        assert!(rejected(json!({"speakers": ["Ghost"]})).contains("Ghost"));
        assert!(
            rejected(json!({"speakers": ["Ilan"], "brief": "a".repeat(4_097)})).contains("4096")
        );
        assert!(input(json!({"speakers": ["Ilan"], "brief": "é".repeat(2_048)})).is_ok());
    }

    #[test]
    fn prompt_has_rules_escaped_speakers_lore_and_only_text_before_caret() {
        let parsed =
            input(json!({"speakers": ["Mara <V>", "Ilan"], "brief": "the <debt>"})).unwrap();
        let prompt = build_prompt(&parsed, parsed.dialogue.as_ref().unwrap());
        assert!(prompt.starts_with(DIALOGUE_RULES));
        assert!(DIALOGUE_RULES.contains("between 4 and 12 spoken lines"));
        assert!(DIALOGUE_RULES.contains("never state"));
        assert!(DIALOGUE_RULES.contains("straight double quotes"));
        assert!(prompt.contains("<action>dialogue</action>"));
        assert!(prompt.contains("<speaker>Mara &lt;V&gt;</speaker>\n<speaker>Ilan</speaker>"));
        assert!(prompt.contains("<brief>\nthe &lt;debt&gt;\n</brief>"));
        assert!(prompt.contains("Rainy night"));
        assert!(prompt.contains("Known, but MUST NOT be revealed in prose."));
        assert!(prompt.contains("Stone tower."));
        assert!(prompt.contains("<before_caret>\nBefore.\n</before_caret>"));
        assert!(!prompt.contains("After"));
        assert!(!prompt.contains("ghost-text"));
    }
}
