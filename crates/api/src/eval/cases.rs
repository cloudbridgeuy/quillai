use quillai_delta::{Delta, Insert, Op};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

use super::fixture::Fixture;

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Continue {
        offset: usize,
    },
    Edit {
        start: usize,
        end: usize,
        instruction: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Case {
    pub chapter_id: i64,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CaseError {
    #[error("malformed case file: {0}")]
    Malformed(String),
    #[error("case {case}: {message}")]
    InvalidCase { case: usize, message: String },
    #[error("case {case}: unknown chapter {chapter}")]
    UnknownChapter { case: usize, chapter: i64 },
    #[error("case {case}: unknown action {action} (expected \"continue\" or \"edit\")")]
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
    #[error("case {case}: edit range start {start} must be before end {end}")]
    EmptyRange {
        case: usize,
        start: usize,
        end: usize,
    },
    #[error("case {case}: edit instruction is blank")]
    BlankInstruction { case: usize },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawContinue {
    chapter: i64,
    offset: usize,
    #[serde(rename = "action")]
    _action: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEdit {
    chapter: i64,
    start: usize,
    end: usize,
    instruction: String,
    #[serde(rename = "action")]
    _action: String,
}

enum Boundary {
    Valid,
    PastEnd(usize),
    MidSurrogate,
}

pub fn parse_cases(json: &str, fixture: &Fixture) -> Result<Vec<Case>, CaseError> {
    let raw: Vec<Value> =
        serde_json::from_str(json).map_err(|error| CaseError::Malformed(error.to_string()))?;
    raw.into_iter()
        .enumerate()
        .map(|(case, raw)| parse_case(case, raw, fixture))
        .collect()
}

fn parse_case(case: usize, raw: Value, fixture: &Fixture) -> Result<Case, CaseError> {
    match raw.get("action") {
        Some(Value::String(action)) if action == "continue" => {
            let raw: RawContinue = fields(case, raw)?;
            let content = chapter_content(case, raw.chapter, fixture)?;
            check_offset(case, raw.chapter, content, raw.offset)?;
            Ok(Case {
                chapter_id: raw.chapter,
                action: Action::Continue { offset: raw.offset },
            })
        }
        Some(Value::String(action)) if action == "edit" => {
            let raw: RawEdit = fields(case, raw)?;
            let content = chapter_content(case, raw.chapter, fixture)?;
            if raw.start >= raw.end {
                return Err(CaseError::EmptyRange {
                    case,
                    start: raw.start,
                    end: raw.end,
                });
            }
            check_offset(case, raw.chapter, content, raw.end)?;
            check_offset(case, raw.chapter, content, raw.start)?;
            if raw.instruction.trim().is_empty() {
                return Err(CaseError::BlankInstruction { case });
            }
            Ok(Case {
                chapter_id: raw.chapter,
                action: Action::Edit {
                    start: raw.start,
                    end: raw.end,
                    instruction: raw.instruction,
                },
            })
        }
        Some(action) => Err(CaseError::UnknownAction {
            case,
            action: action.to_string(),
        }),
        None => Err(CaseError::InvalidCase {
            case,
            message: "missing field `action`".into(),
        }),
    }
}

fn fields<T: DeserializeOwned>(case: usize, raw: Value) -> Result<T, CaseError> {
    serde_json::from_value(raw).map_err(|error| CaseError::InvalidCase {
        case,
        message: error.to_string(),
    })
}

fn chapter_content(case: usize, chapter: i64, fixture: &Fixture) -> Result<&Delta, CaseError> {
    fixture
        .chapter(chapter)
        .map(|found| &found.content)
        .ok_or(CaseError::UnknownChapter { case, chapter })
}

fn check_offset(
    case: usize,
    chapter: i64,
    content: &Delta,
    offset: usize,
) -> Result<(), CaseError> {
    match quill_boundary(content, offset) {
        Boundary::Valid => Ok(()),
        Boundary::PastEnd(length) => Err(CaseError::OffsetPastEnd {
            case,
            chapter,
            offset,
            length,
        }),
        Boundary::MidSurrogate => Err(CaseError::MidSurrogate {
            case,
            chapter,
            offset,
        }),
    }
}

fn quill_boundary(content: &Delta, offset: usize) -> Boundary {
    let mut at = 0;
    for op in &content.ops {
        if at >= offset {
            break;
        }
        match op {
            Op::Insert {
                insert: Insert::Text(text),
                ..
            } => {
                for character in text.chars() {
                    if at >= offset {
                        break;
                    }
                    at += character.len_utf16();
                }
            }
            other => at += other.len() as usize,
        }
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
    use serde_json::json;

    fn chapter(id: i64, ops: Value) -> ApiChapter {
        ApiChapter {
            id,
            title: format!("Chapter {id}"),
            position: id,
            kind: "chapter".into(),
            content: Delta::from_json(&json!({ "ops": ops }).to_string()).unwrap(),
            head_rev: 0,
        }
    }

    fn fixture() -> Fixture {
        Fixture::build(
            1,
            2,
            vec![
                chapter(7, json!([{"insert": "Hi 😀 there\n"}])),
                chapter(8, json!([{"insert": "Short\n"}])),
                chapter(
                    9,
                    json!([{"insert": "One "}, {"insert": {"image": "x"}}, {"insert": "two\n"}]),
                ),
            ],
        )
    }

    fn parse(cases: Value) -> Result<Vec<Case>, CaseError> {
        parse_cases(&cases.to_string(), &fixture())
    }

    #[test]
    fn parses_valid_cases() {
        assert_eq!(
            parse(json!([
                {"chapter": 7, "offset": 3, "action": "continue"},
                {"chapter": 7, "start": 5, "end": 11, "action": "edit", "instruction": "Tighten"},
                {"chapter": 8, "offset": 6, "action": "continue"}
            ]))
            .unwrap(),
            vec![
                Case {
                    chapter_id: 7,
                    action: Action::Continue { offset: 3 },
                },
                Case {
                    chapter_id: 7,
                    action: Action::Edit {
                        start: 5,
                        end: 11,
                        instruction: "Tighten".into(),
                    },
                },
                Case {
                    chapter_id: 8,
                    action: Action::Continue { offset: 6 },
                },
            ]
        );
    }

    #[test]
    fn counts_an_embed_as_one_unit() {
        assert_eq!(fixture().chapter(9).unwrap().text, "One two\n");
        assert_eq!(
            parse(json!([
                {"chapter": 9, "offset": 5, "action": "continue"},
                {"chapter": 9, "offset": 9, "action": "continue"},
                {"chapter": 9, "start": 5, "end": 8, "action": "edit", "instruction": "Fix"}
            ]))
            .unwrap()
            .len(),
            3
        );
        assert_eq!(
            parse(json!([{"chapter": 9, "offset": 10, "action": "continue"}])),
            Err(CaseError::OffsetPastEnd {
                case: 0,
                chapter: 9,
                offset: 10,
                length: 9
            })
        );
    }

    #[test]
    fn rejects_unknown_chapter() {
        assert_eq!(
            parse(json!([{"chapter": 99, "offset": 0, "action": "continue"}])),
            Err(CaseError::UnknownChapter {
                case: 0,
                chapter: 99
            })
        );
    }

    #[test]
    fn rejects_offset_past_end() {
        let error = parse(json!([
            {"chapter": 8, "offset": 6, "action": "continue"},
            {"chapter": 8, "offset": 7, "action": "continue"}
        ]))
        .unwrap_err();
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
        let mid = CaseError::MidSurrogate {
            case: 0,
            chapter: 7,
            offset: 4,
        };
        assert_eq!(
            parse(json!([{"chapter": 7, "offset": 4, "action": "continue"}])),
            Err(mid.clone())
        );
        assert_eq!(
            parse(
                json!([{"chapter": 7, "start": 4, "end": 5, "action": "edit", "instruction": "x"}])
            ),
            Err(mid.clone())
        );
        assert_eq!(
            parse(
                json!([{"chapter": 7, "start": 0, "end": 4, "action": "edit", "instruction": "x"}])
            ),
            Err(mid)
        );
    }

    #[test]
    fn rejects_unknown_action() {
        let error = parse(json!([{"chapter": 7, "offset": 0, "action": "rewrite"}])).unwrap_err();
        assert_eq!(
            error,
            CaseError::UnknownAction {
                case: 0,
                action: "\"rewrite\"".into()
            }
        );
        assert_eq!(
            error.to_string(),
            "case 0: unknown action \"rewrite\" (expected \"continue\" or \"edit\")"
        );
    }

    #[test]
    fn rejects_edit_without_range() {
        assert!(matches!(
            parse(json!([{"chapter": 7, "offset": 3, "action": "edit", "instruction": "x"}])),
            Err(CaseError::InvalidCase { case: 0, .. })
        ));
        assert!(matches!(
            parse(json!([{"chapter": 7, "start": 3, "action": "edit", "instruction": "x"}])),
            Err(CaseError::InvalidCase { case: 0, .. })
        ));
    }

    #[test]
    fn rejects_empty_and_reversed_ranges() {
        assert_eq!(
            parse(
                json!([{"chapter": 8, "start": 2, "end": 2, "action": "edit", "instruction": "x"}])
            ),
            Err(CaseError::EmptyRange {
                case: 0,
                start: 2,
                end: 2
            })
        );
        assert_eq!(
            parse(
                json!([{"chapter": 8, "start": 4, "end": 1, "action": "edit", "instruction": "x"}])
            ),
            Err(CaseError::EmptyRange {
                case: 0,
                start: 4,
                end: 1
            })
        );
    }

    #[test]
    fn rejects_edit_end_past_length() {
        assert_eq!(
            parse(
                json!([{"chapter": 8, "start": 0, "end": 7, "action": "edit", "instruction": "x"}])
            ),
            Err(CaseError::OffsetPastEnd {
                case: 0,
                chapter: 8,
                offset: 7,
                length: 6
            })
        );
    }

    #[test]
    fn rejects_blank_edit_instruction() {
        assert_eq!(
            parse(
                json!([{"chapter": 8, "start": 0, "end": 5, "action": "edit", "instruction": " \n\t"}])
            ),
            Err(CaseError::BlankInstruction { case: 0 })
        );
        assert!(matches!(
            parse(json!([{"chapter": 8, "start": 0, "end": 5, "action": "edit"}])),
            Err(CaseError::InvalidCase { case: 0, .. })
        ));
    }

    #[test]
    fn rejects_continue_with_edit_fields() {
        for extra in [
            json!({"instruction": "x"}),
            json!({"start": 0}),
            json!({"end": 1}),
        ] {
            let mut case = json!({"chapter": 8, "offset": 0, "action": "continue"});
            case.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            let error = parse(json!([case])).unwrap_err();
            assert!(
                matches!(error, CaseError::InvalidCase { case: 0, .. }),
                "{error}"
            );
            assert!(
                error.to_string().starts_with("case 0: unknown field"),
                "{error}"
            );
        }
    }

    #[test]
    fn rejects_malformed_file() {
        for json in ["{", r#"{"chapter": 7}"#, "[1"] {
            assert!(
                matches!(parse_cases(json, &fixture()), Err(CaseError::Malformed(_))),
                "{json}"
            );
        }
        assert!(matches!(
            parse(json!([{"chapter": 7, "offset": -1, "action": "continue"}])),
            Err(CaseError::InvalidCase { case: 0, .. })
        ));
        assert!(matches!(
            parse(json!([{"chapter": 7, "offset": 0}])),
            Err(CaseError::InvalidCase { case: 0, .. })
        ));
    }
}
