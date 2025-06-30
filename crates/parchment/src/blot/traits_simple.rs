//! Core trait definitions for the Parchment blot system
//!
//! This module defines the fundamental traits that all blots must implement,
//! providing the interface for document content manipulation, tree navigation,
//! and DOM interaction. The trait system is designed for dynamic dispatch
//! compatibility while maintaining type safety.
//!
//! ## Trait Hierarchy
//!
//! - [`BlotTrait`]: Base trait for all blots with core functionality
//! - [`ParentBlotTrait`]: Extension for blots that can contain children
//! - [`LeafBlotTrait`]: Extension for terminal/leaf blots
//! - [`BlotConstructor`]: Static methods for blot creation and metadata
//!
//! ## Design Principles
//!
//! - **Dynamic Compatibility**: All traits support `dyn` usage for runtime polymorphism
//! - **DOM Integration**: Direct integration with WebAssembly DOM APIs
//! - **Type Safety**: Rust's type system prevents common document model errors
//! - **Performance**: Minimal overhead for common operations

use crate::collection::linked_list::LinkedList;
use crate::scope::Scope;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, Node};

/// Base trait for all blots in the Parchment document model
///
/// BlotTrait defines the core interface that all document content nodes must implement.
/// It provides methods for DOM interaction, content manipulation, lifecycle management,
/// and metadata access. The trait is designed for dynamic dispatch compatibility.
///
/// # Core Responsibilities
///
/// - **Metadata**: Provide blot type information (name, tag, scope)
/// - **DOM Binding**: Maintain connection to underlying DOM node
/// - **Content Operations**: Support text insertion, deletion, and length queries
/// - **Lifecycle**: Handle attachment, detachment, and removal from document
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::blot::{BlotTrait};
/// use quillai_parchment::scope::Scope;
///
/// // All blots implement this trait
/// fn process_blot(blot: &dyn BlotTrait) {
///     println!("Blot: {} ({})", blot.get_blot_name(), blot.length());
///
///     if blot.get_scope().matches(Scope::Block) {
///         println!("This is a block-level blot");
///     }
/// }
/// ```
pub trait BlotTrait {
    // === Metadata Methods ===

    /// Get the blot type name (e.g., "text", "paragraph", "bold")
    fn get_blot_name(&self) -> &'static str;

    /// Get the corresponding DOM tag name (e.g., "span", "p", "strong")
    fn get_tag_name(&self) -> &'static str;

    /// Get the scope classification for this blot type
    fn get_scope(&self) -> Scope;

    /// Get the CSS class name for this blot type, if any
    fn get_class_name(&self) -> Option<&'static str> {
        None
    }

    // === Core Interface ===

    /// Get reference to the underlying DOM node
    fn dom_node(&self) -> &Node;

    /// Get the content length of this blot (characters for text, 1 for embeds)
    fn length(&self) -> usize;

    // === Lifecycle Management ===

    /// Attach this blot to the document (called when inserted into tree)
    fn attach(&mut self);

    /// Detach this blot from the document (called when removed from tree)
    fn detach(&mut self);

    /// Remove this blot and clean up resources
    fn remove(&mut self);

    // === Content Editing ===

    /// Delete content at the specified index and length
    ///
    /// # Parameters
    /// * `index` - Starting position for deletion
    /// * `length` - Number of characters/units to delete
    fn delete_at(&mut self, index: usize, length: usize);

    /// Insert content at the specified index
    ///
    /// # Parameters
    /// * `index` - Position to insert content
    /// * `value` - Content to insert
    fn insert_at(&mut self, index: usize, value: &str);

    // === Runtime Type Support ===

    /// Support for downcasting to concrete types (needed for tree navigation)
    fn as_any(&self) -> &dyn std::any::Any;

    /// Build children from existing DOM structure (for parent blots)
    ///
    /// Default implementation does nothing. Parent blots override this to
    /// recursively build their child blot tree from existing DOM content.
    ///
    /// # Returns
    /// `Ok(())` on success, `Err(JsValue)` on DOM processing errors
    fn build_children(&mut self) -> Result<(), JsValue> {
        Ok(())
    }
}

