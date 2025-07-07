use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement, Node, Text as DomText};
use quillai_delta::{Delta, Op, AttributeMap};
use std::collections::HashMap;
use crate::editor::dom_integration::{DomChange, DomChangeType};

/// Comprehensive Delta operation conversion system
///
/// This module provides utilities for converting between different representations
/// of document changes: DOM events, DOM changes, Delta operations, and Parchment operations.
pub struct DeltaOperationConverter {
    /// Current document length for position calculations
    document_length: usize,
    /// Mapping of attribute names to their conversion functions
    attribute_converters: HashMap<String, Box<dyn Fn(&serde_json::Value) -> String>>,
}

impl DeltaOperationConverter {
    /// Create a new Delta operation converter
    pub fn new() -> Self {
        let mut converter = Self {
            document_length: 0,
            attribute_converters: HashMap::new(),
        };
        
        converter.setup_default_attribute_converters();
        converter
    }

    /// Set up default attribute conversion functions
    fn setup_default_attribute_converters(&mut self) {
        // Bold attribute converter
        self.attribute_converters.insert(
            "bold".to_string(),
            Box::new(|value| {
                if value.as_bool().unwrap_or(false) {
                    "font-weight: bold".to_string()
                } else {
                    String::new()
                }
            })
        );

        // Italic attribute converter
        self.attribute_converters.insert(
            "italic".to_string(),
            Box::new(|value| {
                if value.as_bool().unwrap_or(false) {
                    "font-style: italic".to_string()
                } else {
                    String::new()
                }
            })
        );

        // Color attribute converter
        self.attribute_converters.insert(
            "color".to_string(),
            Box::new(|value| {
                if let Some(color) = value.as_str() {
                    format!("color: {}", color)
                } else {
                    String::new()
                }
            })
        );

        // Background color attribute converter
        self.attribute_converters.insert(
            "background".to_string(),
            Box::new(|value| {
                if let Some(bg) = value.as_str() {
                    format!("background-color: {}", bg)
                } else {
                    String::new()
                }
            })
        );

        // Font family attribute converter
        self.attribute_converters.insert(
            "font".to_string(),
            Box::new(|value| {
                if let Some(font) = value.as_str() {
                    format!("font-family: {}", font)
                } else {
                    String::new()
                }
            })
        );

        // Font size attribute converter
        self.attribute_converters.insert(
            "size".to_string(),
            Box::new(|value| {
                if let Some(size) = value.as_str() {
                    format!("font-size: {}", size)
                } else {
                    String::new()
                }
            })
        );
    }

    /// Update the current document length
    pub fn set_document_length(&mut self, length: usize) {
        self.document_length = length;
    }

    /// Convert DOM changes to a Delta operation
    ///
    /// Takes a list of DOM changes and converts them into a single Delta that represents
    /// all the changes. This is the core method for translating user interactions into
    /// Delta operations.
    ///
    /// # Arguments
    ///
    /// * `changes` - Vector of DOM changes to convert
    ///
    /// # Returns
    ///
    /// Delta representing all the changes
    pub fn dom_changes_to_delta(&self, changes: Vec<DomChange>) -> Delta {
        let mut delta = Delta::new();
        let mut current_position = 0;

        for change in changes {
            match change.change_type {
                DomChangeType::Insert => {
                    // Add retain operation if we need to skip content
                    if change.position > current_position {
                        let retain_length = change.position - current_position;
                        delta = delta.retain(retain_length as u32);
                        current_position = change.position;
                    }

                    // Add insert operation
                    if let Some(content) = change.content {
                        let attributes = change.attributes.map(|attrs| self.convert_attributes_to_delta(&attrs));
                        delta = delta.insert(&content, attributes);
                        current_position += content.chars().count();
                    }
                }
                DomChangeType::Delete => {
                    // Add retain operation if we need to skip content
                    if change.position > current_position {
                        let retain_length = change.position - current_position;
                        delta = delta.retain(retain_length as u32);
                        current_position = change.position;
                    }

                    // Add delete operation
                    delta = delta.delete(change.length);
                    // Note: current_position doesn't advance for deletes
                }
                DomChangeType::Retain => {
                    // Add retain operation if we need to skip content
                    if change.position > current_position {
                        let retain_length = change.position - current_position;
                        delta = delta.retain(retain_length as u32);
                        current_position = change.position;
                    }

                    // Add retain operation with or without attributes
                    if let Some(attributes) = change.attributes {
                        let delta_attributes = self.convert_attributes_to_delta(&attributes);
                        delta = delta.retain_with_attributes(change.length, delta_attributes);
                    } else {
                        delta = delta.retain(change.length);
                    }
                    current_position += change.length as usize;
                }
                DomChangeType::FormatChange => {
                    // Add retain operation if we need to skip content
                    if change.position > current_position {
                        let retain_length = change.position - current_position;
                        delta = delta.retain(retain_length as u32);
                        current_position = change.position;
                    }

                    // Add retain operation with formatting attributes
                    if let Some(attributes) = change.attributes {
                        let delta_attributes = self.convert_attributes_to_delta(&attributes);
                        delta = delta.retain_with_attributes(change.length, delta_attributes);
                    }
                    current_position += change.length as usize;
                }
            }
        }

        delta
    }

