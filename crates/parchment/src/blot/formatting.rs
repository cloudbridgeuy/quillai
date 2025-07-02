//! Formatting state management for blots
//!
//! This module provides the infrastructure for tracking and managing formatting
//! state on blots, including class attributes, inline styles, and custom attributes.
//! It integrates with the attributor system to provide consistent formatting
//! application and synchronization.

use crate::registry::AttributorTrait;
use std::collections::{HashMap, HashSet};
use wasm_bindgen::prelude::*;

/// Represents the formatting state of a blot
///
/// FormattingState tracks all applied formatting including classes, inline styles,
/// and custom attributes. It provides efficient lookup and modification operations
/// while maintaining consistency with the DOM.
#[derive(Debug, Clone, Default)]
pub struct FormattingState {
    /// Applied CSS classes
    classes: HashSet<String>,
    /// Inline style properties (property -> value)
    styles: HashMap<String, String>,
    /// Custom attributes (name -> value)
    attributes: HashMap<String, String>,
}

impl FormattingState {
    /// Create a new empty formatting state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any formatting is applied
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty() && self.styles.is_empty() && self.attributes.is_empty()
    }

    /// Clear all formatting
    pub fn clear(&mut self) {
        self.classes.clear();
        self.styles.clear();
        self.attributes.clear();
    }

    // === Class Management ===

    /// Add a CSS class
    pub fn add_class(&mut self, class_name: &str) -> bool {
        self.classes.insert(class_name.to_string())
    }

    /// Remove a CSS class
    pub fn remove_class(&mut self, class_name: &str) -> bool {
        self.classes.remove(class_name)
    }

    /// Check if a CSS class is applied
    pub fn has_class(&self, class_name: &str) -> bool {
        self.classes.contains(class_name)
    }

    /// Get all applied classes
    pub fn classes(&self) -> &HashSet<String> {
        &self.classes
    }

    /// Set all classes, replacing existing ones
    pub fn set_classes(&mut self, classes: HashSet<String>) {
        self.classes = classes;
    }

    /// Get classes as a space-separated string
    pub fn classes_string(&self) -> String {
        let mut classes: Vec<String> = self.classes.iter().cloned().collect();
        classes.sort(); // Ensure consistent ordering
        classes.join(" ")
    }

    /// Parse and set classes from a space-separated string
    pub fn parse_classes(&mut self, class_string: &str) {
        self.classes.clear();
        for class in class_string.split_whitespace() {
            if !class.is_empty() {
                self.classes.insert(class.to_string());
            }
        }
    }

    // === Style Management ===

    /// Set a CSS style property
    pub fn set_style(&mut self, property: &str, value: &str) {
        if value.is_empty() {
            self.styles.remove(property);
        } else {
            self.styles.insert(property.to_string(), value.to_string());
        }
    }

    /// Remove a CSS style property
    pub fn remove_style(&mut self, property: &str) -> Option<String> {
        self.styles.remove(property)
    }

    /// Get a CSS style property value
    pub fn get_style(&self, property: &str) -> Option<&String> {
        self.styles.get(property)
    }

    /// Check if a CSS style property is set
    pub fn has_style(&self, property: &str) -> bool {
        self.styles.contains_key(property)
    }

    /// Get all style properties
    pub fn styles(&self) -> &HashMap<String, String> {
        &self.styles
    }

    /// Set all styles, replacing existing ones
    pub fn set_styles(&mut self, styles: HashMap<String, String>) {
        self.styles = styles;
    }

    /// Get styles as a CSS string
    pub fn styles_string(&self) -> String {
        let mut style_pairs: Vec<String> = self.styles
            .iter()
            .map(|(prop, value)| format!("{}: {}", prop, value))
            .collect();
        style_pairs.sort(); // Ensure consistent ordering
        style_pairs.join("; ")
    }

    /// Parse and set styles from a CSS string
    pub fn parse_styles(&mut self, style_string: &str) {
        self.styles.clear();
        for declaration in style_string.split(';') {
            let declaration = declaration.trim();
            if let Some(colon_pos) = declaration.find(':') {
                let property = declaration[..colon_pos].trim();
                let value = declaration[colon_pos + 1..].trim();
                if !property.is_empty() && !value.is_empty() {
                    self.styles.insert(property.to_string(), value.to_string());
                }
            }
        }
    }

    // === Attribute Management ===

    /// Set a custom attribute
    pub fn set_attribute(&mut self, name: &str, value: &str) {
        if value.is_empty() {
            self.attributes.remove(name);
        } else {
            self.attributes.insert(name.to_string(), value.to_string());
        }
    }

    /// Remove a custom attribute
    pub fn remove_attribute(&mut self, name: &str) -> Option<String> {
        self.attributes.remove(name)
    }

    /// Get a custom attribute value
    pub fn get_attribute(&self, name: &str) -> Option<&String> {
        self.attributes.get(name)
    }

    /// Check if a custom attribute is set
    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    /// Get all custom attributes
    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }

    /// Set all attributes, replacing existing ones
    pub fn set_attributes(&mut self, attributes: HashMap<String, String>) {
        self.attributes = attributes;
    }

    // === Getter Methods for Compatibility Checking ===

    /// Get classes for comparison (returns reference to internal HashSet)
    pub fn get_classes(&self) -> &HashSet<String> {
        &self.classes
    }

    /// Get styles for comparison (returns reference to internal HashMap)
    pub fn get_styles(&self) -> &HashMap<String, String> {
        &self.styles
    }

    /// Get attributes for comparison (returns reference to internal HashMap)
    pub fn get_attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }
}

