use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use wasm_bindgen::prelude::*;
use web_sys::{Element, Node};

use crate::blot::traits_simple::{BlotTrait, RegistryDefinition};
use crate::scope::Scope;

// Error type for registry operations
#[wasm_bindgen]
pub struct ParchmentError {
    message: String,
}

impl ParchmentError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

// Temporary trait for attributors
pub trait AttributorTrait {
    fn attr_name(&self) -> &str;
    fn key_name(&self) -> &str;
    fn scope(&self) -> Scope;
    fn add(&self, node: &Element, value: &JsValue) -> bool;
    fn remove(&self, node: &Element);
    fn value(&self, node: &Element) -> JsValue;
}

/// Global registry for blot definitions using thread-safe OnceLock
static DEFINITION_REGISTRY: OnceLock<Mutex<HashMap<String, RegistryDefinition>>> = OnceLock::new();

#[wasm_bindgen]
pub struct Registry {
    // Registry storage
    blot_names: HashMap<String, String>,
    tag_names: HashMap<String, String>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            blot_names: HashMap::new(),
            tag_names: HashMap::new(),
        }
    }

    /// Register a blot type with its metadata
    pub fn register_blot_type(&mut self, blot_name: &str, tag_name: &str) {
        self.blot_names
            .insert(blot_name.to_string(), tag_name.to_string());
        self.tag_names
            .insert(tag_name.to_string(), blot_name.to_string());
    }

    /// Register a blot definition
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

    /// Query registry by name
    pub fn query_by_name(&self, name: &str) -> Option<&String> {
        self.blot_names.get(name)
    }

    /// Query registry by tag
    pub fn query_by_tag(&self, tag: &str) -> Option<&String> {
        self.tag_names.get(tag)
    }

    /// Query definition by name
    pub fn query_definition(name: &str) -> Option<RegistryDefinition> {
        let registry = DEFINITION_REGISTRY.get()?;
        let map = registry.lock().ok()?;
        map.get(name).cloned()
    }

    /// Register a DOM node with its blot - simplified implementation
    /// Note: In a full implementation, this would use a proper DOM-to-Blot mapping system
    pub fn register_blot_instance(_dom_node: &Node, _blot_ptr: &JsValue) -> Result<(), JsValue> {
        // Simplified implementation - registry functionality will be handled by browser WeakMap
        Ok(())
    }

    /// Unregister a DOM node - simplified implementation
    pub fn unregister_blot_instance(_dom_node: &Node) -> bool {
        // Simplified implementation - cleanup will be handled by browser WeakMap
        true
    }

    /// Find blot by DOM node - simplified implementation
    pub fn find_blot_by_node(_dom_node: &Node) -> Option<JsValue> {
        // Simplified implementation - lookup will be handled by browser-side registry
        None
    }

    /// Create a blot from a DOM node - mirrors TypeScript Registry.create()
    /// Implements proper blot type detection based on DOM node characteristics
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
    }

    #[test]
    fn registry_default() {
        let registry = Registry::default();
        assert_eq!(registry.blot_names.len(), 0);
    }
}
