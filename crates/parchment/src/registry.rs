//!
//! The Registry module provides the core infrastructure for managing blot types,
//! their metadata, and the mapping between DOM nodes and blot instances. It serves
//! as the central coordination point for the Parchment document model.
//!
//! ## Key Responsibilities
//!
//! - **Type Registration**: Register blot types with their tag names and metadata
//! - **DOM Mapping**: Create appropriate blot instances from DOM nodes
//! - **Lookup Operations**: Find blot types by name or tag
//! - **Instance Management**: Track DOM node to blot instance relationships
//!
//! ## Architecture
//!
//! The registry uses a two-tier system:
//! 1. **Type Registry**: Maps blot names ↔ tag names for type lookup
//! 2. **Definition Registry**: Thread-safe global storage for blot definitions
//!
//! ## Usage
//!
//! ```rust
//! use quillai_parchment::Registry;
//!
//! let mut registry = Registry::new();
//! registry.register_blot_type("text", "span");
//!
//! // Query by name or tag
//! let tag = registry.query_by_name("text");
//! let name = registry.query_by_tag("span");
//! ```

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use wasm_bindgen::prelude::*;
use web_sys::{Element, Node};

use crate::blot::traits_simple::{BlotTrait, RegistryDefinition};
use crate::scope::Scope;

/// DOM-to-Blot mapping system
pub mod dom_map;

use dom_map::DomBlotMap;

/// Error type for registry operations and blot creation failures
///
/// Provides structured error handling for registry operations, including
/// blot registration failures, DOM node processing errors, and lookup failures.
#[wasm_bindgen]
pub struct ParchmentError {
    message: String,
}

impl ParchmentError {
    /// Create a new ParchmentError with the given message
    ///
    /// # Parameters
    /// * `message` - Error description
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }

    /// Get the error message
    ///
    /// # Returns
    /// Reference to the error message string
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Trait for attributor implementations that manage formatting attributes
///
/// Attributors handle the application, removal, and querying of formatting
/// attributes on DOM elements. This trait provides a common interface for
/// different attributor types (base, class, style).
pub trait AttributorTrait {
    /// Get the attribute name this attributor manages
    fn attr_name(&self) -> &str;

    /// Get the key name used for this attributor
    fn key_name(&self) -> &str;

    /// Get the scope this attributor operates within
    fn scope(&self) -> Scope;

    /// Add/apply the attributor's formatting to a DOM element
    ///
    /// # Parameters
    /// * `node` - DOM element to apply formatting to
    /// * `value` - Formatting value to apply
    ///
    /// # Returns
    /// `true` if the formatting was successfully applied
    fn add(&self, node: &Element, value: &JsValue) -> bool;

    /// Remove the attributor's formatting from a DOM element
    ///
    /// # Parameters
    /// * `node` - DOM element to remove formatting from
    fn remove(&self, node: &Element);

    /// Get the current value of this attributor on a DOM element
    ///
    /// # Parameters
    /// * `node` - DOM element to query
    ///
    /// # Returns
    /// Current attributor value as JsValue
    fn value(&self, node: &Element) -> JsValue;
}

/// Global registry for blot definitions using thread-safe OnceLock
///
/// This static registry provides thread-safe access to blot definitions across
/// the entire application. It uses OnceLock for lazy initialization and Mutex
/// for thread-safe access to the underlying HashMap.
static DEFINITION_REGISTRY: OnceLock<Mutex<HashMap<String, RegistryDefinition>>> = OnceLock::new();

/// Central registry for managing blot types and DOM-to-blot mappings
///
/// The Registry serves as the coordination point for the Parchment document model,
/// managing the relationships between blot names, DOM tag names, and blot instances.
/// It provides both type-level registration and instance-level DOM mapping.
///
/// # Examples
///
/// ```rust
/// use quillai_parchment::Registry;
///
/// let mut registry = Registry::new();
///
/// // Register a blot type
/// registry.register_blot_type("paragraph", "p");
///
/// // Query by name or tag
/// assert_eq!(registry.query_by_name("paragraph"), Some(&"p".to_string()));
/// assert_eq!(registry.query_by_tag("p"), Some(&"paragraph".to_string()));
/// ```
#[wasm_bindgen]
pub struct Registry {
    /// Maps blot names to their corresponding DOM tag names
    blot_names: HashMap<String, String>,
    /// Maps DOM tag names to their corresponding blot names
    tag_names: HashMap<String, String>,
    /// DOM-to-Blot mapping system
    dom_blot_map: DomBlotMap,
}

