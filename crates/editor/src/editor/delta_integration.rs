//! Delta integration utilities for the QuillAI Editor.
//!
//! This module provides utilities for working with Delta documents in the editor,
//! including conversion between text and Delta operations, and rendering Delta content.

use quillai_delta::{Delta, Op};

/// Convert plain text to a Delta document.
///
/// This function creates a Delta with a single insert operation containing the provided text.
/// If the text is empty, it returns an empty Delta.
///
/// # Arguments
/// * `text` - The plain text to convert to Delta format
///
/// # Returns
/// A Delta document containing the text as an insert operation
///
/// # Example
/// ```rust,no_run
/// use quillai_editor::delta_integration::text_to_delta;
/// 
/// let delta = text_to_delta("Hello, world!");
/// assert_eq!(delta.ops().len(), 1);
/// ```
pub fn text_to_delta(text: &str) -> Delta {
    if text.is_empty() {
        Delta::new()
    } else {
        Delta::new().insert(text.to_string(), None)
    }
}

/// Convert a Delta document to plain text.
///
/// This function extracts all text content from Delta insert operations,
/// ignoring formatting attributes and embed objects.
///
/// # Arguments
/// * `delta` - The Delta document to convert
///
/// # Returns
/// A string containing all text content from the Delta
///
/// # Example
/// ```rust,no_run
/// use quillai_editor::delta_integration::{text_to_delta, delta_to_text};
/// 
/// let delta = text_to_delta("Hello, world!");
/// let text = delta_to_text(&delta);
/// assert_eq!(text, "Hello, world!");
/// ```
pub fn delta_to_text(delta: &Delta) -> String {
    if delta.ops().is_empty() {
        return String::new();
    }
    
    delta.ops().iter()
        .filter_map(|op| {
            match op {
                Op::Insert { text, .. } => Some(text.clone()),
                Op::InsertEmbed { .. } => Some("[embed]".to_string()), // Placeholder for embeds
                _ => None, // Retain and Delete ops don't contribute to document content
            }
        })
        .collect::<String>()
}

