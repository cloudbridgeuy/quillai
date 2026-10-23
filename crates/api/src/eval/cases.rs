use serde::Deserialize;

use super::fixture::Fixture;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Continue,
    Edit,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Case {
    pub chapter_id: i64,
    pub offset: usize,
    pub action: Action,
    pub instruction: String,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CaseError {
    #[error("malformed case file: {0}")]
    Malformed(String),
    #[error("case {case}: unknown chapter {chapter}")]
    UnknownChapter { case: usize, chapter: i64 },
    #[error("case {case}: unknown action {action:?} (expected \"continue\" or \"edit\")")]
    UnknownAction { case: usize, action: String },
    #[error(
        "case {case}: offset {offset} is past the end of chapter {chapter} ({length} UTF-16 units)"
    )]
    OffsetPastEnd {
        case: usize,
        chapter: i64,
        offset: usize,
        length: usize,
    },
    #[error("case {case}: offset {offset} splits a surrogate pair in chapter {chapter}")]
    MidSurrogate {
        case: usize,
        chapter: i64,
        offset: usize,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCase {
    chapter: i64,
    offset: usize,
    action: String,
    #[serde(default)]
    instruction: String,
}

enum Boundary {
    Valid,
    PastEnd(usize),
    MidSurrogate,
}

pub fn parse_cases(json: &str, fixture: &Fixture) -> Result<Vec<Case>, CaseError> {
    let raw: Vec<RawCase> =
        serde_json::from_str(json).map_err(|error| CaseError::Malformed(error.to_string()))?;
    raw.into_iter()
        .enumerate()
        .map(|(case, raw)| parse_case(case, raw, fixture))
        .collect()
}

fn parse_case(case: usize, raw: RawCase, fixture: &Fixture) -> Result<Case, CaseError> {
    let action = match raw.action.as_str() {
        "continue" => Action::Continue,
        "edit" => Action::Edit,
        _ => {
            return Err(CaseError::UnknownAction {
                case,
                action: raw.action,
            })
        }
    };
    let chapter = fixture
        .chapter(raw.chapter)
        .ok_or(CaseError::UnknownChapter {
            case,
            chapter: raw.chapter,
        })?;
    match utf16_boundary(&chapter.text, raw.offset) {
        Boundary::Valid => Ok(Case {
            chapter_id: raw.chapter,
            offset: raw.offset,
            action,
            instruction: raw.instruction,
        }),
        Boundary::PastEnd(length) => Err(CaseError::OffsetPastEnd {
            case,
            chapter: raw.chapter,
            offset: raw.offset,
            length,
        }),
        Boundary::MidSurrogate => Err(CaseError::MidSurrogate {
            case,
            chapter: raw.chapter,
            offset: raw.offset,
        }),
    }
}

fn utf16_boundary(text: &str, offset: usize) -> Boundary {
    let mut at = 0;
    for character in text.chars() {
        if at >= offset {
            break;
        }
        at += character.len_utf16();
    }
    match at.cmp(&offset) {
        std::cmp::Ordering::Equal => Boundary::Valid,
        std::cmp::Ordering::Less => Boundary::PastEnd(at),
        std::cmp::Ordering::Greater => Boundary::MidSurrogate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::fixture::{ApiChapter, Fixture};
    use quillai_delta::Delta;

    fn fixture() -> Fixture {
        let chapter = |id, text: &str| ApiChapter {
            id,
            title: format!("Chapter {id}"),
            position: id,
            kind: "chapter".into(),
            content: Delta::from_json(&serde_json::json!({"ops": [{"insert": text}]}).to_string())
                .unwrap(),
            head_rev: 0,
        };
        Fixture::build(
            1,
            2,
            vec![chapter(7, "Hi 😀 there\n"), chapter(8, "Short\n")],
        )
    }

    #[test]
    fn parses_valid_cases() {
        let json = r#"[
            {"chapter": 7, "offset": 3, "action": "continue"},
            {"chapter": 7, "offset": 5, "action": "edit", "instruction": "Tighten"},
            {"chapter": 8, "offset": 6, "action": "continue", "instruction": ""}
        ]"#;
        assert_eq!(
            parse_cases(json, &fixture()).unwrap(),
            vec![
                Case {
                    chapter_id: 7,
                    offset: 3,
                    action: Action::Continue,
                    instruction: String::new(),
                },
                Case {
                    chapter_id: 7,
                    offset: 5,
                    action: Action::Edit,
                    instruction: "Tighten".into(),
                },
                Case {
                    chapter_id: 8,
                    offset: 6,
                    action: Action::Continue,
                    instruction: String::new(),
                },
            ]
        );
    }

    #[test]
    fn rejects_unknown_chapter() {
        let json = r#"[{"chapter": 9, "offset": 0, "action": "continue"}]"#;
        assert_eq!(
            parse_cases(json, &fixture()),
            Err(CaseError::UnknownChapter {
                case: 0,
                chapter: 9
            })
        );
    }

    #[test]
    fn rejects_offset_past_end() {
        let json = r#"[
            {"chapter": 8, "offset": 6, "action": "continue"},
            {"chapter": 8, "offset": 7, "action": "continue"}
        ]"#;
        let error = parse_cases(json, &fixture()).unwrap_err();
        assert_eq!(
            error,
            CaseError::OffsetPastEnd {
                case: 1,
                chapter: 8,
                offset: 7,
                length: 6
            }
        );
        assert_eq!(
            error.to_string(),
            "case 1: offset 7 is past the end of chapter 8 (6 UTF-16 units)"
        );
    }

    #[test]
    fn rejects_offset_inside_surrogate_pair() {
        let json = r#"[{"chapter": 7, "offset": 4, "action": "continue"}]"#;
        assert_eq!(
            parse_cases(json, &fixture()),
            Err(CaseError::MidSurrogate {
                case: 0,
                chapter: 7,
                offset: 4
            })
        );
    }

    #[test]
    fn rejects_unknown_action() {
        let json = r#"[{"chapter": 7, "offset": 0, "action": "rewrite"}]"#;
        assert_eq!(
            parse_cases(json, &fixture()),
            Err(CaseError::UnknownAction {
                case: 0,
                action: "rewrite".into()
            })
        );
    }

    #[test]
    fn rejects_malformed_file() {
        for json in [
            "{",
            r#"{"chapter": 7}"#,
            r#"[{"chapter": 7, "offset": -1, "action": "edit"}]"#,
            r#"[{"chapter": 7, "action": "edit"}]"#,
            r#"[{"chapter": 7, "offset": 0, "action": "edit", "extra": 1}]"#,
        ] {
            assert!(
                matches!(parse_cases(json, &fixture()), Err(CaseError::Malformed(_))),
                "{json}"
            );
        }
    }
}
