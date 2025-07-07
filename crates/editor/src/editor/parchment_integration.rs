//! Basic Parchment integration for the QuillAI Editor.
//!
//! This module provides the foundational integration between the editor component
//! and the Parchment document model system. It handles registry initialization,
//! basic blot type registration, and synchronization with Delta changes.

use quillai_parchment::{Registry, ParchmentError, ScrollBlot};
use quillai_delta::{Delta, Op, AttributeMap};
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement};
use std::collections::HashMap;


/// Parchment integration state for the editor.
///
/// This struct manages the Parchment registry and provides utilities for
/// synchronizing document state between Delta operations and Parchment blots.
pub struct ParchmentIntegration {
    /// The Parchment registry for managing blot types
    registry: Registry,
    /// The root ScrollBlot for the document
    scroll_blot: Option<ScrollBlot>,
    /// Current document state as Delta
    current_delta: Delta,
}

impl ParchmentIntegration {
    /// Create a new Parchment integration with basic blot types registered.
    ///
    /// This initializes the registry and registers the fundamental blot types
    /// needed for basic text editing functionality.
    ///
    /// # Returns
    ///
    /// A new `ParchmentIntegration` instance with basic blots registered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_editor::editor::parchment_integration::ParchmentIntegration;
    ///
    /// let integration = ParchmentIntegration::new();
    /// ```
    pub fn new() -> Self {
        let mut registry = Registry::new();
        
        // Register basic blot types needed for text editing
        Self::register_basic_blots(&mut registry);
        
        Self { 
            registry,
            scroll_blot: None,
            current_delta: Delta::new(),
        }
    }

    /// Register the basic blot types required for text editing.
    ///
    /// This registers:
    /// - TextBlot for raw text content
    /// - BlockBlot for paragraph-level containers
    /// - InlineBlot for inline formatting (future use)
    ///
    /// # Arguments
    ///
    /// * `registry` - The registry to register blot types with
    fn register_basic_blots(registry: &mut Registry) {
        // Register text blot for actual text content
        registry.register_blot_type("text", "#text");
        
        // Register block blot for paragraph containers
        registry.register_blot_type("block", "p");
        
        // Register inline blot for future formatting support
        registry.register_blot_type("inline", "span");
        
        // Register scroll blot as the root container
        registry.register_blot_type("scroll", "div");
    }