/// Convert a Delta document to a JSON string representation.
///
/// This function creates a simplified JSON representation of the Delta document
/// suitable for serialization and storage. Note that this is a basic implementation
/// that will be enhanced when full serde support is added to the Delta crate.
///
/// # Arguments
/// * `delta` - The Delta document to serialize
///
/// # Returns
/// A JSON string representing the Delta document
///
/// # Example
/// ```rust,no_run
/// use quillai_editor::delta_integration::{text_to_delta, delta_to_json};
/// 
/// let delta = text_to_delta("Hello");
/// let json = delta_to_json(&delta);
/// assert!(json.contains("Hello"));
/// ```
pub fn delta_to_json(delta: &Delta) -> String {
    if delta.ops().is_empty() {
        return r#"{"ops":[]}"#.to_string();
    }
    
    // Build JSON manually for now (until we add serde support)
    let ops_json: Vec<String> = delta.ops().iter().map(|op| {
        match op {
            Op::Insert { text, attributes } => {
                let escaped_text = text.replace('"', r#"\""#).replace('\n', r#"\n"#);
                if attributes.is_some() {
                    // TODO: Serialize attributes when needed
                    format!(r#"{{"insert":"{}","attributes":{{}}}}"#, escaped_text)
                } else {
                    format!(r#"{{"insert":"{}"}}"#, escaped_text)
                }
            }
            Op::InsertEmbed { embed, attributes } => {
                // Simplified embed representation
                if attributes.is_some() {
                    format!(r#"{{"insert":{{"{}":"embed"}},"attributes":{{}}}}"#, embed.embed_type)
                } else {
                    format!(r#"{{"insert":{{"{}":"embed"}}}}"#, embed.embed_type)
                }
            }
            Op::Delete(length) => {
                format!(r#"{{"delete":{}}}"#, length)
            }
            Op::Retain { length, attributes } => {
                if attributes.is_some() {
                    format!(r#"{{"retain":{},"attributes":{{}}}}"#, length)
                } else {
                    format!(r#"{{"retain":{}}}"#, length)
                }
            }
            Op::RetainEmbed { embed, attributes } => {
                if attributes.is_some() {
                    format!(r#"{{"retain":{{"{}":"embed"}},"attributes":{{}}}}"#, embed.embed_type)
                } else {
                    format!(r#"{{"retain":{{"{}":"embed"}}}}"#, embed.embed_type)
                }
            }
        }
    }).collect();
    
    format!(r#"{{"ops":[{}]}}"#, ops_json.join(","))
}

/// Create a Delta operation that represents inserting text at a specific position.
///
/// This function creates a Delta that can be composed with an existing document
/// to insert text at the specified position.
///
/// # Arguments
/// * `position` - The character position where text should be inserted
/// * `text` - The text to insert
/// * `document_length` - The current length of the document
///
/// # Returns
/// A Delta containing retain and insert operations to insert text at the position
///
/// # Example
/// ```rust,no_run
/// use quillai_editor::delta_integration::create_insert_operation;
/// 
/// let op = create_insert_operation(5, "world", 10);
/// // This creates a Delta that retains 5 characters, inserts "world", then retains the rest
/// ```
pub fn create_insert_operation(position: usize, text: &str, document_length: usize) -> Delta {
    let mut delta = Delta::new();
    
    // Retain characters before insertion point
    if position > 0 {
        delta = delta.retain(position, None);
    }
    
    // Insert the new text
    if !text.is_empty() {
        delta = delta.insert(text.to_string(), None);
    }
    
    // Retain characters after insertion point
    let remaining = document_length.saturating_sub(position);
    if remaining > 0 {
        delta = delta.retain(remaining, None);
    }
    
    delta
}

/// Create a Delta operation that represents deleting text at a specific position.
///
/// This function creates a Delta that can be composed with an existing document
/// to delete text at the specified position.
///
/// # Arguments
/// * `position` - The character position where deletion should start
/// * `length` - The number of characters to delete
/// * `document_length` - The current length of the document
///
/// # Returns
/// A Delta containing retain and delete operations to remove text at the position
///
/// # Example
/// ```rust,no_run
/// use quillai_editor::delta_integration::create_delete_operation;
/// 
/// let op = create_delete_operation(5, 3, 10);
/// // This creates a Delta that retains 5 characters, deletes 3, then retains the rest
/// ```
pub fn create_delete_operation(position: usize, length: usize, document_length: usize) -> Delta {
    let mut delta = Delta::new();
    
    // Retain characters before deletion point
    if position > 0 {
        delta = delta.retain(position, None);
    }
    
    // Delete the specified number of characters
    if length > 0 {
        delta = delta.delete(length);
    }
    
    // Retain characters after deletion point
    let remaining = document_length.saturating_sub(position + length);
    if remaining > 0 {
        delta = delta.retain(remaining, None);
    }
    
    delta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_to_delta() {
        let delta = text_to_delta("Hello, world!");
        assert_eq!(delta.ops().len(), 1);
        
        if let Op::Insert { text, .. } = &delta.ops()[0] {
            assert_eq!(text, "Hello, world!");
        } else {
            panic!("Expected insert operation");
        }
    }

    #[test]
    fn test_text_to_delta_empty() {
        let delta = text_to_delta("");
        assert!(delta.ops().is_empty());
    }

    #[test]
    fn test_delta_to_text() {
        let delta = Delta::new()
            .insert("Hello, ".to_string(), None)
            .insert("world!".to_string(), None);
        
        let text = delta_to_text(&delta);
        assert_eq!(text, "Hello, world!");
    }

    #[test]
    fn test_delta_to_text_empty() {
        let delta = Delta::new();
        let text = delta_to_text(&delta);
        assert_eq!(text, "");
    }

    #[test]
    fn test_delta_to_json() {
        let delta = text_to_delta("Hello");
        let json = delta_to_json(&delta);
        assert!(json.contains("Hello"));
        assert!(json.contains("ops"));
        assert!(json.contains("insert"));
    }

    #[test]
    fn test_create_insert_operation() {
        let op = create_insert_operation(5, "world", 10);
        assert_eq!(op.ops().len(), 3); // retain, insert, retain
        
        // Check the operations
        assert!(matches!(op.ops()[0], Op::Retain { length: 5, .. }));
        assert!(matches!(op.ops()[1], Op::Insert { .. }));
        assert!(matches!(op.ops()[2], Op::Retain { length: 5, .. }));
    }

    #[test]
    fn test_create_delete_operation() {
        let op = create_delete_operation(5, 3, 10);
        assert_eq!(op.ops().len(), 3); // retain, delete, retain
        
        // Check the operations
        assert!(matches!(op.ops()[0], Op::Retain { length: 5, .. }));
        assert!(matches!(op.ops()[1], Op::Delete(3)));
        assert!(matches!(op.ops()[2], Op::Retain { length: 2, .. }));
    }
}