impl Registry {
    /// Create a new empty registry
    ///
    /// Initializes a new registry with empty blot name and tag name mappings.
    /// This is typically used when creating a fresh document or editor instance.
    ///
    /// # Returns
    /// New Registry instance with empty mappings
    pub fn new() -> Self {
        Self {
            blot_names: HashMap::new(),
            tag_names: HashMap::new(),
            dom_blot_map: DomBlotMap::new(),
        }
    }

    /// Register a blot type with its corresponding DOM tag name
    ///
    /// Creates a bidirectional mapping between a blot name and its DOM tag name,
    /// allowing lookup in both directions. This is essential for the DOM-to-blot
    /// creation process and blot-to-DOM serialization.
    ///
    /// # Parameters
    /// * `blot_name` - The name of the blot type (e.g., "paragraph", "bold")
    /// * `tag_name` - The corresponding DOM tag name (e.g., "p", "strong")
    ///
    /// # Examples
    /// ```rust
    /// use quillai_parchment::Registry;
    ///
    /// let mut registry = Registry::new();
    /// registry.register_blot_type("paragraph", "p");
    /// registry.register_blot_type("bold", "strong");
    /// ```
    pub fn register_blot_type(&mut self, blot_name: &str, tag_name: &str) {
        // If this blot name was previously registered with a different tag, clean up the old reverse mapping
        if let Some(old_tag) = self.blot_names.get(blot_name) {
            if old_tag != tag_name {
                self.tag_names.remove(old_tag);
            }
        }
        
        // If this tag was previously registered with a different blot name, clean up the old forward mapping
        if let Some(old_blot) = self.tag_names.get(tag_name) {
            if old_blot != blot_name {
                self.blot_names.remove(old_blot);
            }
        }
        
        // Insert new mappings
        self.blot_names
            .insert(blot_name.to_string(), tag_name.to_string());
        self.tag_names
            .insert(tag_name.to_string(), blot_name.to_string());
    }