    /// Convert a Delta operation to DOM changes
    ///
    /// Takes a Delta and converts it into a list of DOM changes that can be applied
    /// to update the DOM structure. This is useful for applying programmatic changes
    /// to the document.
    ///
    /// # Arguments
    ///
    /// * `delta` - The Delta to convert
    ///
    /// # Returns
    ///
    /// Vector of DOM changes representing the Delta
    pub fn delta_to_dom_changes(&self, delta: &Delta) -> Vec<DomChange> {
        let mut changes = Vec::new();
        let mut current_position = 0;

        for operation in delta.ops() {
            match operation {
                Op::Insert { insert, attributes } => {
                    let dom_attributes = attributes.as_ref()
                        .map(|attrs| self.convert_delta_attributes_to_dom(attrs));

                    changes.push(DomChange {
                        change_type: DomChangeType::Insert,
                        position: current_position as u32,
                        length: insert.chars().count() as u32,
                        content: Some(insert.clone()),
                        attributes: dom_attributes,
                    });

                    current_position += insert.chars().count();
                }
                Op::Retain { retain, attributes } => {
                    if let Some(attrs) = attributes {
                        let dom_attributes = self.convert_delta_attributes_to_dom(attrs);
                        changes.push(DomChange {
                            change_type: DomChangeType::FormatChange,
                            position: current_position as u32,
                            length: *retain,
                            content: None,
                            attributes: Some(dom_attributes),
                        });
                    }
                    current_position += *retain as usize;
                }
                Op::Delete { delete } => {
                    changes.push(DomChange {
                        change_type: DomChangeType::Delete,
                        position: current_position as u32,
                        length: *delete,
                        content: None,
                        attributes: None,
                    });
                    // Note: position doesn't advance for deletes
                }
            }
        }

        changes
    }

    /// Convert DOM attributes to Delta attributes
    fn convert_attributes_to_delta(&self, dom_attributes: &HashMap<String, String>) -> AttributeMap {
        let mut delta_attributes = AttributeMap::new();

        for (key, value) in dom_attributes {
            match key.as_str() {
                "style" => {
                    // Parse CSS style string and convert to individual attributes
                    self.parse_style_to_delta_attributes(value, &mut delta_attributes);
                }
                "class" => {
                    // Convert CSS classes to Delta attributes
                    self.parse_class_to_delta_attributes(value, &mut delta_attributes);
                }
                "href" => {
                    // Link attribute
                    delta_attributes.insert("link".to_string(), serde_json::Value::String(value.clone()));
                }
                _ => {
                    // Direct attribute mapping
                    delta_attributes.insert(key.clone(), serde_json::Value::String(value.clone()));
                }
            }
        }

        delta_attributes
    }

