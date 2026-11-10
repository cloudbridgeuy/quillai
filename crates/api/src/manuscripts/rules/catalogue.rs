use serde_json::{Map, Value};

use crate::error::Error;

pub enum ParamKind {
    Int { min: i64, max: i64 },
    Words,
}

pub struct ParamDef {
    pub key: &'static str,
    pub kind: ParamKind,
}

pub struct RuleDef {
    pub id: &'static str,
    pub languages: &'static [&'static str],
    pub params: &'static [ParamDef],
}

pub const CATALOGUE: &[RuleDef] = &[
    RuleDef {
        id: "dialogue-punctuation",
        languages: &["en"],
        params: &[],
    },
    RuleDef {
        id: "redundant-phrases",
        languages: &["en"],
        params: &[],
    },
];

const LANGUAGES: [&str; 2] = ["en", "es"];
const SCOPES: [&str; 3] = ["narration", "dialogue", "both"];

pub struct Update {
    pub language: Option<String>,
    pub rules: Option<Map<String, Value>>,
}

pub fn parse_update(body: Value, catalogue: &[RuleDef]) -> Result<Update, Error> {
    let Value::Object(mut body) = body else {
        return invalid("body must be an object");
    };
    let language = match body.remove("language") {
        None => None,
        Some(Value::String(language)) if LANGUAGES.contains(&language.as_str()) => Some(language),
        Some(_) => return invalid("language must be en or es"),
    };
    let rules = match body.remove("rules") {
        None => None,
        Some(Value::Object(rules)) => {
            for (id, entry) in &rules {
                check_entry(id, entry, catalogue)?;
            }
            Some(rules)
        }
        Some(_) => return invalid("rules must be an object"),
    };
    Ok(Update { language, rules })
}

fn check_entry(id: &str, entry: &Value, catalogue: &[RuleDef]) -> Result<(), Error> {
    let Value::Object(entry) = entry else {
        return invalid(&format!("rule {id} must be an object"));
    };
    if entry
        .get("enabled")
        .is_some_and(|enabled| !enabled.is_boolean())
    {
        return invalid(&format!("rule {id} enabled must be a boolean"));
    }
    if entry
        .get("scope")
        .is_some_and(|scope| !scope.as_str().is_some_and(|scope| SCOPES.contains(&scope)))
    {
        return invalid(&format!(
            "rule {id} scope must be narration, dialogue or both"
        ));
    }
    let params = match entry.get("params") {
        None => return Ok(()),
        Some(Value::Object(params)) => params,
        Some(_) => return invalid(&format!("rule {id} params must be an object")),
    };
    let Some(rule) = catalogue.iter().find(|rule| rule.id == id) else {
        return Ok(());
    };
    for def in rule.params {
        let Some(value) = params.get(def.key) else {
            continue;
        };
        let valid = match def.kind {
            ParamKind::Int { min, max } => value.as_i64().is_some_and(|n| (min..=max).contains(&n)),
            ParamKind::Words => value.as_array().is_some_and(|words| {
                words
                    .iter()
                    .all(|word| word.as_str().is_some_and(|word| !word.trim().is_empty()))
            }),
        };
        if !valid {
            return invalid(&format!("rule {id} param {} is invalid", def.key));
        }
    }
    Ok(())
}

fn invalid<T>(message: &str) -> Result<T, Error> {
    Err(Error::Validation(message.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TEST_CATALOGUE: &[RuleDef] = &[RuleDef {
        id: "long-sentence",
        languages: &["en"],
        params: &[
            ParamDef {
                key: "maxWords",
                kind: ParamKind::Int { min: 10, max: 80 },
            },
            ParamDef {
                key: "words",
                kind: ParamKind::Words,
            },
        ],
    }];

    #[test]
    fn catalogue_lists_the_shipped_rules() {
        let shipped: Vec<_> = CATALOGUE
            .iter()
            .map(|rule| (rule.id, rule.languages, rule.params.len()))
            .collect();
        let english: &[&str] = &["en"];
        assert_eq!(
            shipped,
            vec![
                ("dialogue-punctuation", english, 0),
                ("redundant-phrases", english, 0)
            ]
        );
    }

    fn accepts(params: Value) -> bool {
        parse_update(
            json!({"rules": {"long-sentence": {"params": params}}}),
            TEST_CATALOGUE,
        )
        .is_ok()
    }

    #[test]
    fn known_params_are_checked_against_the_catalogue() {
        assert!(accepts(
            json!({"maxWords": 10, "words": ["suddenly"], "other": "kept"})
        ));
        assert!(accepts(json!({"maxWords": 80})));
        assert!(!accepts(json!({"maxWords": 9})));
        assert!(!accepts(json!({"maxWords": 81})));
        assert!(!accepts(json!({"maxWords": 35.5})));
        assert!(!accepts(json!({"maxWords": "35"})));
        assert!(!accepts(json!({"words": ["ok", " "]})));
        assert!(!accepts(json!({"words": [1]})));
        assert!(!accepts(json!({"words": "suddenly"})));
    }
}
