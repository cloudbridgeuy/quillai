use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Node, Text, Window};

/// DOM utility functions for creating and manipulating DOM nodes
pub struct Dom;

impl Dom {
    /// Get the global window object
    pub fn window() -> Result<Window, JsValue> {
        web_sys::window().ok_or_else(|| JsValue::from_str("No global window object"))
    }

    /// Get the document from the window
    pub fn document() -> Result<Document, JsValue> {
        Self::window()?
            .document()
            .ok_or_else(|| JsValue::from_str("No document in window"))
    }

    /// Create an HTML element with the given tag name
    pub fn create_element(tag_name: &str) -> Result<Element, JsValue> {
        Self::document()?.create_element(tag_name)
    }

    /// Create a text node with the given content
    pub fn create_text_node(content: &str) -> Result<Text, JsValue> {
        Ok(Self::document()?.create_text_node(content))
    }

    /// Append a child node to a parent node
    pub fn append_child(parent: &Node, child: &Node) -> Result<Node, JsValue> {
        parent.append_child(child)
    }

    /// Set the text content of a node
    pub fn set_text_content(node: &Node, content: &str) {
        node.set_text_content(Some(content));
    }

    /// Get the text content of a node
    pub fn get_text_content(node: &Node) -> Option<String> {
        node.text_content()
    }

    /// Remove a child node from its parent
    pub fn remove_child(parent: &Node, child: &Node) -> Result<Node, JsValue> {
        parent.remove_child(child)
    }

    /// Insert a node before a reference node
    pub fn insert_before(
        parent: &Node,
        new_node: &Node,
        reference_node: Option<&Node>,
    ) -> Result<Node, JsValue> {
        parent.insert_before(new_node, reference_node)
    }

    /// Replace a child node with a new node
    pub fn replace_child(
        parent: &Node,
        new_child: &Node,
        old_child: &Node,
    ) -> Result<Node, JsValue> {
        parent.replace_child(new_child, old_child)
    }

    /// Create a text node and return it as a Node (convenience method)
    pub fn create_text(content: &str) -> Result<Node, JsValue> {
        Ok(Self::create_text_node(content)?.into())
    }
}