/// Trait for blots that can contain child blots (containers)
///
/// ParentBlotTrait extends BlotTrait with functionality for managing child blots
/// in a hierarchical document structure. It provides tree navigation, child
/// manipulation, and content aggregation methods.
///
/// # Key Features
///
/// - **Child Management**: Add, remove, and reorder child blots
/// - **Tree Navigation**: Find descendants and compute paths
/// - **Content Aggregation**: Combine child content into text representation
/// - **DOM Synchronization**: Keep blot tree synchronized with DOM structure
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::blot::ParentBlotTrait;
///
/// fn analyze_container(parent: &dyn ParentBlotTrait) {
///     println!("Container has {} children", parent.children_count());
///     println!("Text content: {}", parent.text_content());
///
///     if parent.is_empty() {
///         println!("Container is empty");
///     }
/// }
/// ```
pub trait ParentBlotTrait: BlotTrait {
    /// Children management
    fn children(&self) -> &LinkedList<Box<dyn BlotTrait>>;
    fn children_mut(&mut self) -> &mut LinkedList<Box<dyn BlotTrait>>;
    fn children_count(&self) -> usize {
        self.children().length as usize
    }
    fn is_empty(&self) -> bool {
        self.children().length == 0
    }

    /// DOM element access
    fn dom_element(&self) -> &HtmlElement;

    /// Child operations - mirrors TypeScript ParentBlot API
    fn append_child(&mut self, child: Box<dyn BlotTrait>) -> Result<(), JsValue>;
    fn insert_before(
        &mut self,
        child: Box<dyn BlotTrait>,
        ref_blot: Option<&dyn BlotTrait>,
    ) -> Result<(), JsValue>;
    fn remove_child(&mut self, child: &dyn BlotTrait) -> Result<Box<dyn BlotTrait>, JsValue>;

    /// Tree navigation methods
    fn descendant(
        &self,
        matcher: fn(&dyn BlotTrait) -> bool,
        index: Option<usize>,
    ) -> Option<&dyn BlotTrait>;
    fn descendants(
        &self,
        matcher: fn(&dyn BlotTrait) -> bool,
        index: Option<usize>,
        length: Option<usize>,
    ) -> Vec<&dyn BlotTrait>;
    fn path(&self, index: usize) -> Vec<(&dyn BlotTrait, usize)>;

    /// Basic child operations (convenience methods)
    fn append_text(&mut self, text: &str) -> Result<(), JsValue>;
    fn clear(&mut self);
    fn text_content(&self) -> String;
}

/// Trait for leaf blots that contain actual content (terminal nodes)
///
/// LeafBlotTrait extends BlotTrait for blots that represent actual content
/// rather than containers. These are typically text nodes, embeds, or other
/// self-contained content elements.
///
/// # Characteristics
///
/// - **Terminal Nodes**: Cannot contain child blots
/// - **Content Storage**: Hold actual document content (text, embed data)
/// - **Value Operations**: Direct content manipulation methods
///
/// # Examples
///
/// ```rust
/// use quillai_parchment::blot::LeafBlotTrait;
///
/// fn update_content(leaf: &mut dyn LeafBlotTrait, new_text: &str) {
///     let old_value = leaf.value();
///     leaf.set_value(new_text);
///     println!("Updated: '{}' -> '{}'", old_value, new_text);
/// }
/// ```
pub trait LeafBlotTrait: BlotTrait {
    /// Get the current content value of this leaf blot
    ///
    /// # Returns
    /// String representation of the blot's content
    fn value(&self) -> String;

    /// Set the content value of this leaf blot
    ///
    /// # Parameters
    /// * `value` - New content to set
    fn set_value(&mut self, value: &str);
}

/// Context object for optimization operations during document updates
///
/// OptimizeContext carries state and configuration data during the optimization
/// phase of document updates, allowing blots to coordinate optimization decisions.
#[derive(Default)]
pub struct OptimizeContext {
    /// Arbitrary data storage for optimization state
    pub data: HashMap<String, JsValue>,
}

/// Context object for update operations during document modifications
///
/// UpdateContext carries state and configuration data during document update
/// cycles, enabling blots to coordinate changes and maintain consistency.
#[derive(Default)]
pub struct UpdateContext {
    /// Arbitrary data storage for update state
    pub data: HashMap<String, JsValue>,
}