    /// Register a blot definition in the global registry
    ///
    /// Stores a complete blot definition in the thread-safe global registry.
    /// This enables runtime blot creation and type checking.
    ///
    /// # Parameters
    /// * `definition` - Complete blot definition with metadata
    ///
    /// # Returns
    /// `Ok(())` on success, `Err(String)` if registry lock fails
    pub fn register_definition(definition: RegistryDefinition) -> Result<(), String> {
        let registry = DEFINITION_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));

        match registry.lock() {
            Ok(mut map) => {
                map.insert(definition.blot_name.clone(), definition);
                Ok(())
            }
            Err(_) => Err("Failed to acquire registry lock".to_string()),
        }
    }

    /// Query DOM tag name by blot name
    ///
    /// # Parameters
    /// * `name` - Blot name to look up
    ///
    /// # Returns
    /// DOM tag name if found, None otherwise
    pub fn query_by_name(&self, name: &str) -> Option<&String> {
        self.blot_names.get(name)
    }

    /// Query blot name by DOM tag name
    ///
    /// # Parameters
    /// * `tag` - DOM tag name to look up
    ///
    /// # Returns
    /// Blot name if found, None otherwise
    pub fn query_by_tag(&self, tag: &str) -> Option<&String> {
        self.tag_names.get(tag)
    }

    /// Query blot definition from global registry
    ///
    /// # Parameters
    /// * `name` - Blot name to look up
    ///
    /// # Returns
    /// Cloned blot definition if found, None otherwise
    pub fn query_definition(name: &str) -> Option<RegistryDefinition> {
        let registry = DEFINITION_REGISTRY.get()?;
        let map = registry.lock().ok()?;
        map.get(name).cloned()
    }

    /// Register a blot for a DOM node
    ///
    /// Creates a bidirectional mapping between a DOM node and its blot instance.
    /// This is the core method for associating blots with their DOM representations.
    ///
    /// # Parameters
    /// * `node` - DOM node to associate with the blot
    /// * `blot_ptr` - Raw pointer to the blot instance
    ///
    /// # Returns
    /// `Ok(())` on success, `Err(JsValue)` on failure
    ///
    /// # Safety
    /// The caller must ensure that the blot pointer remains valid for the lifetime
    /// of the mapping. The blot should be unregistered before being dropped.
    pub fn register_blot_for_node(
        &mut self,
        node: &Node,
        blot_ptr: *mut dyn BlotTrait,
    ) -> Result<(), JsValue> {
        self.dom_blot_map.register_blot(node, blot_ptr)
    }

    /// Find blot associated with a DOM node
    ///
    /// Looks up the blot instance associated with the given DOM node.
    /// Returns None if no mapping exists.
    ///
    /// # Parameters
    /// * `node` - DOM node to look up
    ///
    /// # Returns
    /// Blot pointer if found, None otherwise
    pub fn find_blot_for_node(&mut self, node: &Node) -> Option<*mut dyn BlotTrait> {
        self.dom_blot_map.find_blot_by_node(node)
    }

    /// Unregister a blot by its DOM node
    ///
    /// Removes the mapping between a DOM node and its blot instance.
    /// Safe to call multiple times for the same node.
    ///
    /// # Parameters
    /// * `node` - DOM node to unregister
    ///
    /// # Returns
    /// `true` if a mapping existed and was removed, `false` otherwise
    pub fn unregister_blot_for_node(&mut self, node: &Node) -> bool {
        self.dom_blot_map.unregister_blot_by_node(node)
    }

    /// Clean up mappings for disconnected DOM nodes
    ///
    /// Performs automatic cleanup of mappings for DOM nodes that are no longer
    /// connected to the document. This helps prevent memory leaks.
    ///
    /// # Returns
    /// Number of mappings that were cleaned up
    pub fn cleanup_disconnected_nodes(&mut self) -> usize {
        self.dom_blot_map.cleanup_disconnected_nodes()
    }

    /// Get statistics about the DOM-to-Blot mapping system
    ///
    /// Returns information about the current state of the mapping system,
    /// useful for debugging and performance monitoring.
    pub fn get_mapping_stats(&self) -> dom_map::DomBlotMapStats {
        self.dom_blot_map.get_stats()
    }

    /// Validate the consistency of the DOM-to-Blot mapping system
    ///
    /// Performs internal consistency checks on the mapping data structures.
    /// Useful for debugging and ensuring data integrity.
    ///
    /// # Returns
    /// `Ok(())` if consistent, `Err(String)` with error description if not
    pub fn validate_mapping_consistency(&self) -> Result<(), String> {
        self.dom_blot_map.validate_consistency()
    }

    // Legacy methods for backward compatibility
    /// Register a DOM node with its blot - legacy method
    pub fn register_blot_instance(_dom_node: &Node, _blot_ptr: &JsValue) -> Result<(), JsValue> {
        // Legacy method - use register_blot_for_node instead
        web_sys::console::warn_1(&JsValue::from_str(
            "register_blot_instance is deprecated, use register_blot_for_node instead"
        ));
        Ok(())
    }

    /// Unregister a DOM node - legacy method
    pub fn unregister_blot_instance(_dom_node: &Node) -> bool {
        // Legacy method - use unregister_blot_for_node instead
        web_sys::console::warn_1(&JsValue::from_str(
            "unregister_blot_instance is deprecated, use unregister_blot_for_node instead"
        ));
        true
    }

    /// Find blot by DOM node - legacy method
    pub fn find_blot_by_node(_dom_node: &Node) -> Option<JsValue> {
        // Legacy method - use find_blot_for_node instead
        web_sys::console::warn_1(&JsValue::from_str(
            "find_blot_by_node is deprecated, use find_blot_for_node instead"
        ));
        None
    }

    /// Create appropriate blot instance from DOM node
    ///
    /// Analyzes a DOM node and creates the corresponding blot type based on
    /// node type, tag name, and CSS properties. This is the core method for
    /// converting existing DOM content into the Parchment document model.
    ///
    /// # Parameters
    /// * `dom_node` - DOM node to convert to blot
    ///
    /// # Returns
    /// Boxed blot trait object on success, JsValue error on failure
    ///
    /// # Examples
    /// ```javascript
    /// // From JavaScript after WASM init
    /// const pElement = document.createElement('p');
    /// const blot = Registry.create_blot_from_node(pElement);
    /// ```
    pub fn create_blot_from_node(dom_node: &Node) -> Result<Box<dyn BlotTrait>, JsValue> {
        use web_sys::{Element, Text};

        match dom_node.node_type() {
            // Text nodes become TextBlots
            Node::TEXT_NODE => {
                if let Some(text_node) = dom_node.dyn_ref::<Text>() {
                    let text_content = text_node.text_content().unwrap_or_default();
                    let text_blot =
                        crate::blot::text::TextBlot::from_node(dom_node.clone(), &text_content)?;
                    Ok(Box::new(text_blot))
                } else {
                    Err("Failed to cast text node".into())
                }
            }

            // Element nodes - determine blot type based on tag name and attributes
            Node::ELEMENT_NODE => {
                if let Some(element) = dom_node.dyn_ref::<Element>() {
                    let tag_name = element.tag_name().to_lowercase();

                    match tag_name.as_str() {
                        // Block-level elements
                        "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote"
                        | "pre" | "ol" | "ul" | "li" => {
                            // Check if this is a scroll container
                            if element.class_list().contains("parchment-scroll") {
                                let scroll_blot =
                                    crate::blot::scroll::ScrollBlot::from_element(element.clone());
                                Ok(Box::new(scroll_blot))
                            } else {
                                let block_blot =
                                    crate::blot::block::BlockBlot::from_element(element.clone());
                                Ok(Box::new(block_blot))
                            }
                        }

                        // Inline elements
                        "span" | "strong" | "em" | "b" | "i" | "u" | "s" | "code" => {
                            let inline_blot =
                                crate::blot::inline::InlineBlot::from_element(element.clone());
                            Ok(Box::new(inline_blot))
                        }

                        // Link elements (also inline but might need special handling)
                        "a" => {
                            let inline_blot =
                                crate::blot::inline::InlineBlot::from_element(element.clone());
                            Ok(Box::new(inline_blot))
                        }

                        // Embedded elements (void/self-closing)
                        "img" | "br" | "hr" | "input" | "video" | "audio" | "iframe" => {
                            let embed_blot =
                                crate::blot::embed::EmbedBlot::from_element(element.clone());
                            Ok(Box::new(embed_blot))
                        }

                        // Default: treat unknown elements as block blots
                        _ => {
                            // Check if element has inline characteristics
                            let computed_display = Self::get_computed_display(element);

                            if computed_display == "inline" || computed_display == "inline-block" {
                                let inline_blot =
                                    crate::blot::inline::InlineBlot::from_element(element.clone());
                                Ok(Box::new(inline_blot))
                            } else {
                                let block_blot =
                                    crate::blot::block::BlockBlot::from_element(element.clone());
                                Ok(Box::new(block_blot))
                            }
                        }
                    }
                } else {
                    Err("Failed to cast element node".into())
                }
            }

            // Ignore other node types (comments, processing instructions, etc.)
            _ => Err(format!("Unsupported node type: {}", dom_node.node_type()).into()),
        }
    }

    /// Helper method to get computed display style using window.getComputedStyle
    fn get_computed_display(element: &Element) -> String {
        // Try to get computed style using window.getComputedStyle
        if let Some(window) = web_sys::window() {
            if let Ok(Some(style)) = window.get_computed_style(element) {
                if let Ok(display) = style.get_property_value("display") {
                    if !display.is_empty() {
                        return display;
                    }
                }
            }
        }

        // Fallback: check inline style attribute
        element
            .get_attribute("style")
            .and_then(|style| {
                if style.contains("display:") {
                    Some(
                        style
                            .split("display:")
                            .nth(1)?
                            .split(';')
                            .next()?
                            .trim()
                            .to_string(),
                    )
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "block".to_string())
    }

    /// Static convenience methods for global registry access
    pub fn register_blot(dom_node: &Node, blot_ptr: &JsValue) -> Result<(), JsValue> {
        Self::register_blot_instance(dom_node, blot_ptr)
    }

    pub fn unregister_blot(dom_node: &Node) -> bool {
        Self::unregister_blot_instance(dom_node)
    }

    pub fn find_blot(dom_node: &Node) -> Option<JsValue> {
        Self::find_blot_by_node(dom_node)
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn registry_creation() {
        let registry = Registry::new();
        assert_eq!(registry.blot_names.len(), 0);
        assert_eq!(registry.tag_names.len(), 0);
        
        let stats = registry.get_mapping_stats();
        assert_eq!(stats.total_mappings, 0);
    }

    #[test]
    fn registry_default() {
        let registry = Registry::default();
        assert_eq!(registry.blot_names.len(), 0);
        assert_eq!(registry.tag_names.len(), 0);
    }

    #[test]
    fn blot_type_registration() {
        let mut registry = Registry::new();
        
        registry.register_blot_type("paragraph", "p");
        registry.register_blot_type("bold", "strong");
        
        assert_eq!(registry.query_by_name("paragraph"), Some(&"p".to_string()));
        assert_eq!(registry.query_by_name("bold"), Some(&"strong".to_string()));
        assert_eq!(registry.query_by_tag("p"), Some(&"paragraph".to_string()));
        assert_eq!(registry.query_by_tag("strong"), Some(&"bold".to_string()));
        
        // Test non-existent lookups
        assert_eq!(registry.query_by_name("nonexistent"), None);
        assert_eq!(registry.query_by_tag("nonexistent"), None);
    }

    #[test]
    fn blot_type_overwrite() {
        let mut registry = Registry::new();
        
        registry.register_blot_type("test", "div");
        assert_eq!(registry.query_by_name("test"), Some(&"div".to_string()));
        
        // Overwrite with new tag
        registry.register_blot_type("test", "span");
        assert_eq!(registry.query_by_name("test"), Some(&"span".to_string()));
        assert_eq!(registry.query_by_tag("span"), Some(&"test".to_string()));
        
        // Old tag should no longer map to "test"
        assert_ne!(registry.query_by_tag("div"), Some(&"test".to_string()));
    }

    #[test]
    fn mapping_consistency_validation() {
        let registry = Registry::new();
        
        // Empty registry should be consistent
        assert!(registry.validate_mapping_consistency().is_ok());
    }

    #[test]
    fn mapping_stats() {
        let registry = Registry::new();
        let stats = registry.get_mapping_stats();
        
        assert_eq!(stats.total_mappings, 0);
        assert_eq!(stats.node_metadata_count, 0);
        assert_eq!(stats.blot_pointers_count, 0);
        assert_eq!(stats.next_node_id, 1);
        assert_eq!(stats.next_blot_id, 1);
    }

    // Note: DOM-related tests require WASM environment and are in separate test files
    // These tests focus on the non-DOM functionality that can be tested in native Rust

    #[test]
    fn error_creation() {
        let error = ParchmentError::new("test error");
        assert_eq!(error.message(), "test error");
    }

    #[test]
    fn definition_registry_operations() {
        use crate::blot::traits_simple::RegistryDefinition;
        use crate::scope::Scope;
        
        let definition = RegistryDefinition {
            blot_name: "test_blot".to_string(),
            tag_name: "div".to_string(),
            scope: Scope::Block,
            class_name: Some("test-class".to_string()),
        };
        
        // Test registration
        assert!(Registry::register_definition(definition.clone()).is_ok());
        
        // Test query
        let retrieved = Registry::query_definition("test_blot");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.blot_name, "test_blot");
        assert_eq!(retrieved.tag_name, "div");
        
        // Test non-existent query
        assert!(Registry::query_definition("nonexistent").is_none());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn legacy_method_warnings() {
        // These methods should work but emit warnings
        // We can't easily test the warnings in unit tests, but we can test they don't panic
        
        let result = Registry::register_blot_instance(
            &wasm_bindgen::JsValue::NULL.unchecked_into(),
            &wasm_bindgen::JsValue::NULL
        );
        assert!(result.is_ok());
        
        let result = Registry::unregister_blot_instance(
            &wasm_bindgen::JsValue::NULL.unchecked_into()
        );
        assert_eq!(result, true);
        
        let result = Registry::find_blot_by_node(
            &wasm_bindgen::JsValue::NULL.unchecked_into()
        );
        assert!(result.is_none());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn static_convenience_methods() {
        // Test the static convenience methods
        let result = Registry::register_blot(
            &wasm_bindgen::JsValue::NULL.unchecked_into(),
            &wasm_bindgen::JsValue::NULL
        );
        assert!(result.is_ok());
        
        let result = Registry::unregister_blot(
            &wasm_bindgen::JsValue::NULL.unchecked_into()
        );
        assert_eq!(result, true);
        
        let result = Registry::find_blot(
            &wasm_bindgen::JsValue::NULL.unchecked_into()
        );
        assert!(result.is_none());
    }
}
