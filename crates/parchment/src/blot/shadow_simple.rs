use crate::blot::traits_simple::{create_element_with_class, BlotTrait};
use crate::registry::Registry;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::Node;

/// ShadowBlot is the base implementation for all Blots
/// This mirrors the TypeScript ShadowBlot class (simplified)
pub struct ShadowBlot {
    pub dom_node: Node,
}

impl ShadowBlot {
    /// Create a new ShadowBlot - mirrors TypeScript constructor
    pub fn new(dom_node: Node) -> Result<Self, JsValue> {
        let blot = ShadowBlot { dom_node };

        // Note: Registry registration handled by caller
        // Registry::register_blot(&blot.dom_node, &blot)?;

        Ok(blot)
    }

    /// Create DOM node - mirrors TypeScript static create method
    pub fn create_dom_node(
        tag_name: &str,
        class_name: Option<&str>,
        value: Option<&JsValue>,
    ) -> Result<Node, JsValue> {
        if tag_name.is_empty() {
            return Err("Blot definition missing tagName".into());
        }

        let element = create_element_with_class(tag_name, class_name)?;

        // Handle values
        if let Some(val) = value {
            if let Some(text_value) = val.as_string() {
                element.set_text_content(Some(&text_value));
            }
        }

        Ok(element.into())
    }
}

impl BlotTrait for ShadowBlot {
    fn get_blot_name(&self) -> &'static str {
        "abstract"
    }

    fn get_tag_name(&self) -> &'static str {
        "div" // Default tag name
    }

    fn get_scope(&self) -> Scope {
        Scope::Block
    }

    fn dom_node(&self) -> &Node {
        &self.dom_node
    }

    fn length(&self) -> usize {
        1 // Default length for base blot
    }

    /// Attach this blot - mirrors TypeScript attach()
    fn attach(&mut self) {
        // Base implementation does nothing, overridden by subclasses
    }

    /// Detach this blot - mirrors TypeScript detach()
    fn detach(&mut self) {
        Registry::unregister_blot(&self.dom_node);
    }

    /// Remove this blot - mirrors TypeScript remove()
    fn remove(&mut self) {
        if let Some(parent_node) = self.dom_node.parent_node() {
            let _ = parent_node.remove_child(&self.dom_node);
        }
        self.detach();
    }

    /// Delete at index - mirrors TypeScript deleteAt()
    fn delete_at(&mut self, _index: usize, _length: usize) {
        // Default implementation removes the entire blot
        self.remove();
    }

    /// Insert at index - mirrors TypeScript insertAt()
    fn insert_at(&mut self, _index: usize, value: &str) {
        // Default implementation sets text content
        self.dom_node.set_text_content(Some(value));
    }

    /// Support for downcasting - needed for tree navigation
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Drop for ShadowBlot {
    fn drop(&mut self) {
        // Ensure we unregister from the registry when dropped
        Registry::unregister_blot(&self.dom_node);
    }
}
