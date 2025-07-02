//! Operation types for the Delta format
//!
//! This module defines the fundamental operation types that make up a Delta:
//! - **Insert**: Add new content (text or embeds)
//! - **Delete**: Remove existing content
//! - **Retain**: Keep existing content, optionally modifying attributes
//!
//! Operations are the atomic units of change in the Delta format. They can be
//! combined, transformed, and inverted to support collaborative editing.

use crate::attributes::AttributeMap;
use serde_json::Value as JsonValue;

/// Represents embedded content in a Delta document
///
/// Embeds are non-text objects like images, videos, or custom widgets that
/// occupy a single character position in the document. Each embed has a type
/// identifier and associated data.
///
/// # Examples
///
/// ```rust
/// use quillai_delta::op::EmbedData;
/// use serde_json::json;
///
/// // Create an image embed
/// let image = EmbedData::new(
///     "image".to_string(),
///     json!({
///         "url": "https://example.com/image.png",
///         "alt": "Example image",
///         "width": 300,
///         "height": 200
///     })
/// );
///
/// // Create a video embed
/// let video = EmbedData::new(
///     "video".to_string(),
///     json!({
///         "url": "https://example.com/video.mp4",
///         "poster": "https://example.com/poster.jpg"
///     })
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbedData {
    /// The type of embed (e.g., "image", "video", "formula")
    pub embed_type: String,
    /// JSON data associated with the embed
    pub data: JsonValue,
}

impl EmbedData {
    /// Creates a new embed with the specified type and data
    ///
    /// # Arguments
    ///
    /// * `embed_type` - A string identifying the type of embed
    /// * `data` - JSON data containing embed-specific information
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::op::EmbedData;
    /// use serde_json::json;
    ///
    /// let formula = EmbedData::new(
    ///     "formula".to_string(),
    ///     json!({ "latex": "E = mc^2" })
    /// );
    /// ```
    pub fn new(embed_type: String, data: JsonValue) -> Self {
        Self { embed_type, data }
    }
}

/// Represents a single operation in a Delta
///
/// Operations are the building blocks of the Delta format. Each operation
/// describes a specific action to perform on a document:
///
/// - **Insert operations** add new content
/// - **Delete operations** remove existing content
/// - **Retain operations** preserve existing content, optionally changing attributes
///
/// # Examples
///
/// ```rust
/// use quillai_delta::{Op, AttributeMap, AttributeValue};
/// use std::collections::BTreeMap;
///
/// // Insert plain text
/// let insert = Op::Insert {
///     text: "Hello world".to_string(),
///     attributes: None,
/// };
///
/// // Insert formatted text
/// let mut attrs = BTreeMap::new();
/// attrs.insert("bold".to_string(), AttributeValue::Boolean(true));
/// let formatted_insert = Op::Insert {
///     text: "Bold text".to_string(),
///     attributes: Some(attrs),
/// };
///
/// // Delete operation
/// let delete = Op::Delete(5);
///
/// // Retain with attribute changes
/// let mut attrs = BTreeMap::new();
/// attrs.insert("italic".to_string(), AttributeValue::Boolean(true));
/// let retain = Op::Retain {
///     length: 10,
///     attributes: Some(attrs),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Inserts new text content with optional formatting attributes
    ///
    /// The text is inserted at the current position in the document.
    /// Attributes define formatting like bold, italic, color, etc.
    Insert {
        /// The text content to insert
        text: String,
        /// Optional formatting attributes
        attributes: Option<AttributeMap>,
    },
    
    /// Inserts an embedded object with optional attributes
    ///
    /// Embeds represent non-text content like images or videos.
    /// They occupy exactly one character position in the document.
    InsertEmbed {
        /// The embed data including type and content
        embed: EmbedData,
        /// Optional formatting attributes for the embed
        attributes: Option<AttributeMap>,
    },
    
    /// Deletes a specified number of characters
    ///
    /// The deletion starts at the current position and removes
    /// the specified number of characters forward.
    Delete(usize),
    
    /// Retains existing characters, optionally modifying their attributes
    ///
    /// Retain operations preserve content while potentially changing
    /// its formatting. A retain without attributes is a no-op that
    /// simply advances the position.
    Retain {
        /// The number of characters to retain
        length: usize,
        /// Optional attribute changes to apply
        attributes: Option<AttributeMap>,
    },
    
    /// Retains an existing embed, optionally modifying its attributes
    ///
    /// This operation must match the exact embed type and data
    /// at the current position.
    RetainEmbed {
        /// The embed to retain (must match existing embed)
        embed: EmbedData,
        /// Optional attribute changes to apply
        attributes: Option<AttributeMap>,
    },
}