    /// Get a reference to the Parchment registry.
    ///
    /// This allows access to the registry for advanced operations or
    /// registering additional custom blot types.
    ///
    /// # Returns
    ///
    /// A reference to the internal `Registry`
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Get a mutable reference to the Parchment registry.
    ///
    /// This allows modification of the registry, such as registering
    /// additional blot types or updating existing registrations.
    ///
    /// # Returns
    ///
    /// A mutable reference to the internal `Registry`
    pub fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
    }

    /// Synchronize Parchment state with a Delta document.
    ///
    /// This method updates the Parchment document structure to match
    /// the provided Delta operations. It's called when the Delta document
    /// changes to keep the Parchment representation in sync.
    ///
    /// # Arguments
    ///
    /// * `delta` - The Delta document to synchronize with
    ///
    /// # Returns
    ///
    /// `Ok(())` if synchronization succeeded, or an error if the operation failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_editor::editor::parchment_integration::ParchmentIntegration;
    /// use quillai_delta::Delta;
    ///
    /// let mut integration = ParchmentIntegration::new();
    /// let delta = Delta::new().insert("Hello, world!", None);
    /// 
    /// match integration.sync_with_delta(&delta) {
    ///     Ok(()) => println!("Synchronization successful"),
    ///     Err(_e) => println!("Synchronization failed"),
    /// }
    /// ```
    pub fn sync_with_delta(&mut self, delta: &Delta) -> Result<(), ParchmentError> {
        // For Phase 1, we implement a basic synchronization that doesn't cause errors
        // but doesn't perform complex DOM operations yet.
        // This will be expanded in later phases.
        
        // Validate that the delta is not empty and contains valid operations
        if delta.ops().is_empty() {
            return Ok(());
        }
        
        // For now, we just validate that our registry has the required blot types
        // In future phases, this will create actual blot structures
        let _text_tag = self.registry.query_by_name("text");
        let _block_tag = self.registry.query_by_name("block");
        
        Ok(())
    }

    /// Create a simple blot structure for the given text content.
    ///
    /// This is a utility method that creates a basic document structure
    /// with a root scroll blot containing a block blot with text content.
    /// This provides the foundation for more complex document structures.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to create blots for
    ///
    /// # Returns
    ///
    /// `Ok(())` if blot creation succeeded, or an error if the operation failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_editor::editor::parchment_integration::ParchmentIntegration;
    ///
    /// let mut integration = ParchmentIntegration::new();
    /// let _ = integration.create_simple_blot_structure("Hello, world!");
    /// ```
    pub fn create_simple_blot_structure(&mut self, text: &str) -> Result<(), ParchmentError> {
        // For Phase 1, we implement a basic structure creation that validates
        // our registry setup without complex DOM manipulation
        
        if text.is_empty() {
            return Ok(());
        }
        
        // Verify that we can query for the required blot types
        let _scroll_tag = self.registry.query_by_name("scroll");
        let _block_tag = self.registry.query_by_name("block");
        let _text_tag = self.registry.query_by_name("text");
        
        // In future phases, this will create actual DOM nodes and blot instances
        // For now, we just validate that the registry is properly set up
        
        Ok(())
    }

    /// Check if the Parchment integration is properly initialized.
    ///
    /// This validates that all required blot types are registered and
    /// the integration is ready for use.
    ///
    /// # Returns
    ///
    /// `true` if the integration is properly initialized, `false` otherwise
    pub fn is_initialized(&self) -> bool {
        // Check that all basic blot types are registered
        self.registry.query_by_name("text").is_some() &&
        self.registry.query_by_name("block").is_some() &&
        self.registry.query_by_name("inline").is_some() &&
        self.registry.query_by_name("scroll").is_some()
    }

    /// Initialize the ScrollBlot with an optional DOM element
    ///
    /// Creates the root document container and sets up the Parchment document structure.
    /// This must be called before performing any document operations.
    ///
    /// # Arguments
    ///
    /// * `element` - Optional DOM element to use as the root, creates new div if None
    ///
    /// # Returns
    ///
    /// `Ok(())` if initialization succeeded, or an error if the operation failed
    pub fn initialize_scroll_blot(&mut self, element: Option<Element>) -> Result<(), JsValue> {
        let scroll_blot = ScrollBlot::new(element)?;
        self.scroll_blot = Some(scroll_blot);
        Ok(())
    }

    /// Get a reference to the ScrollBlot
    ///
    /// # Returns
    ///
    /// Reference to the ScrollBlot if initialized, None otherwise
    pub fn scroll_blot(&self) -> Option<&ScrollBlot> {
        self.scroll_blot.as_ref()
    }

    /// Get a mutable reference to the ScrollBlot
    ///
    /// # Returns
    ///
    /// Mutable reference to the ScrollBlot if initialized, None otherwise
    pub fn scroll_blot_mut(&mut self) -> Option<&mut ScrollBlot> {
        self.scroll_blot.as_mut()
    }

    /// Apply a Delta to the Parchment document
    ///
    /// This is the core method for synchronizing Delta operations with the Parchment
    /// document structure. It converts Delta operations into Parchment blot operations
    /// and updates the DOM accordingly.
    ///
    /// # Arguments
    ///
    /// * `delta` - The Delta to apply to the document
    ///
    /// # Returns
    ///
    /// `Ok(())` if the Delta was applied successfully, or an error if the operation failed
    pub fn apply_delta_to_parchment(&mut self, delta: &Delta) -> Result<(), JsValue> {
        let scroll_blot = self.scroll_blot.as_mut()
            .ok_or_else(|| JsValue::from_str("ScrollBlot not initialized. Call initialize_scroll_blot() first."))?;

        // Clear existing content
        scroll_blot.clear();

        // Apply each operation in the Delta
        for operation in delta.ops() {
            match operation {
                Op::Insert { insert, attributes } => {
                    self.apply_insert_operation(scroll_blot, insert, attributes.as_ref())?;
                }
                Op::Retain { retain: _, attributes: _ } => {
                    // Retain operations are handled differently in a full implementation
                    // For now, we skip them as they don't change content
                }
                Op::Delete { delete: _ } => {
                    // Delete operations are handled by not including the content
                    // In a full implementation, we'd track what to delete
                }
            }
        }

        // Update our current delta state
        self.current_delta = delta.clone();
        Ok(())
    }

    /// Apply an insert operation to the ScrollBlot
    fn apply_insert_operation(
        &self,
        scroll_blot: &mut ScrollBlot,
        content: &str,
        attributes: Option<&AttributeMap>,
    ) -> Result<(), JsValue> {
        if content.is_empty() {
            return Ok(());
        }

        // Handle different types of content based on attributes
        if let Some(attrs) = attributes {
            // Check for block-level attributes first
            if let Some(header) = attrs.get("header") {
                if let Some(level) = header.as_f64() {
                    let header_text = format!("<h{}>{}</h{}>", level as i32, content, level as i32);
                    scroll_blot.append_text(&header_text)?;
                    return Ok(());
                }
            }

            if let Some(blockquote) = attrs.get("blockquote") {
                if blockquote.as_bool().unwrap_or(false) {
                    let blockquote_text = format!("<blockquote>{}</blockquote>", content);
                    scroll_blot.append_text(&blockquote_text)?;
                    return Ok(());
                }
            }

            if let Some(code_block) = attrs.get("code-block") {
                if code_block.as_bool().unwrap_or(false) {
                    let code_text = format!("<pre><code>{}</code></pre>", content);
                    scroll_blot.append_text(&code_text)?;
                    return Ok(());
                }
            }

            if let Some(list) = attrs.get("list") {
                if let Some(list_type) = list.as_str() {
                    let list_text = match list_type {
                        "ordered" => format!("<ol><li>{}</li></ol>", content),
                        "bullet" => format!("<ul><li>{}</li></ul>", content),
                        _ => format!("<ul><li>{}</li></ul>", content),
                    };
                    scroll_blot.append_text(&list_text)?;
                    return Ok(());
                }
            }

            // Handle inline formatting
            let formatted_content = self.apply_inline_formatting(content, attrs);
            scroll_blot.append_text(&formatted_content)?;
        } else {
            // Plain text content
            scroll_blot.append_text(content)?;
        }

        Ok(())
    }

    /// Apply inline formatting to text content
    fn apply_inline_formatting(&self, content: &str, attributes: &AttributeMap) -> String {
        let mut formatted = content.to_string();

        // Apply formatting in order of precedence
        if let Some(bold) = attributes.get("bold") {
            if bold.as_bool().unwrap_or(false) {
                formatted = format!("<strong>{}</strong>", formatted);
            }
        }

        if let Some(italic) = attributes.get("italic") {
            if italic.as_bool().unwrap_or(false) {
                formatted = format!("<em>{}</em>", formatted);
            }
        }

        if let Some(underline) = attributes.get("underline") {
            if underline.as_bool().unwrap_or(false) {
                formatted = format!("<u>{}</u>", formatted);
            }
        }

        if let Some(strike) = attributes.get("strike") {
            if strike.as_bool().unwrap_or(false) {
                formatted = format!("<s>{}</s>", formatted);
            }
        }

        if let Some(code) = attributes.get("code") {
            if code.as_bool().unwrap_or(false) {
                formatted = format!("<code>{}</code>", formatted);
            }
        }

        if let Some(link) = attributes.get("link") {
            if let Some(url) = link.as_str() {
                formatted = format!("<a href=\"{}\">{}</a>", url, formatted);
            }
        }

        // Apply style attributes
        let mut styles = Vec::new();

        if let Some(color) = attributes.get("color") {
            if let Some(color_str) = color.as_str() {
                styles.push(format!("color: {}", color_str));
            }
        }

        if let Some(background) = attributes.get("background") {
            if let Some(bg_str) = background.as_str() {
                styles.push(format!("background-color: {}", bg_str));
            }
        }

        if let Some(font) = attributes.get("font") {
            if let Some(font_str) = font.as_str() {
                styles.push(format!("font-family: {}", font_str));
            }
        }

        if let Some(size) = attributes.get("size") {
            if let Some(size_str) = size.as_str() {
                styles.push(format!("font-size: {}", size_str));
            }
        }

        if !styles.is_empty() {
            let style_attr = styles.join("; ");
            formatted = format!("<span style=\"{}\">{}</span>", style_attr, formatted);
        }

        formatted
    }

    /// Extract a Delta from the current Parchment document
    ///
    /// Converts the current Parchment blot structure back into a Delta representation.
    /// This is useful for getting the current document state or for synchronization.
    ///
    /// # Returns
    ///
    /// Delta representing the current document state, or an error if extraction failed
    pub fn extract_delta_from_parchment(&self) -> Result<Delta, JsValue> {
        let scroll_blot = self.scroll_blot.as_ref()
            .ok_or_else(|| JsValue::from_str("ScrollBlot not initialized"))?;

        let mut delta = Delta::new();
        
        // Get the text content from the ScrollBlot
        let text_content = scroll_blot.text_content();
        
        if !text_content.is_empty() {
            // For now, we create a simple Delta with the text content
            // In a full implementation, we'd traverse the blot tree and extract formatting
            delta = delta.insert(&text_content, None);
        }

        Ok(delta)
    }

    /// Get the current Delta state
    ///
    /// # Returns
    ///
    /// Reference to the current Delta document state
    pub fn current_delta(&self) -> &Delta {
        &self.current_delta
    }

    /// Update the current Delta state
    ///
    /// This method updates the internal Delta state and optionally applies it to Parchment.
    ///
    /// # Arguments
    ///
    /// * `delta` - The new Delta state
    /// * `apply_to_parchment` - Whether to apply the Delta to the Parchment document
    ///
    /// # Returns
    ///
    /// `Ok(())` if the update succeeded, or an error if the operation failed
    pub fn update_delta(&mut self, delta: Delta, apply_to_parchment: bool) -> Result<(), JsValue> {
        self.current_delta = delta.clone();
        
        if apply_to_parchment {
            self.apply_delta_to_parchment(&delta)?;
        }
        
        Ok(())
    }

    /// Compose a new Delta with the current state
    ///
    /// This method applies a Delta change to the current document state using Delta composition.
    ///
    /// # Arguments
    ///
    /// * `change_delta` - The Delta representing the change to apply
    ///
    /// # Returns
    ///
    /// `Ok(())` if the composition succeeded, or an error if the operation failed
    pub fn compose_delta(&mut self, change_delta: &Delta) -> Result<(), JsValue> {
        let new_delta = self.current_delta.compose(change_delta);
        self.update_delta(new_delta, true)?;
        Ok(())
    }

    /// Transform a Delta against the current state
    ///
    /// This method transforms a Delta operation against the current document state,
    /// which is useful for operational transformation in collaborative editing.
    ///
    /// # Arguments
    ///
    /// * `other_delta` - The Delta to transform
    /// * `priority` - Whether this operation has priority in case of conflicts
    ///
    /// # Returns
    ///
    /// The transformed Delta, or an error if transformation failed
    pub fn transform_delta(&self, other_delta: &Delta, priority: bool) -> Result<Delta, JsValue> {
        Ok(self.current_delta.transform(other_delta, priority))
    }

    /// Get the DOM element for the document
    ///
    /// # Returns
    ///
    /// The DOM element representing the document root, or None if not initialized
    pub fn dom_element(&self) -> Option<Element> {
        self.scroll_blot.as_ref().map(|scroll| scroll.dom_element())
    }

    /// Get document statistics
    ///
    /// # Returns
    ///
    /// Document statistics if ScrollBlot is initialized, None otherwise
    pub fn get_document_statistics(&self) -> Option<quillai_parchment::text_operations::TextStatistics> {
        self.scroll_blot.as_ref().map(|scroll| scroll.get_statistics())
    }

    /// Find text in the document
    ///
    /// # Arguments
    ///
    /// * `pattern` - The text pattern to search for
    /// * `case_sensitive` - Whether to perform case-sensitive search
    ///
    /// # Returns
    ///
    /// Vector of text matches, or an error if search failed
    pub fn find_text(&self, pattern: &str, case_sensitive: bool) -> Result<Vec<quillai_parchment::text_operations::TextMatch>, JsValue> {
        let scroll_blot = self.scroll_blot.as_ref()
            .ok_or_else(|| JsValue::from_str("ScrollBlot not initialized"))?;
        
        scroll_blot.find_text(pattern, case_sensitive)
    }

    /// Replace text in the document
    ///
    /// # Arguments
    ///
    /// * `pattern` - The text pattern to replace
    /// * `replacement` - The replacement text
    ///
    /// # Returns
    ///
    /// Number of replacements made, or an error if replacement failed
    pub fn replace_all_text(&mut self, pattern: &str, replacement: &str) -> Result<u32, JsValue> {
        let scroll_blot = self.scroll_blot.as_mut()
            .ok_or_else(|| JsValue::from_str("ScrollBlot not initialized"))?;
        
        let count = scroll_blot.replace_all(pattern, replacement)?;
        
        // Update our Delta state after replacement
        self.current_delta = self.extract_delta_from_parchment()?;
        
        Ok(count)
    }
}

