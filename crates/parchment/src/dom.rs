//! DOM manipulation utilities for WebAssembly environments.
//!
//! This module provides a safe, ergonomic interface for DOM operations in
//! WebAssembly contexts. It wraps the `web_sys` APIs with error handling
//! and convenience methods optimized for Parchment's document operations.
//!
//! # Core Functionality
//!
//! - **Element Creation**: Safe creation of HTML elements and text nodes
//! - **Tree Manipulation**: Insertion, removal, and replacement of nodes
//! - **Content Management**: Setting and getting text content
//! - **Error Handling**: Proper error propagation for DOM operations
//!
//! # Usage Examples
//!
//! ## Creating Elements
//!
//! ```rust,no_run
//! use quillai_parchment::dom::Dom;
//!
//! // Create a paragraph element
//! let p = Dom::create_element("p")?;
//!
//! // Create a text node
//! let text = Dom::create_text_node("Hello, world!")?;
//!
//! // Append text to paragraph
//! Dom::append_child(&p, &text)?;
//! # Ok::<(), wasm_bindgen::JsValue>(())
//! ```
//!
//! ## Manipulating Content
//!
//! ```rust,no_run
//! use quillai_parchment::dom::Dom;
//!
//! let element = Dom::create_element("div")?;
//! Dom::set_text_content(&element, "New content");
//!
//! let content = Dom::get_text_content(&element);
//! assert_eq!(content, Some("New content".to_string()));
//! # Ok::<(), wasm_bindgen::JsValue>(())
//! ```
//!
//! # WebAssembly Integration
//!
//! All methods are designed to work seamlessly in WebAssembly environments,
//! providing the DOM manipulation capabilities needed for rich text editing
//! and document management in web browsers.

use wasm_bindgen::prelude::*;
use web_sys::{Element, Node, Text};

use crate::utils::dom;

/// Utility functions for DOM manipulation in WebAssembly environments.
///
/// This struct provides static methods for common DOM operations, wrapping
/// the `web_sys` APIs with proper error handling and ergonomic interfaces.
/// All methods are designed to be safe and handle edge cases gracefully.
///
/// # Design Principles
///
/// - **Safety**: All operations include proper error handling
/// - **Ergonomics**: Simple, intuitive method signatures
/// - **Performance**: Minimal overhead over raw `web_sys` calls
/// - **Compatibility**: Works across all modern browsers
pub struct Dom;

impl Dom {
    /// Get the global window object.
    ///
    /// This method provides access to the browser's global window object,
    /// which is the entry point for most DOM operations.
    ///
    /// # Returns
    ///
    /// * `Ok(Window)` - The global window object
    /// * `Err(JsValue)` - If no window object is available (e.g., in Node.js)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let window = Dom::window()?;
    /// // Use window for further DOM operations
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn window() -> Result<web_sys::Window, JsValue> {
        dom::window()
    }

    /// Get the document object from the global window.
    ///
    /// This method provides access to the browser's document object,
    /// which is used for creating and manipulating DOM elements.
    ///
    /// # Returns
    ///
    /// * `Ok(Document)` - The document object
    /// * `Err(JsValue)` - If no document is available
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let document = Dom::document()?;
    /// // Use document to create elements
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn document() -> Result<web_sys::Document, JsValue> {
        dom::document()
    }

    /// Create an HTML element with the specified tag name.
    ///
    /// This method creates a new HTML element of the given type. The element
    /// is created but not yet attached to the document tree.
    ///
    /// # Arguments
    ///
    /// * `tag_name` - The HTML tag name (e.g., "div", "p", "span")
    ///
    /// # Returns
    ///
    /// * `Ok(Element)` - The newly created element
    /// * `Err(JsValue)` - If element creation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let div = Dom::create_element("div")?;
    /// let paragraph = Dom::create_element("p")?;
    /// let span = Dom::create_element("span")?;
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn create_element(tag_name: &str) -> Result<Element, JsValue> {
        dom::create_element(tag_name)
    }

    /// Create a text node with the specified content.
    ///
    /// This method creates a new text node containing the given string content.
    /// Text nodes represent the actual text content within HTML elements.
    ///
    /// # Arguments
    ///
    /// * `content` - The text content for the node
    ///
    /// # Returns
    ///
    /// * `Ok(Text)` - The newly created text node
    /// * `Err(JsValue)` - If text node creation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let text = Dom::create_text_node("Hello, world!")?;
    /// let empty_text = Dom::create_text_node("")?;
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn create_text_node(content: &str) -> Result<Text, JsValue> {
        dom::create_text_node(content)
    }

    /// Append a child node to a parent node.
    ///
    /// This method adds the child node to the end of the parent's child list.
    /// If the child is already in the document tree, it will be moved to the
    /// new location.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent node to append to
    /// * `child` - The child node to append
    ///
    /// # Returns
    ///
    /// * `Ok(Node)` - The appended child node
    /// * `Err(JsValue)` - If the append operation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let div = Dom::create_element("div")?;
    /// let text = Dom::create_text_node("Hello")?;
    /// Dom::append_child(&div, &text)?;
    ///
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn append_child(parent: &Node, child: &Node) -> Result<Node, JsValue> {
        parent.append_child(child)
    }

