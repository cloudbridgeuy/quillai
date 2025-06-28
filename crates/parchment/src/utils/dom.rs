//! DOM utility functions for WebAssembly browser environments.
//!
//! This module provides convenient wrapper functions around the `web_sys` DOM APIs,
//! offering simplified interfaces for common DOM operations in WebAssembly contexts.
//! These utilities are designed to reduce boilerplate and provide consistent error
//! handling across Parchment's DOM interactions.
//!
//! # Core Functionality
//!
//! - **Browser object access**: Safe access to window and document objects
//! - **Element creation**: Simplified element and text node creation
//! - **Element lookup**: Convenient element selection by ID
//! - **Error handling**: Consistent error propagation for DOM operations
//!
//! # Usage Examples
//!
//! ## Basic DOM Operations
//!
//! ```rust
//! use parchment::utils::dom::*;
//!
//! // Get browser objects
//! let window = window()?;
//! let document = document()?;
//!
//! // Create elements
//! let div = create_element("div")?;
//! let text = create_text_node("Hello, world!")?;
//!
//! // Find existing elements
//! let existing = get_element_by_id("my-element")?;
//! ```
//!
//! ## Error Handling
//!
//! All functions return `Result` types with appropriate error messages:
//!
//! ```rust
//! use parchment::utils::dom::*;
//!
//! match window() {
//!     Ok(win) => {
//!         // Use window object
//!     }
//!     Err(e) => {
//!         // Handle error (e.g., not in browser environment)
//!     }
//! }
//! ```
//!
//! # WebAssembly Integration
//!
//! These utilities are specifically designed for WebAssembly environments and
//! provide the DOM access needed for rich text editing and document manipulation
//! in web browsers.

use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Window};

/// Get the browser's global window object.
///
/// This function provides safe access to the browser's global `window` object,
/// which is the entry point for most DOM operations in web environments.
///
/// # Returns
///
/// * `Ok(Window)` - The global window object
/// * `Err(JsValue)` - If no window object exists (e.g., in Node.js or server environments)
///
/// # Examples
///
/// ```rust
/// use parchment::utils::dom::window;
///
/// let window = window()?;
/// let location = window.location();
/// ```
///
/// # Errors
///
/// This function will return an error if called in an environment where no
/// global `window` object exists, such as Node.js or server-side contexts.
pub fn window() -> Result<Window, JsValue> {
    web_sys::window().ok_or_else(|| JsValue::from_str("no global `window` exists"))
}

/// Get the browser's document object.
///
/// This function provides safe access to the browser's `document` object,
/// which is used for creating and manipulating DOM elements.
///
/// # Returns
///
/// * `Ok(Document)` - The document object
/// * `Err(JsValue)` - If no document exists or window is unavailable
///
/// # Examples
///
/// ```rust
/// use parchment::utils::dom::document;
///
/// let doc = document()?;
/// let title = doc.title();
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - No global `window` object exists
/// - The window object has no associated document
pub fn document() -> Result<Document, JsValue> {
    window()?
        .document()
        .ok_or_else(|| JsValue::from_str("should have a document on window"))
}

/// Create a new DOM element with the specified tag name.
///
/// This function creates a new HTML element of the given type. The element
/// is created but not yet attached to the document tree.
///
/// # Arguments
///
/// * `tag_name` - The HTML tag name (e.g., "div", "p", "span", "strong")
///
/// # Returns
///
/// * `Ok(Element)` - The newly created element
/// * `Err(JsValue)` - If element creation fails or DOM is unavailable
///
/// # Examples
///
/// ```rust
/// use parchment::utils::dom::create_element;
///
/// let div = create_element("div")?;
/// let paragraph = create_element("p")?;
/// let span = create_element("span")?;
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - The DOM is not available (no window/document)
/// - The tag name is invalid or not supported
pub fn create_element(tag_name: &str) -> Result<Element, JsValue> {
    document()?.create_element(tag_name)
}

/// Create a text node with the specified content.
///
/// This function creates a new text node containing the given string content.
/// Text nodes represent the actual text content within HTML elements and are
/// fundamental building blocks for rich text editing.
///
/// # Arguments
///
/// * `content` - The text content for the node
///
/// # Returns
///
/// * `Ok(Text)` - The newly created text node
/// * `Err(JsValue)` - If text node creation fails or DOM is unavailable
///
/// # Examples
///
/// ```rust
/// use parchment::utils::dom::create_text_node;
///
/// let text = create_text_node("Hello, world!")?;
/// let empty_text = create_text_node("")?;
/// let unicode_text = create_text_node("🦀 Rust + WASM")?;
/// ```
///
/// # Notes
///
/// Text nodes can contain any valid Unicode string, including empty strings.
/// The content is not HTML-escaped, so it represents literal text content.
pub fn create_text_node(content: &str) -> Result<web_sys::Text, JsValue> {
    Ok(document()?.create_text_node(content))
}

/// Get an element by its ID attribute.
///
/// This function searches the document for an element with the specified ID
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
/// ```rust
/// use parchment::utils::dom::get_element_by_id;
///
/// // Look for an element with id="editor"
/// match get_element_by_id("editor")? {
///     Some(element) => {
///         // Element found, use it
///         element.set_inner_html("<p>Content loaded</p>");
///     }
///     None => {
///         // Element not found
///         console::log_1(&"Editor element not found".into());
///     }
/// }
/// ```
///
/// # Notes
///
/// - ID attributes should be unique within a document
/// - The search is case-sensitive
/// - Returns the first matching element if multiple elements have the same ID
pub fn get_element_by_id(id: &str) -> Result<Option<Element>, JsValue> {
    Ok(document()?.get_element_by_id(id))
}