/// Complete definition of a blot type for registry storage
///
/// RegistryDefinition contains all metadata needed to identify, create,
/// and categorize a blot type within the Parchment system.
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::blot::RegistryDefinition;
/// use quillai_parchment::scope::Scope;
///
/// let definition = RegistryDefinition {
///     blot_name: "paragraph".to_string(),
///     tag_name: "p".to_string(),
///     scope: Scope::BlockBlot,
///     class_name: Some("parchment-paragraph".to_string()),
/// };
/// ```
#[derive(Clone)]
pub struct RegistryDefinition {
    /// The unique name of this blot type
    pub blot_name: String,
    /// The DOM tag name this blot corresponds to
    pub tag_name: String,
    /// The scope classification for this blot
    pub scope: Scope,
    /// Optional CSS class name for styling
    pub class_name: Option<String>,
}

/// Trait for static blot construction and metadata methods
///
/// BlotConstructor provides the static interface for blot types, including
/// DOM node creation, metadata access, and value extraction. This trait
/// complements the instance-based BlotTrait interface.
///
/// # Static Methods
///
/// Unlike BlotTrait which operates on instances, BlotConstructor provides
/// type-level operations that don't require a blot instance.
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::blot::BlotConstructor;
/// use quillai_parchment::scope::Scope;
///
/// // Example implementation for a paragraph blot
/// struct ParagraphBlot;
///
/// impl BlotConstructor for ParagraphBlot {
///     fn create(value: Option<&wasm_bindgen::JsValue>) -> Result<web_sys::Node, wasm_bindgen::JsValue> {
///         // Create <p> element
///         todo!()
///     }
///
///     fn value_from_dom(dom_node: &web_sys::Node) -> wasm_bindgen::JsValue { todo!() }
///     fn blot_name() -> &'static str { "paragraph" }
///     fn tag_name() -> &'static str { "p" }
///     fn scope() -> Scope { Scope::BlockBlot }
/// }
/// ```
pub trait BlotConstructor {
    /// Create a new DOM node for this blot type
    ///
    /// # Parameters
    /// * `value` - Optional initial value for the blot
    ///
    /// # Returns
    /// New DOM node on success, JsValue error on failure
    fn create(value: Option<&JsValue>) -> Result<Node, JsValue>;

    // === Static Metadata ===

    /// Get the blot type name
    fn blot_name() -> &'static str;

    /// Get the DOM tag name for this blot type
    fn tag_name() -> &'static str;

    /// Get the scope classification for this blot type
    fn scope() -> Scope;

    /// Get the CSS class name for this blot type, if any
    fn class_name() -> Option<&'static str> {
        None
    }

    /// Extract value from an existing DOM node
    ///
    /// # Parameters
    /// * `dom_node` - DOM node to extract value from
    ///
    /// # Returns
    /// Extracted value as JsValue
    fn value_from_dom(dom_node: &Node) -> JsValue;
}

/// Helper function to create DOM elements with optional CSS class names
///
/// Utility function for creating properly configured DOM elements for blots.
/// Handles the common pattern of creating an element and optionally setting
/// its CSS class name.
///
/// # Parameters
/// * `tag_name` - HTML tag name to create (e.g., "p", "span", "div")
/// * `class_name` - Optional CSS class name to apply
///
/// # Returns
/// Configured HtmlElement on success, JsValue error on failure
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::blot::{create_element_with_class};
///
/// // Create a paragraph with a class
/// let p_element = create_element_with_class("p", Some("parchment-paragraph"))?;
///
/// // Create a span without a class
/// let span_element = create_element_with_class("span", None)?;
///
/// # Ok::<(), wasm_bindgen::JsValue>(())
/// ```
pub fn create_element_with_class(
    tag_name: &str,
    class_name: Option<&str>,
) -> Result<HtmlElement, JsValue> {
    let window = web_sys::window().ok_or("No global window")?;
    let document = window.document().ok_or("No document")?;
    let element = document
        .create_element(tag_name)?
        .dyn_into::<HtmlElement>()
        .map_err(|_| "Failed to cast to HtmlElement")?;

    if let Some(class) = class_name {
        element.set_class_name(class);
    }

    Ok(element)
}