    /// Set the text content of a node.
    ///
    /// This method replaces all child nodes of the target node with a single
    /// text node containing the specified content. This is useful for setting
    /// the text content of elements.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to set text content for
    /// * `content` - The new text content
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let div = Dom::create_element("div")?;
    /// Dom::set_text_content(&div, "New content");
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn set_text_content(node: &Node, content: &str) {
        node.set_text_content(Some(content));
    }

    /// Get the text content of a node.
    ///
    /// This method returns the text content of the node and all its descendants.
    /// For text nodes, this returns the text data. For elements, this returns
    /// the concatenated text of all descendant text nodes.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to get text content from
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The text content if available
    /// * `None` - If the node has no text content
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let div = Dom::create_element("div")?;
    /// Dom::set_text_content(&div, "Hello");
    /// let content = Dom::get_text_content(&div);
    /// assert_eq!(content, Some("Hello".to_string()));
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn get_text_content(node: &Node) -> Option<String> {
        node.text_content()
    }

    /// Remove a child node from its parent.
    ///
    /// This method removes the specified child node from the parent's child list.
    /// The removed node can be reinserted elsewhere in the document.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent node to remove from
    /// * `child` - The child node to remove
    ///
    /// # Returns
    ///
    /// * `Ok(Node)` - The removed child node
    /// * `Err(JsValue)` - If the removal operation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let div = Dom::create_element("div")?;
    /// let text = Dom::create_text_node("Hello")?;
    /// Dom::append_child(&div, &text)?;
    /// let removed = Dom::remove_child(&div, &text)?;
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn remove_child(parent: &Node, child: &Node) -> Result<Node, JsValue> {
        parent.remove_child(child)
    }

    /// Insert a node before a reference node.
    ///
    /// This method inserts the new node as a child of the parent, positioned
    /// immediately before the reference node. If the reference node is `None`,
    /// the new node is appended to the end of the child list.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent node to insert into
    /// * `new_node` - The node to insert
    /// * `reference_node` - The node to insert before (or `None` to append)
    ///
    /// # Returns
    ///
    /// * `Ok(Node)` - The inserted node
    /// * `Err(JsValue)` - If the insertion operation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let div = Dom::create_element("div")?;
    /// let first = Dom::create_text_node("First")?;
    /// let second = Dom::create_text_node("Second")?;
    ///
    /// Dom::append_child(&div, &second)?;
    /// Dom::insert_before(&div, &first, Some(&second))?;
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn insert_before(
        parent: &Node,
        new_node: &Node,
        reference_node: Option<&Node>,
    ) -> Result<Node, JsValue> {
        parent.insert_before(new_node, reference_node)
    }

    /// Replace a child node with a new node.
    ///
    /// This method replaces an existing child node with a new node in the
    /// parent's child list. The old child is removed and the new child takes
    /// its position.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent node containing the child to replace
    /// * `new_child` - The new node to insert
    /// * `old_child` - The existing child node to replace
    ///
    /// # Returns
    ///
    /// * `Ok(Node)` - The replaced (old) child node
    /// * `Err(JsValue)` - If the replacement operation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let div = Dom::create_element("div")?;
    /// let old_text = Dom::create_text_node("Old")?;
    /// let new_text = Dom::create_text_node("New")?;
    ///
    /// Dom::append_child(&div, &old_text)?;
    /// let replaced = Dom::replace_child(&div, &new_text, &old_text)?;
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn replace_child(
        parent: &Node,
        new_child: &Node,
        old_child: &Node,
    ) -> Result<Node, JsValue> {
        parent.replace_child(new_child, old_child)
    }

    /// Create a text node and return it as a generic Node.
    ///
    /// This is a convenience method that creates a text node and converts it
    /// to the more general `Node` type, which is useful when working with
    /// mixed collections of elements and text nodes.
    ///
    /// # Arguments
    ///
    /// * `content` - The text content for the node
    ///
    /// # Returns
    ///
    /// * `Ok(Node)` - The newly created text node as a generic Node
    /// * `Err(JsValue)` - If text node creation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// let text_node = Dom::create_text("Hello, world!")?;
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// // text_node is now a Node that can be used with other DOM methods
    /// ```
    pub fn create_text(content: &str) -> Result<Node, JsValue> {
        Ok(dom::create_text_node(content)?.into())
    }

    /// Get an element by its ID attribute.
    ///
    /// This method searches the document for an element with the specified ID
    /// attribute and returns it if found. This is a common operation for locating
    /// specific elements in the DOM.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID attribute value to search for
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Element))` - The element with the matching ID
    /// * `Ok(None)` - If no element with the ID exists
    /// * `Err(JsValue)` - If the DOM is unavailable
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use quillai_parchment::dom::Dom;
    ///
    /// match Dom::get_element_by_id("editor")? {
    ///     Some(element) => {
    ///         // Element found, use it
    ///     }
    ///     None => {
    ///         // Element not found
    ///     }
    /// }
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn get_element_by_id(id: &str) -> Result<Option<Element>, JsValue> {
        dom::get_element_by_id(id)
    }
}
