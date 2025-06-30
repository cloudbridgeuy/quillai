//! Shadow blot base implementation for all blot types
//!
//! ShadowBlot provides the fundamental base implementation that all blots inherit from.
//! It handles basic DOM operations, registry management, and provides default
//! implementations for core blot functionality. This is the foundation of the
//! entire blot hierarchy.
//!
//! ## Purpose
//!
//! - **Base Implementation**: Provides common functionality for all blots
//! - **DOM Management**: Handles basic DOM node operations and lifecycle
//! - **Registry Integration**: Manages registration and cleanup
//! - **Default Behavior**: Provides sensible defaults for blot operations
//!
//! ## Inheritance Hierarchy
//!
//! ```text
//! ShadowBlot (base)
//! ├── ParentBlot (containers)
//! │   ├── BlockBlot (paragraphs, headers)
//! │   ├── InlineBlot (formatting)
//! │   └── ScrollBlot (root container)
//! └── LeafBlot implementations
//!     ├── TextBlot (text content)
//!     └── EmbedBlot (media, widgets)
//! ```
//!
//! ## Usage
//!
//! ShadowBlot is typically not used directly but serves as the base for
//! all other blot implementations. It provides the common interface and
//! default behaviors that specialized blots can override.

use crate::blot::traits_simple::{create_element_with_class, BlotTrait};
use crate::registry::Registry;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::Node;

/// Base implementation for all blots in the Parchment system
///
/// ShadowBlot serves as the foundational class for all blot types, providing
/// common DOM operations, registry management, and default implementations
/// for the BlotTrait interface. All other blots inherit from this base.
///
/// # Characteristics
///
/// - **Abstract Base**: Provides foundation for all blot implementations
/// - **DOM Wrapper**: Wraps and manages a single DOM node
/// - **Registry Aware**: Handles registration and cleanup automatically
/// - **Lifecycle Management**: Provides attach/detach/remove operations
///
/// # Default Behavior
///
/// - Length of 1 (suitable for atomic blots)
/// - Block scope (can be overridden by subclasses)
/// - Basic text content operations
/// - Automatic registry cleanup on drop
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::blot::{BlotTrait, ShadowBlot};
/// use quillai_parchment::dom::Dom;
///
/// // Create a basic blot (typically done by subclasses)
/// let dom_node = Dom::create_element("div")?;
/// let blot = ShadowBlot::new(dom_node.into())?;
///
/// // Basic operations
/// assert_eq!(blot.length(), 1);
/// assert_eq!(blot.get_blot_name(), "abstract");
///
/// # Ok::<(), wasm_bindgen::JsValue>(())
/// ```
pub struct ShadowBlot {
    /// The underlying DOM node that this blot represents
    pub dom_node: Node,
}

impl ShadowBlot {
    /// Create a new ShadowBlot wrapping the given DOM node
    ///
    /// Initializes a new base blot that wraps the provided DOM node.
    /// This is the fundamental constructor used by all blot types.
    ///
    /// # Parameters
    /// * `dom_node` - DOM node to wrap and manage
    ///
    /// # Returns
    /// New ShadowBlot instance on success, JsValue error on failure
    ///
    /// # Note
    /// Registry registration is handled by the caller to avoid circular
    /// dependencies during blot construction.
    pub fn new(dom_node: Node) -> Result<Self, JsValue> {
        let blot = ShadowBlot { dom_node };

        // Registry registration is deferred to the caller to allow
        // proper initialization of the blot hierarchy

        Ok(blot)
    }

    /// Create a DOM node with specified properties for blot use
    ///
    /// Static factory method that creates a properly configured DOM element
    /// for use as a blot's underlying node. Handles tag creation, CSS classes,
    /// and initial content.
    ///
    /// # Parameters
    /// * `tag_name` - HTML tag name (e.g., "div", "span", "p")
    /// * `class_name` - Optional CSS class name to apply
    /// * `value` - Optional initial text content
    ///
    /// # Returns
    /// Configured DOM node on success, JsValue error on creation failure
    ///
    /// # Errors
    /// Returns error if tag_name is empty or DOM creation fails
    ///
    /// # Examples
    /// ```rust,no_run
    /// use quillai_parchment::blot::{BlotTrait, ShadowBlot};
    ///
    /// let node = ShadowBlot::create_dom_node("div", Some("blot"), None)?;
    /// let blot = ShadowBlot::new(node)?;
    ///
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn create_dom_node(
        tag_name: &str,
        class_name: Option<&str>,
        value: Option<&JsValue>,
    ) -> Result<Node, JsValue> {
        if tag_name.is_empty() {
            return Err("Blot definition missing tagName".into());
        }

        let element = create_element_with_class(tag_name, class_name)?;

        // Set initial text content if provided
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
