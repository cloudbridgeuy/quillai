use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeltaDocument {
    pub ops: Vec<DeltaOp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeltaOp {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert: Option<InsertValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retain: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InsertValue {
    Text(String),
    Embed(Value),
}

fn is_insert_op(op: &DeltaOp) -> bool {
    matches!(
        op.insert,
        Some(InsertValue::Text(_)) | Some(InsertValue::Embed(_))
    )
}

fn op_kind_count(op: &DeltaOp) -> usize {
    usize::from(op.insert.is_some())
        + usize::from(op.delete.is_some())
        + usize::from(op.retain.is_some())
}

fn sanitize_op(op: DeltaOp) -> Result<Option<DeltaOp>, Error> {
    match op_kind_count(&op) {
        0 => Err(Error::Validation(
            "document ops must include exactly one of insert, delete, or retain".into(),
        )),
        1 if is_insert_op(&op) => Ok(Some(op)),
        1 => Ok(None),
        _ => Err(Error::Validation(
            "document ops must not mix insert, delete, and retain".into(),
        )),
    }
}

pub fn sanitize_document(document: DeltaDocument) -> Result<DeltaDocument, Error> {
    let ops = document
        .ops
        .into_iter()
        .map(sanitize_op)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(DeltaDocument { ops })
}

pub fn plain_text(document: &DeltaDocument) -> String {
    document
        .ops
        .iter()
        .filter_map(|op| match &op.insert {
            Some(InsertValue::Text(text)) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

pub fn plain_text_len(document: &DeltaDocument) -> usize {
    document
        .ops
        .iter()
        .filter_map(|op| match &op.insert {
            Some(InsertValue::Text(text)) => Some(text.chars().count()),
            _ => None,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text_op(text: &str) -> DeltaOp {
        DeltaOp {
            insert: Some(InsertValue::Text(text.into())),
            delete: None,
            retain: None,
            attributes: None,
        }
    }

    #[test]
    fn sanitize_document_keeps_insert_ops_only() {
        let document = DeltaDocument {
            ops: vec![
                text_op("Hello"),
                DeltaOp {
                    insert: None,
                    delete: Some(2),
                    retain: None,
                    attributes: None,
                },
                DeltaOp {
                    insert: Some(InsertValue::Embed(json!({ "image": "x" }))),
                    delete: None,
                    retain: None,
                    attributes: None,
                },
            ],
        };

        let sanitized = sanitize_document(document).unwrap();

        assert_eq!(sanitized.ops.len(), 2);
        assert!(matches!(
            sanitized.ops[0].insert,
            Some(InsertValue::Text(_))
        ));
        assert!(matches!(
            sanitized.ops[1].insert,
            Some(InsertValue::Embed(_))
        ));
    }

    #[test]
    fn sanitize_document_rejects_mixed_ops() {
        let err = sanitize_document(DeltaDocument {
            ops: vec![DeltaOp {
                insert: Some(InsertValue::Text("Hello".into())),
                delete: Some(1),
                retain: None,
                attributes: None,
            }],
        })
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("document ops must not mix insert, delete, and retain"));
    }

    #[test]
    fn sanitize_document_rejects_op_without_kind() {
        let err = sanitize_document(DeltaDocument {
            ops: vec![DeltaOp {
                insert: None,
                delete: None,
                retain: None,
                attributes: None,
            }],
        })
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("document ops must include exactly one"));
    }

    #[test]
    fn plain_text_concatenates_text_inserts() {
        let document = DeltaDocument {
            ops: vec![
                text_op("Hello"),
                DeltaOp {
                    insert: Some(InsertValue::Embed(json!({ "divider": true }))),
                    delete: None,
                    retain: None,
                    attributes: None,
                },
                text_op(" world"),
            ],
        };

        assert_eq!(plain_text(&document), "Hello world");
    }

    #[test]
    fn plain_text_len_counts_text_span_only() {
        let document = DeltaDocument {
            ops: vec![
                text_op("Hi"),
                DeltaOp {
                    insert: Some(InsertValue::Embed(json!({ "divider": true }))),
                    delete: None,
                    retain: None,
                    attributes: None,
                },
                text_op(" there"),
            ],
        };

        assert_eq!(plain_text_len(&document), 8);
    }
}