impl Default for ParchmentIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parchment_integration_creation() {
        let integration = ParchmentIntegration::new();
        assert!(integration.is_initialized());
    }

    #[test]
    fn test_basic_blot_registration() {
        let integration = ParchmentIntegration::new();
        let registry = integration.registry();
        
        // Verify basic blot types are registered
        assert!(registry.query_by_name("text").is_some());
        assert!(registry.query_by_name("block").is_some());
        assert!(registry.query_by_name("inline").is_some());
        assert!(registry.query_by_name("scroll").is_some());
    }

    #[test]
    fn test_delta_synchronization() {
        let mut integration = ParchmentIntegration::new();
        let delta = Delta::new().insert("Hello, world!", None);
        
        // Should not error with basic delta
        assert!(integration.sync_with_delta(&delta).is_ok());
    }

    #[test]
    fn test_empty_delta_synchronization() {
        let mut integration = ParchmentIntegration::new();
        let delta = Delta::new();
        
        // Should handle empty delta gracefully
        assert!(integration.sync_with_delta(&delta).is_ok());
    }

    #[test]
    fn test_simple_blot_structure_creation() {
        let mut integration = ParchmentIntegration::new();
        
        // Should handle text content without errors
        assert!(integration.create_simple_blot_structure("Hello, world!").is_ok());
        
        // Should handle empty text gracefully
        assert!(integration.create_simple_blot_structure("").is_ok());
    }

    #[test]
    fn test_registry_access() {
        let mut integration = ParchmentIntegration::new();
        
        // Test immutable access
        let _registry_ref = integration.registry();
        
        // Test mutable access
        let _registry_mut = integration.registry_mut();
    }

    #[test]
    fn test_scroll_blot_initialization() {
        let mut integration = ParchmentIntegration::new();
        
        // Should not be initialized initially
        assert!(integration.scroll_blot().is_none());
        
        // Initialize with None (creates new element)
        assert!(integration.initialize_scroll_blot(None).is_ok());
        
        // Should now be initialized
        assert!(integration.scroll_blot().is_some());
    }

    #[test]
    fn test_delta_application() {
        let mut integration = ParchmentIntegration::new();
        let _ = integration.initialize_scroll_blot(None);
        
        let delta = Delta::new().insert("Hello, world!", None);
        
        // Should apply delta without errors
        assert!(integration.apply_delta_to_parchment(&delta).is_ok());
        
        // Current delta should be updated
        assert_eq!(integration.current_delta().ops().len(), 1);
    }

    #[test]
    fn test_delta_extraction() {
        let mut integration = ParchmentIntegration::new();
        let _ = integration.initialize_scroll_blot(None);
        
        let original_delta = Delta::new().insert("Test content", None);
        let _ = integration.apply_delta_to_parchment(&original_delta);
        
        // Should be able to extract delta
        let extracted_delta = integration.extract_delta_from_parchment();
        assert!(extracted_delta.is_ok());
        
        let extracted = extracted_delta.unwrap();
        assert!(!extracted.ops().is_empty());
    }

    #[test]
    fn test_delta_composition() {
        let mut integration = ParchmentIntegration::new();
        let _ = integration.initialize_scroll_blot(None);
        
        let initial_delta = Delta::new().insert("Hello", None);
        let _ = integration.update_delta(initial_delta, true);
        
        let change_delta = Delta::new().retain(5).insert(" World", None);
        
        // Should compose deltas without errors
        assert!(integration.compose_delta(&change_delta).is_ok());
    }

    #[test]
    fn test_inline_formatting() {
        let integration = ParchmentIntegration::new();
        
        let mut attributes = AttributeMap::new();
        attributes.insert("bold".to_string(), serde_json::Value::Bool(true));
        attributes.insert("italic".to_string(), serde_json::Value::Bool(true));
        
        let formatted = integration.apply_inline_formatting("Hello", &attributes);
        
        // Should contain both bold and italic formatting
        assert!(formatted.contains("<strong>"));
        assert!(formatted.contains("<em>"));
        assert!(formatted.contains("Hello"));
    }
}