/// Trait for blots that can have formatting applied
///
/// This trait provides the interface for blots to manage their formatting state
/// and integrate with the attributor system.
pub trait FormattableBlot {
    /// Get the formatting state (read-only)
    fn formatting_state(&self) -> &FormattingState;

    /// Get the formatting state (mutable)
    fn formatting_state_mut(&mut self) -> &mut FormattingState;

    /// Apply formatting using an attributor
    ///
    /// This method applies formatting through the attributor system and updates
    /// the internal formatting state to match.
    fn apply_format(&mut self, attributor: &dyn AttributorTrait, value: &JsValue) -> Result<(), JsValue>;

    /// Remove formatting using an attributor
    ///
    /// This method removes formatting through the attributor system and updates
    /// the internal formatting state to match.
    fn remove_format(&mut self, attributor: &dyn AttributorTrait) -> Result<(), JsValue>;

    /// Synchronize formatting state with DOM
    ///
    /// This method reads the current DOM state and updates the internal formatting
    /// state to match. Used during initialization and after external DOM changes.
    fn sync_formatting_from_dom(&mut self) -> Result<(), JsValue>;

    /// Apply formatting state to DOM
    ///
    /// This method applies the current internal formatting state to the DOM.
    /// Used for bidirectional synchronization.
    fn apply_formatting_to_dom(&self) -> Result<(), JsValue>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatting_state_classes() {
        let mut state = FormattingState::new();
        
        // Test adding classes
        assert!(state.add_class("bold"));
        assert!(state.add_class("italic"));
        assert!(!state.add_class("bold")); // Already exists
        
        // Test checking classes
        assert!(state.has_class("bold"));
        assert!(state.has_class("italic"));
        assert!(!state.has_class("underline"));
        
        // Test class string
        let class_string = state.classes_string();
        assert!(class_string.contains("bold"));
        assert!(class_string.contains("italic"));
        
        // Test removing classes
        assert!(state.remove_class("bold"));
        assert!(!state.remove_class("bold")); // Already removed
        assert!(!state.has_class("bold"));
        assert!(state.has_class("italic"));
    }

    #[test]
    fn test_formatting_state_styles() {
        let mut state = FormattingState::new();
        
        // Test setting styles
        state.set_style("color", "red");
        state.set_style("font-size", "14px");
        
        // Test getting styles
        assert_eq!(state.get_style("color"), Some(&"red".to_string()));
        assert_eq!(state.get_style("font-size"), Some(&"14px".to_string()));
        assert_eq!(state.get_style("margin"), None);
        
        // Test style string
        let style_string = state.styles_string();
        assert!(style_string.contains("color: red"));
        assert!(style_string.contains("font-size: 14px"));
        
        // Test removing styles
        assert_eq!(state.remove_style("color"), Some("red".to_string()));
        assert_eq!(state.remove_style("color"), None); // Already removed
        assert!(!state.has_style("color"));
        assert!(state.has_style("font-size"));
    }

    #[test]
    fn test_formatting_state_parse_classes() {
        let mut state = FormattingState::new();
        
        // Test parsing class string
        state.parse_classes("bold italic underline");
        assert!(state.has_class("bold"));
        assert!(state.has_class("italic"));
        assert!(state.has_class("underline"));
        
        // Test with extra whitespace
        state.parse_classes("  bold   italic  ");
        assert!(state.has_class("bold"));
        assert!(state.has_class("italic"));
        assert!(!state.has_class("underline")); // Should be cleared
    }

    #[test]
    fn test_formatting_state_parse_styles() {
        let mut state = FormattingState::new();
        
        // Test parsing style string
        state.parse_styles("color: red; font-size: 14px; margin: 10px");
        assert_eq!(state.get_style("color"), Some(&"red".to_string()));
        assert_eq!(state.get_style("font-size"), Some(&"14px".to_string()));
        assert_eq!(state.get_style("margin"), Some(&"10px".to_string()));
        
        // Test with extra whitespace and semicolons
        state.parse_styles("  color: blue ;  font-weight: bold  ; ");
        assert_eq!(state.get_style("color"), Some(&"blue".to_string()));
        assert_eq!(state.get_style("font-weight"), Some(&"bold".to_string()));
        assert!(!state.has_style("font-size")); // Should be cleared
    }

    #[test]
    fn test_formatting_state_attributes() {
        let mut state = FormattingState::new();
        
        // Test setting attributes
        state.set_attribute("data-id", "123");
        state.set_attribute("aria-label", "Button");
        
        // Test getting attributes
        assert_eq!(state.get_attribute("data-id"), Some(&"123".to_string()));
        assert_eq!(state.get_attribute("aria-label"), Some(&"Button".to_string()));
        assert_eq!(state.get_attribute("title"), None);
        
        // Test removing attributes
        assert_eq!(state.remove_attribute("data-id"), Some("123".to_string()));
        assert_eq!(state.remove_attribute("data-id"), None); // Already removed
        assert!(!state.has_attribute("data-id"));
        assert!(state.has_attribute("aria-label"));
    }

    #[test]
    fn test_formatting_state_empty() {
        let mut state = FormattingState::new();
        assert!(state.is_empty());
        
        state.add_class("test");
        assert!(!state.is_empty());
        
        state.clear();
        assert!(state.is_empty());
        
        state.set_style("color", "red");
        assert!(!state.is_empty());
        
        state.clear();
        assert!(state.is_empty());
        
        state.set_attribute("data-test", "value");
        assert!(!state.is_empty());
        
        state.clear();
        assert!(state.is_empty());
    }
}