    /// Convert Delta attributes to DOM attributes
    fn convert_delta_attributes_to_dom(&self, delta_attributes: &AttributeMap) -> HashMap<String, String> {
        let mut dom_attributes = HashMap::new();
        let mut styles = Vec::new();
        let mut classes = Vec::new();

        for (key, value) in delta_attributes {
            match key.as_str() {
                "bold" => {
                    if value.as_bool().unwrap_or(false) {
                        styles.push("font-weight: bold".to_string());
                    }
                }
                "italic" => {
                    if value.as_bool().unwrap_or(false) {
                        styles.push("font-style: italic".to_string());
                    }
                }
                "underline" => {
                    if value.as_bool().unwrap_or(false) {
                        styles.push("text-decoration: underline".to_string());
                    }
                }
                "strike" => {
                    if value.as_bool().unwrap_or(false) {
                        styles.push("text-decoration: line-through".to_string());
                    }
                }
                "color" => {
                    if let Some(color) = value.as_str() {
                        styles.push(format!("color: {}", color));
                    }
                }
                "background" => {
                    if let Some(bg) = value.as_str() {
                        styles.push(format!("background-color: {}", bg));
                    }
                }
                "font" => {
                    if let Some(font) = value.as_str() {
                        styles.push(format!("font-family: {}", font));
                    }
                }
                "size" => {
                    if let Some(size) = value.as_str() {
                        styles.push(format!("font-size: {}", size));
                    }
                }
                "link" => {
                    if let Some(url) = value.as_str() {
                        dom_attributes.insert("href".to_string(), url.to_string());
                    }
                }
                "header" => {
                    if let Some(level) = value.as_f64() {
                        classes.push(format!("header-{}", level as i32));
                    }
                }
                "blockquote" => {
                    if value.as_bool().unwrap_or(false) {
                        classes.push("blockquote".to_string());
                    }
                }
                "code-block" => {
                    if value.as_bool().unwrap_or(false) {
                        classes.push("code-block".to_string());
                    }
                }
                "list" => {
                    if let Some(list_type) = value.as_str() {
                        classes.push(format!("list-{}", list_type));
                    }
                }
                _ => {
                    // Direct attribute mapping
                    if let Some(str_value) = value.as_str() {
                        dom_attributes.insert(key.clone(), str_value.to_string());
                    }
                }
            }
        }

        // Combine styles into a single style attribute
        if !styles.is_empty() {
            dom_attributes.insert("style".to_string(), styles.join("; "));
        }

        // Combine classes into a single class attribute
        if !classes.is_empty() {
            dom_attributes.insert("class".to_string(), classes.join(" "));
        }

        dom_attributes
    }

    /// Parse CSS style string to Delta attributes
    fn parse_style_to_delta_attributes(&self, style: &str, delta_attributes: &mut AttributeMap) {
        for style_rule in style.split(';') {
            let rule = style_rule.trim();
            if rule.is_empty() {
                continue;
            }

            if let Some((property, value)) = rule.split_once(':') {
                let prop = property.trim();
                let val = value.trim();

                match prop {
                    "font-weight" => {
                        if val == "bold" || val == "700" || val == "800" || val == "900" {
                            delta_attributes.insert("bold".to_string(), serde_json::Value::Bool(true));
                        }
                    }
                    "font-style" => {
                        if val == "italic" {
                            delta_attributes.insert("italic".to_string(), serde_json::Value::Bool(true));
                        }
                    }
                    "text-decoration" => {
                        if val.contains("underline") {
                            delta_attributes.insert("underline".to_string(), serde_json::Value::Bool(true));
                        }
                        if val.contains("line-through") {
                            delta_attributes.insert("strike".to_string(), serde_json::Value::Bool(true));
                        }
                    }
                    "color" => {
                        delta_attributes.insert("color".to_string(), serde_json::Value::String(val.to_string()));
                    }
                    "background-color" => {
                        delta_attributes.insert("background".to_string(), serde_json::Value::String(val.to_string()));
                    }
                    "font-family" => {
                        delta_attributes.insert("font".to_string(), serde_json::Value::String(val.to_string()));
                    }
                    "font-size" => {
                        delta_attributes.insert("size".to_string(), serde_json::Value::String(val.to_string()));
                    }
                    _ => {
                        // Store unknown CSS properties as-is
                        delta_attributes.insert(format!("css-{}", prop), serde_json::Value::String(val.to_string()));
                    }
                }
            }
        }
    }

