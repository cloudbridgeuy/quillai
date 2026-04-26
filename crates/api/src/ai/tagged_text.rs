use super::segments::{DeltaDocument, DeltaOp, InsertValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextTag {
    Generated,
    Revised,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedText {
    pub tag: TextTag,
    pub text: String,
}

impl TaggedText {
    pub fn generated(text: &str) -> Self {
        Self {
            tag: TextTag::Generated,
            text: text.into(),
        }
    }

    pub fn revised(text: &str) -> Self {
        Self {
            tag: TextTag::Revised,
            text: text.into(),
        }
    }
}

pub fn into_delta(chunk: &TaggedText) -> DeltaDocument {
    DeltaDocument {
        ops: vec![DeltaOp {
            insert: Some(InsertValue::Text(chunk.text.clone())),
            delete: None,
            retain: None,
            attributes: None,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_delta_wraps_text_as_single_insert() {
        let tagged = TaggedText {
            tag: TextTag::Generated,
            text: "A new line".into(),
        };

        let delta = into_delta(&tagged);

        assert_eq!(delta.ops.len(), 1);
        assert_eq!(
            delta.ops[0].insert,
            Some(InsertValue::Text("A new line".into()))
        );
    }
}