impl Op {
    /// Calculates the length of content affected by this operation
    ///
    /// - Text operations count Unicode characters (not bytes)
    /// - Embeds always have length 1
    /// - Delete and retain use their explicit length values
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Op;
    ///
    /// let insert = Op::Insert {
    ///     text: "Hello".to_string(),
    ///     attributes: None,
    /// };
    /// assert_eq!(insert.length(), 5);
    ///
    /// let delete = Op::Delete(10);
    /// assert_eq!(delete.length(), 10);
    /// ```
    pub fn length(&self) -> usize {
        match self {
            Op::Insert { text, .. } => text.chars().count(),
            Op::InsertEmbed { .. } => 1,
            Op::Delete(len) => *len,
            Op::Retain { length, .. } => *length,
            Op::RetainEmbed { .. } => 1,
        }
    }

    /// Returns a reference to the operation's attributes if present
    ///
    /// Delete operations never have attributes. All other operations
    /// may optionally have attributes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Op, AttributeMap, AttributeValue};
    ///
    /// let mut attrs = AttributeMap::new();
    /// attrs.insert("bold".to_string(), AttributeValue::Boolean(true));
    ///
    /// let op = Op::Insert {
    ///     text: "Hello".to_string(),
    ///     attributes: Some(attrs.clone()),
    /// };
    ///
    /// assert_eq!(op.attributes(), Some(&attrs));
    /// ```
    pub fn attributes(&self) -> Option<&AttributeMap> {
        match self {
            Op::Insert { attributes, .. }
            | Op::InsertEmbed { attributes, .. }
            | Op::Retain { attributes, .. }
            | Op::RetainEmbed { attributes, .. } => attributes.as_ref(),
            Op::Delete(_) => None,
        }
    }

    /// Returns a mutable reference to the operation's attributes if present
    ///
    /// This allows modifying attributes in place. Delete operations
    /// never have attributes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Op, AttributeMap, AttributeValue};
    ///
    /// let mut op = Op::Insert {
    ///     text: "Hello".to_string(),
    ///     attributes: Some(AttributeMap::new()),
    /// };
    ///
    /// if let Some(attrs) = op.attributes_mut() {
    ///     attrs.insert("italic".to_string(), AttributeValue::Boolean(true));
    /// }
    /// ```
    pub fn attributes_mut(&mut self) -> Option<&mut AttributeMap> {
        match self {
            Op::Insert { attributes, .. }
            | Op::InsertEmbed { attributes, .. }
            | Op::Retain { attributes, .. }
            | Op::RetainEmbed { attributes, .. } => attributes.as_mut(),
            Op::Delete(_) => None,
        }
    }

    /// Creates a new operation with the specified attributes
    ///
    /// This consumes the operation and returns a new one with the given
    /// attributes. Delete operations ignore this call since they cannot
    /// have attributes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Op, AttributeMap, AttributeValue};
    ///
    /// let mut attrs = AttributeMap::new();
    /// attrs.insert("color".to_string(), AttributeValue::String("#ff0000".to_string()));
    ///
    /// let op = Op::Insert {
    ///     text: "Red text".to_string(),
    ///     attributes: None,
    /// }.with_attributes(attrs);
    /// ```
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

    /// Checks if this operation inserts new content
    ///
    /// Returns true for both text and embed insertions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Op;
    ///
    /// let insert = Op::Insert {
    ///     text: "Hello".to_string(),
    ///     attributes: None,
    /// };
    /// assert!(insert.is_insert());
    ///
    /// let delete = Op::Delete(5);
    /// assert!(!delete.is_insert());
    /// ```
    pub fn is_insert(&self) -> bool {
        matches!(self, Op::Insert { .. } | Op::InsertEmbed { .. })
    }

    /// Checks if this operation deletes content
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Op;
    ///
    /// let delete = Op::Delete(5);
    /// assert!(delete.is_delete());
    ///
    /// let retain = Op::Retain { length: 5, attributes: None };
    /// assert!(!retain.is_delete());
    /// ```
    pub fn is_delete(&self) -> bool {
        matches!(self, Op::Delete(_))
    }

    /// Checks if this operation retains existing content
    ///
    /// Returns true for both text and embed retains.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Op;
    ///
    /// let retain = Op::Retain { length: 10, attributes: None };
    /// assert!(retain.is_retain());
    ///
    /// let insert = Op::Insert {
    ///     text: "Hello".to_string(),
    ///     attributes: None,
    /// };
    /// assert!(!insert.is_retain());
    /// ```
    pub fn is_retain(&self) -> bool {
        matches!(self, Op::Retain { .. } | Op::RetainEmbed { .. })
    }

    /// Returns the operation type as a string
    ///
    /// This is primarily used for debugging and logging purposes.
    ///
    /// # Returns
    ///
    /// - "insert" for text insertions
    /// - "insert_embed" for embed insertions
    /// - "delete" for deletions
    /// - "retain" for text retains
    /// - "retain_embed" for embed retains
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Op;
    ///
    /// let insert = Op::Insert {
    ///     text: "Hello".to_string(),
    ///     attributes: None,
    /// };
    /// assert_eq!(insert.op_type(), "insert");
    ///
    /// let delete = Op::Delete(5);
    /// assert_eq!(delete.op_type(), "delete");
    /// ```
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