    /// Parse CSS class string to Delta attributes
    fn parse_class_to_delta_attributes(&self, class: &str, delta_attributes: &mut AttributeMap) {
        for class_name in class.split_whitespace() {
            match class_name {
                "bold" => {
                    delta_attributes.insert("bold".to_string(), serde_json::Value::Bool(true));
                }
                "italic" => {
                    delta_attributes.insert("italic".to_string(), serde_json::Value::Bool(true));
                }
                "underline" => {
                    delta_attributes.insert("underline".to_string(), serde_json::Value::Bool(true));
                }
                "strike" => {
                    delta_attributes.insert("strike".to_string(), serde_json::Value::Bool(true));
                }
                "blockquote" => {
                    delta_attributes.insert("blockquote".to_string(), serde_json::Value::Bool(true));
                }
                "code-block" => {
                    delta_attributes.insert("code-block".to_string(), serde_json::Value::Bool(true));
                }
                name if name.starts_with("header-") => {
                    if let Ok(level) = name[7..].parse::<i32>() {
                        delta_attributes.insert("header".to_string(), serde_json::Value::Number(serde_json::Number::from(level)));
                    }
                }
                name if name.starts_with("list-") => {
                    let list_type = &name[5..];
                    delta_attributes.insert("list".to_string(), serde_json::Value::String(list_type.to_string()));
                }
                _ => {
                    // Store unknown classes as-is
                    delta_attributes.insert(format!("class-{}", class_name), serde_json::Value::Bool(true));
                }
            }
        }
    }

    /// Create a Delta operation for text insertion
    ///
    /// # Arguments
    ///
    /// * `position` - Position to insert at
    /// * `text` - Text to insert
    /// * `attributes` - Optional formatting attributes
    ///
    /// # Returns
    ///
    /// Delta representing the insertion
    pub fn create_insert_delta(&self, position: usize, text: &str, attributes: Option<AttributeMap>) -> Delta {
        let mut delta = Delta::new();
        
        if position > 0 {
            delta = delta.retain(position, None);
        }
        
        delta = delta.insert(text, attributes);
        delta
    }

    /// Create a Delta operation for text deletion
    ///
    /// # Arguments
    ///
    /// * `position` - Position to delete from
    /// * `length` - Number of characters to delete
    ///
    /// # Returns
    ///
    /// Delta representing the deletion
    pub fn create_delete_delta(&self, position: usize, length: usize) -> Delta {
        let mut delta = Delta::new();
        
        if position > 0 {
            delta = delta.retain(position as u32);
        }
        
        delta = delta.delete(length as u32);
        delta
    }

    /// Create a Delta operation for formatting changes
    ///
    /// # Arguments
    ///
    /// * `position` - Position to start formatting
    /// * `length` - Length of text to format
    /// * `attributes` - Formatting attributes to apply
    ///
    /// # Returns
    ///
    /// Delta representing the formatting change
    pub fn create_format_delta(&self, position: usize, length: usize, attributes: AttributeMap) -> Delta {
        let mut delta = Delta::new();
        
        if position > 0 {
            delta = delta.retain(position as u32);
        }
        
        delta = delta.retain_with_attributes(length as u32, attributes);
        delta
    }

    /// Optimize a Delta by merging consecutive operations of the same type
    ///
    /// # Arguments
    ///
    /// * `delta` - The Delta to optimize
    ///
    /// # Returns
    ///
    /// Optimized Delta with merged operations
    pub fn optimize_delta(&self, delta: &Delta) -> Delta {
        // For now, return the delta as-is
        // In a full implementation, we would merge consecutive operations
        delta.clone()
    }

    /// Validate that a Delta is well-formed
    ///
    /// # Arguments
    ///
    /// * `delta` - The Delta to validate
    ///
    /// # Returns
    ///
    /// `true` if the Delta is valid, `false` otherwise
    pub fn validate_delta(&self, delta: &Delta) -> bool {
        // Basic validation: check that operations are in correct order
        let mut has_insert_or_delete = false;
        
        for operation in delta.ops() {
            match operation {
                Op::Retain { .. } => {
                    // Retain operations should come before insert/delete
                    if has_insert_or_delete {
                        return false;
                    }
                }
                Op::Insert { .. } | Op::Delete { .. } => {
                    has_insert_or_delete = true;
                }
            }
        }
        
        true
    }
}

