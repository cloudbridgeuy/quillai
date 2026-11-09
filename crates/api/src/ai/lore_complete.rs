use serde::Deserialize;

use super::lore::LoreKind;
use super::models::{
    AiAction, DocumentSlice, GenerateInput, SectionOverrides, DEFAULT_MAX_OUTPUT_TOKENS,
};
use super::prompt::escape_prompt_text;
use super::segments::DeltaDocument;
use crate::error::Error;

const MAX_LORE_COMPLETE_BYTES: usize = 65_536;

pub const LORE_COMPLETE_RULES: &str = "You are completing a reference note about a story element. Respond with plain text only, and do not exceed two sentences. Start exactly where the note ends. Do not repeat or rephrase it. ";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoreCompleteInput {
    pub name: String,
    pub kind: LoreKind,
    pub text: String,
}

pub fn parse(value: Option<serde_json::Value>) -> Result<GenerateInput, Error> {
    let value = value.ok_or_else(|| {
        Error::Validation("loreComplete is required when action is lore_complete".into())
    })?;
    let lore_complete: LoreCompleteInput = serde_json::from_value(value)
        .map_err(|error| Error::Validation(format!("invalid loreComplete: {error}")))?;
    if lore_complete.name.trim().is_empty() {
        return Err(Error::Validation("loreComplete name is required".into()));
    }
    if lore_complete.text.len() > MAX_LORE_COMPLETE_BYTES {
        return Err(Error::Validation(format!(
            "loreComplete text exceeds {MAX_LORE_COMPLETE_BYTES} UTF-8 bytes"
        )));
    }
    Ok(GenerateInput {
        lore: Vec::new(),
        author_guidance: None,
        section_overrides: SectionOverrides::default(),
        action: AiAction::LoreComplete,
        document: DocumentSlice {
            delta: DeltaDocument { ops: Vec::new() },
            text: String::new(),
        },
        selection: None,
        story_context: None,
        instruction: None,
        messages: Vec::new(),
        resume_prompt: None,
        max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
        lore_complete: Some(lore_complete),
    })
}

pub fn build_prompt(input: &LoreCompleteInput) -> String {
    format!(
        "{LORE_COMPLETE_RULES}<action>lore_complete</action>\n<name>{}</name>\n<kind>{}</kind>\n<note>\n{}\n</note>",
        escape_prompt_text(&input.name),
        input.kind.as_str(),
        escape_prompt_text(&input.text)
    )
}
