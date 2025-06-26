use crate::attributes::AttributeMap;
use serde_json::Value as JsonValue;

/// Represents an embed object with type information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbedData {
    pub embed_type: String,
    pub data: JsonValue,
}

impl EmbedData {
    pub fn new(embed_type: String, data: JsonValue) -> Self {
        Self { embed_type, data }
    }
}

/// Represents a single operation in a Delta
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Insert text with optional attributes
    Insert {
        text: String,
        attributes: Option<AttributeMap>,
    },
    /// Insert an embed object with optional attributes
    InsertEmbed {
        embed: EmbedData,
        attributes: Option<AttributeMap>,
    },
    /// Delete a number of characters
    Delete(usize),
    /// Retain a number of characters with optional attribute changes
    Retain {
        length: usize,
        attributes: Option<AttributeMap>,
    },
    /// Retain an embed object with optional attribute changes
    RetainEmbed {
        embed: EmbedData,
        attributes: Option<AttributeMap>,
    },
}

impl Op {
    /// Calculate the length of an operation for document positioning
    pub fn length(&self) -> usize {
        match self {
            Op::Insert { text, .. } => text.chars().count(),
            Op::InsertEmbed { .. } => 1,
            Op::Delete(len) => *len,
            Op::Retain { length, .. } => *length,
            Op::RetainEmbed { .. } => 1,
        }
    }

    /// Get the attributes of an operation if present
    pub fn attributes(&self) -> Option<&AttributeMap> {
        match self {
            Op::Insert { attributes, .. }
            | Op::InsertEmbed { attributes, .. }
            | Op::Retain { attributes, .. }
            | Op::RetainEmbed { attributes, .. } => attributes.as_ref(),
            Op::Delete(_) => None,
        }
    }

    /// Get mutable attributes of an operation if present
    pub fn attributes_mut(&mut self) -> Option<&mut AttributeMap> {
        match self {
            Op::Insert { attributes, .. }
            | Op::InsertEmbed { attributes, .. }
            | Op::Retain { attributes, .. }
            | Op::RetainEmbed { attributes, .. } => attributes.as_mut(),
            Op::Delete(_) => None,
        }
    }

    /// Set attributes for an operation
    pub fn with_attributes(mut self, attrs: AttributeMap) -> Self {
        match &mut self {
            Op::Insert { attributes, .. }
            | Op::InsertEmbed { attributes, .. }
            | Op::Retain { attributes, .. }
            | Op::RetainEmbed { attributes, .. } => {
                *attributes = Some(attrs);
            }
            Op::Delete(_) => {} // Delete operations don't have attributes
        }
        self
    }

    /// Check if this is an insert operation
    pub fn is_insert(&self) -> bool {
        matches!(self, Op::Insert { .. } | Op::InsertEmbed { .. })
    }

    /// Check if this is a delete operation
    pub fn is_delete(&self) -> bool {
        matches!(self, Op::Delete(_))
    }

    /// Check if this is a retain operation
    pub fn is_retain(&self) -> bool {
        matches!(self, Op::Retain { .. } | Op::RetainEmbed { .. })
    }

    /// Get the operation type as a string for debugging/logging
    pub fn op_type(&self) -> &'static str {
        match self {
            Op::Insert { .. } => "insert",
            Op::InsertEmbed { .. } => "insert_embed",
            Op::Delete(_) => "delete",
            Op::Retain { .. } => "retain",
            Op::RetainEmbed { .. } => "retain_embed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::AttributeValue;
    use std::collections::BTreeMap;

    #[test]
    fn test_op_length() {
        let insert_text = Op::Insert {
            text: "Hello".to_string(),
            attributes: None,
        };
        assert_eq!(insert_text.length(), 5);

        let insert_embed = Op::InsertEmbed {
            embed: EmbedData::new("image".to_string(), JsonValue::String("url".to_string())),
            attributes: None,
        };
        assert_eq!(insert_embed.length(), 1);

        let delete = Op::Delete(10);
        assert_eq!(delete.length(), 10);

        let retain = Op::Retain {
            length: 7,
            attributes: None,
        };
        assert_eq!(retain.length(), 7);

        let retain_embed = Op::RetainEmbed {
            embed: EmbedData::new("video".to_string(), JsonValue::String("url".to_string())),
            attributes: None,
        };
        assert_eq!(retain_embed.length(), 1);
    }

    #[test]
    fn test_op_attributes() {
        let mut attrs = BTreeMap::new();
        attrs.insert("bold".to_string(), AttributeValue::Boolean(true));

        let op = Op::Insert {
            text: "text".to_string(),
            attributes: Some(attrs.clone()),
        };

        assert_eq!(op.attributes(), Some(&attrs));
        assert!(op.is_insert());
        assert!(!op.is_delete());
        assert!(!op.is_retain());
    }

    #[test]
    fn test_op_type_checks() {
        let insert = Op::Insert {
            text: "test".to_string(),
            attributes: None,
        };
        assert!(insert.is_insert());
        assert_eq!(insert.op_type(), "insert");

        let delete = Op::Delete(5);
        assert!(delete.is_delete());
        assert_eq!(delete.op_type(), "delete");

        let retain = Op::Retain {
            length: 3,
            attributes: None,
        };
        assert!(retain.is_retain());
        assert_eq!(retain.op_type(), "retain");
    }
}