impl Default for DeltaOperationConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::dom_integration::DomChange;

    #[test]
    fn test_converter_creation() {
        let converter = DeltaOperationConverter::new();
        assert_eq!(converter.document_length, 0);
        assert!(!converter.attribute_converters.is_empty());
    }

    #[test]
    fn test_dom_changes_to_delta_insert() {
        let converter = DeltaOperationConverter::new();
        
        let changes = vec![DomChange {
            change_type: DomChangeType::Insert,
            position: 0,
            length: 5,
            content: Some("Hello".to_string()),
            attributes: None,
        }];

        let delta = converter.dom_changes_to_delta(changes);
        assert_eq!(delta.ops().len(), 1);
        
        if let Op::Insert { insert, attributes: _ } = &delta.ops()[0] {
            assert_eq!(insert, "Hello");
        } else {
            panic!("Expected insert operation");
        }
    }

    #[test]
    fn test_dom_changes_to_delta_delete() {
        let converter = DeltaOperationConverter::new();
        
        let changes = vec![DomChange {
            change_type: DomChangeType::Delete,
            position: 5,
            length: 3,
            content: None,
            attributes: None,
        }];

        let delta = converter.dom_changes_to_delta(changes);
        assert_eq!(delta.ops().len(), 2); // retain + delete
        
        if let Op::Retain { retain, attributes: _ } = &delta.ops()[0] {
            assert_eq!(*retain, 5);
        } else {
            panic!("Expected retain operation");
        }
        
        if let Op::Delete { delete } = &delta.ops()[1] {
            assert_eq!(*delete, 3);
        } else {
            panic!("Expected delete operation");
        }
    }

    #[test]
    fn test_create_insert_delta() {
        let converter = DeltaOperationConverter::new();
        
        let delta = converter.create_insert_delta(5, "World", None);
        assert_eq!(delta.ops().len(), 2); // retain + insert
        
        if let Op::Retain { retain, attributes: _ } = &delta.ops()[0] {
            assert_eq!(*retain, 5);
        } else {
            panic!("Expected retain operation");
        }
        
        if let Op::Insert { insert, attributes: _ } = &delta.ops()[1] {
            assert_eq!(insert, "World");
        } else {
            panic!("Expected insert operation");
        }
    }

    #[test]
    fn test_create_delete_delta() {
        let converter = DeltaOperationConverter::new();
        
        let delta = converter.create_delete_delta(3, 2);
        assert_eq!(delta.ops().len(), 2); // retain + delete
        
        if let Op::Retain { retain, attributes: _ } = &delta.ops()[0] {
            assert_eq!(*retain, 3);
        } else {
            panic!("Expected retain operation");
        }
        
        if let Op::Delete { delete } = &delta.ops()[1] {
            assert_eq!(*delete, 2);
        } else {
            panic!("Expected delete operation");
        }
    }

    #[test]
    fn test_attribute_conversion() {
        let converter = DeltaOperationConverter::new();
        
        let mut dom_attrs = HashMap::new();
        dom_attrs.insert("style".to_string(), "font-weight: bold; color: red".to_string());
        
        let delta_attrs = converter.convert_attributes_to_delta(&dom_attrs);
        
        assert_eq!(delta_attrs.get("bold"), Some(&serde_json::Value::Bool(true)));
        assert_eq!(delta_attrs.get("color"), Some(&serde_json::Value::String("red".to_string())));
    }

    #[test]
    fn test_delta_validation() {
        let converter = DeltaOperationConverter::new();
        
        let valid_delta = Delta::new().retain(5).insert("Hello", None);
        assert!(converter.validate_delta(&valid_delta));
        
        // Test with empty delta
        let empty_delta = Delta::new();
        assert!(converter.validate_delta(&empty_delta));
    }
}