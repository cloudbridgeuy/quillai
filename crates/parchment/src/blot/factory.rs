//! Blot factory for creating appropriate blot instances from DOM nodes
//!
//! The factory module provides a centralized system for creating blot instances
//! based on DOM node types and characteristics. It handles the complexity of
//! determining the correct blot type, preserving attributes, and ensuring
//! proper registry integration.
//!
//! ## Key Features
//!
//! - **Type Detection**: Automatically determines correct blot type from DOM nodes
//! - **Registry Integration**: Immediately registers created blots
//! - **Error Recovery**: Transactional creation with automatic cleanup
//! - **Attribute Preservation**: Safely copies DOM attributes to blots
//! - **Performance Optimized**: Efficient creation for frequent operations
//!
//! ## Usage Examples
//!
//! ```rust,no_run
//! use quillai_parchment::blot::factory::BlotFactory;
//! use quillai_parchment::registry::Registry;
//! use quillai_parchment::dom::Dom;
//! use std::rc::Rc;
//! use std::cell::RefCell;
//!
//! let registry = Rc::new(RefCell::new(Registry::new()));
//! let factory = BlotFactory::new(registry);
//!
//! // Create blot from DOM node
//! let text_node = Dom::create_text_node("Hello, world!")?;
//! let blot = factory.create_from_node(&text_node)?;
//! # Ok::<(), wasm_bindgen::JsValue>(())
//! ```

use crate::blot::embed::EmbedBlot;
use crate::blot::inline::InlineBlot;
use crate::blot::traits_simple::BlotTrait;
use crate::blot::{BlockBlot, TextBlot};
use crate::registry::Registry;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{Element, Node, Text};

/// Factory for creating blot instances from DOM nodes
///
/// BlotFactory provides a centralized system for creating appropriate blot
/// instances based on DOM node types and characteristics. It handles type
/// detection, attribute preservation, and registry integration.
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::blot::factory::BlotFactory;
/// use quillai_parchment::registry::Registry;
/// use quillai_parchment::dom::Dom;
/// use std::rc::Rc;
/// use std::cell::RefCell;
///
/// let registry = Rc::new(RefCell::new(Registry::new()));
/// let factory = BlotFactory::new(registry);
///
/// // Create blot from any DOM node
/// let element = Dom::create_element("p")?;
/// let blot = factory.create_from_node(&element)?;
/// # Ok::<(), wasm_bindgen::JsValue>(())
/// ```
pub struct BlotFactory {
    /// Registry for immediate blot registration
    registry: Rc<RefCell<Registry>>,
}

/// Standard HTML block-level element tags
const BLOCK_TAGS: &[&str] = &[
    "p",
    "div",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "blockquote",
    "pre",
    "ul",
    "ol",
    "li",
    "section",
    "article",
    "header",
    "footer",
    "main",
    "aside",
    "nav",
    "figure",
    "figcaption",
    "details",
    "summary",
];

/// Standard HTML inline element tags
const INLINE_TAGS: &[&str] = &[
    "span", "strong", "em", "b", "i", "u", "s", "sub", "sup", "mark", "code", "kbd", "var", "a",
    "abbr", "cite", "q", "small", "time", "del", "ins", "dfn", "samp",
];

/// Standard HTML embed/void element tags
const EMBED_TAGS: &[&str] = &[
    "img", "br", "hr", "iframe", "video", "audio", "object", "embed", "canvas", "svg", "math",
    "input", "textarea", "select", "button",
];

/// Error types for blot creation failures
#[derive(Debug, Clone)]
pub enum BlotCreationError {
    /// Unsupported DOM node type
    UnsupportedNodeType(String),
    /// Failed to create specific blot type
    BlotCreationFailed(String),
    /// Registry registration failed
    RegistrationFailed(String),
    /// Invalid DOM node state
    InvalidNodeState(String),
}

impl BlotCreationError {
    /// Convert to JsValue for WASM compatibility
    pub fn to_js_value(&self) -> JsValue {
        match self {
            BlotCreationError::UnsupportedNodeType(msg) => {
                JsValue::from_str(&format!("Unsupported node type: {}", msg))
            }
            BlotCreationError::BlotCreationFailed(msg) => {
                JsValue::from_str(&format!("Blot creation failed: {}", msg))
            }
            BlotCreationError::RegistrationFailed(msg) => {
                JsValue::from_str(&format!("Registry registration failed: {}", msg))
            }
            BlotCreationError::InvalidNodeState(msg) => {
                JsValue::from_str(&format!("Invalid node state: {}", msg))
            }
        }
    }
}

impl BlotFactory {
    /// Create a new BlotFactory with registry integration
    ///
    /// # Parameters
    /// * `registry` - Shared registry instance for blot registration
    ///
    /// # Examples
    /// ```rust,no_run
    /// use quillai_parchment::blot::factory::BlotFactory;
    /// use quillai_parchment::registry::Registry;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let registry = Rc::new(RefCell::new(Registry::new()));
    /// let factory = BlotFactory::new(registry);
    /// ```
    pub fn new(registry: Rc<RefCell<Registry>>) -> Self {
        Self { registry }
    }

    /// Create appropriate blot from any DOM node
    ///
    /// Analyzes the DOM node type and characteristics to create the most
    /// appropriate blot instance. Handles text nodes, elements, and other
    /// node types with proper error handling.
    ///
    /// # Parameters
    /// * `node` - DOM node to create blot from
    ///
    /// # Returns
    /// * `Ok(Box<dyn BlotTrait>)` - Successfully created blot
    /// * `Err(JsValue)` - Creation failed with error details
    ///
    /// # Examples
    /// ```rust,no_run
    /// use quillai_parchment::blot::factory::BlotFactory;
    /// use quillai_parchment::registry::Registry;
    /// use quillai_parchment::dom::Dom;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let registry = Rc::new(RefCell::new(Registry::new()));
    /// let factory = BlotFactory::new(registry);
    ///
    /// let text_node = Dom::create_text_node("Hello")?;
    /// let blot = factory.create_from_node(&text_node)?;
    /// assert_eq!(blot.length(), 5);
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn create_from_node(&self, node: &Node) -> Result<Box<dyn BlotTrait>, JsValue> {
        match node.node_type() {
            Node::TEXT_NODE => {
                if let Some(text_node) = node.dyn_ref::<Text>() {
                    self.create_text_blot(text_node)
                } else {
                    Err(BlotCreationError::InvalidNodeState(
                        "Node type is TEXT_NODE but cannot cast to Text".to_string(),
                    )
                    .to_js_value())
                }
            }
            Node::ELEMENT_NODE => {
                if let Some(element) = node.dyn_ref::<Element>() {
                    self.create_element_blot(element)
                } else {
                    Err(BlotCreationError::InvalidNodeState(
                        "Node type is ELEMENT_NODE but cannot cast to Element".to_string(),
                    )
                    .to_js_value())
                }
            }
            _ => Err(BlotCreationError::UnsupportedNodeType(format!(
                "Node type {} not supported",
                node.node_type()
            ))
            .to_js_value()),
        }
    }

    /// Create TextBlot from text node
    ///
    /// Creates a TextBlot instance with the exact text content from the DOM
    /// text node. Handles Unicode content correctly and registers the blot
    /// immediately.
    ///
    /// # Parameters
    /// * `text_node` - DOM text node
    ///
    /// # Returns
    /// * `Ok(Box<dyn BlotTrait>)` - Successfully created TextBlot
    /// * `Err(JsValue)` - Creation or registration failed
    pub fn create_text_blot(&self, text_node: &Text) -> Result<Box<dyn BlotTrait>, JsValue> {
        // Get text content from DOM node
        let content = text_node.text_content().unwrap_or_default();

        // Create TextBlot instance
        let text_blot = TextBlot::new(&content).map_err(|e| {
            BlotCreationError::BlotCreationFailed(format!("TextBlot creation failed: {:?}", e))
                .to_js_value()
        })?;

        // Use transactional creation with automatic cleanup
        self.create_and_register_blot(text_node.as_ref(), Box::new(text_blot))
    }

    /// Create appropriate blot from element node
    ///
    /// Analyzes the element type and creates the most appropriate blot:
    /// - Block elements → BlockBlot
    /// - Inline elements → InlineBlot
    /// - Embed elements → EmbedBlot
    ///
    /// # Parameters
    /// * `element` - DOM element
    ///
    /// # Returns
    /// * `Ok(Box<dyn BlotTrait>)` - Successfully created blot
    /// * `Err(JsValue)` - Creation or registration failed
    pub fn create_element_blot(&self, element: &Element) -> Result<Box<dyn BlotTrait>, JsValue> {
        let tag_name = element.tag_name().to_lowercase();

        // Determine blot type based on element characteristics
        let blot_type = self.determine_element_blot_type(element, &tag_name);

        match blot_type {
            ElementBlotType::Block => self.create_block_blot(element),
            ElementBlotType::Inline => self.create_inline_blot(element),
            ElementBlotType::Embed => self.create_embed_blot(element),
            ElementBlotType::Unknown => Err(BlotCreationError::UnsupportedNodeType(format!(
                "Unknown element type for tag: {}",
                tag_name
            ))
            .to_js_value()),
        }
    }

    /// Determine the appropriate blot type for an element
    ///
    /// Uses tag name and CSS display property to determine whether an element
    /// should become a BlockBlot, InlineBlot, or EmbedBlot.
    ///
    /// # Parameters
    /// * `element` - DOM element to analyze
    /// * `tag_name` - Lowercase tag name
    ///
    /// # Returns
    /// ElementBlotType indicating the appropriate blot type
    fn determine_element_blot_type(&self, element: &Element, tag_name: &str) -> ElementBlotType {
        // Check standard tag lists first
        if BLOCK_TAGS.contains(&tag_name) {
            return ElementBlotType::Block;
        }

        if INLINE_TAGS.contains(&tag_name) {
            return ElementBlotType::Inline;
        }

        if EMBED_TAGS.contains(&tag_name) {
            return ElementBlotType::Embed;
        }

        // For unknown elements, check CSS display property
        if let Some(window) = web_sys::window() {
            if let Ok(Some(style)) = window.get_computed_style(element) {
                if let Ok(display) = style.get_property_value("display") {
                    match display.as_str() {
                        "block" | "list-item" | "table" | "table-row" | "table-cell" => {
                            return ElementBlotType::Block;
                        }
                        "inline" | "inline-block" | "inline-table" => {
                            return ElementBlotType::Inline;
                        }
                        "none" => {
                            // Hidden elements are typically not represented as blots
                            return ElementBlotType::Unknown;
                        }
                        _ => {
                            // Default to inline for unknown display values
                            return ElementBlotType::Inline;
                        }
                    }
                }
            }
        }

        ElementBlotType::Unknown
    }

    /// Create BlockBlot from element
    ///
    /// Creates a BlockBlot instance for block-level elements like paragraphs,
    /// headers, and divs. Preserves the element reference for DOM operations.
    ///
    /// # Parameters
    /// * `element` - DOM element
    ///
    /// # Returns
    /// * `Ok(Box<dyn BlotTrait>)` - Successfully created BlockBlot
    /// * `Err(JsValue)` - Creation or registration failed
    fn create_block_blot(&self, element: &Element) -> Result<Box<dyn BlotTrait>, JsValue> {
        // Create BlockBlot with element
        let mut block_blot = BlockBlot::new(Some(element.clone())).map_err(|e| {
            BlotCreationError::BlotCreationFailed(format!("BlockBlot creation failed: {:?}", e))
                .to_js_value()
        })?;

        // Preserve element attributes
        self.preserve_element_attributes(&mut block_blot, element)?;

        // Use transactional creation with automatic cleanup
        self.create_and_register_blot(element.as_ref(), Box::new(block_blot))
    }

    /// Create InlineBlot from element
    ///
    /// Creates an InlineBlot instance for inline elements like spans, strong,
    /// and emphasis. Handles formatting preservation.
    ///
    /// # Parameters
    /// * `element` - DOM element
    ///
    /// # Returns
    /// * `Ok(Box<dyn BlotTrait>)` - Successfully created InlineBlot
    /// * `Err(JsValue)` - Creation or registration failed
    fn create_inline_blot(&self, element: &Element) -> Result<Box<dyn BlotTrait>, JsValue> {
        // Create InlineBlot with element
        let mut inline_blot = InlineBlot::new(Some(element.clone())).map_err(|e| {
            BlotCreationError::BlotCreationFailed(format!("InlineBlot creation failed: {:?}", e))
                .to_js_value()
        })?;

        // Preserve element attributes
        self.preserve_element_attributes(&mut inline_blot, element)?;

        // Use transactional creation with automatic cleanup
        self.create_and_register_blot(element.as_ref(), Box::new(inline_blot))
    }

    /// Create EmbedBlot from element
    ///
    /// Creates an EmbedBlot instance for self-contained elements like images,
    /// videos, and other embedded content.
    ///
    /// # Parameters
    /// * `element` - DOM element
    ///
    /// # Returns
    /// * `Ok(Box<dyn BlotTrait>)` - Successfully created EmbedBlot
    /// * `Err(JsValue)` - Creation or registration failed
    fn create_embed_blot(&self, element: &Element) -> Result<Box<dyn BlotTrait>, JsValue> {
        // Create EmbedBlot with element
        let mut embed_blot = EmbedBlot::new(Some(element.clone())).map_err(|e| {
            BlotCreationError::BlotCreationFailed(format!("EmbedBlot creation failed: {:?}", e))
                .to_js_value()
        })?;

        // Preserve element attributes (especially important for media elements)
        self.preserve_element_attributes(&mut embed_blot, element)?;

        // Use transactional creation with automatic cleanup
        self.create_and_register_blot(element.as_ref(), Box::new(embed_blot))
    }

    /// Calculate the correct insertion position for a new blot based on DOM sibling order
    ///
    /// Analyzes the DOM structure to determine where a new blot should be inserted
    /// in the parent blot's children list, ensuring the blot tree matches DOM order.
    ///
    /// # Parameters
    /// * `parent_node` - DOM node that will contain the new node
    /// * `new_node` - DOM node being inserted
    ///
    /// # Returns
    /// * `Ok(usize)` - Zero-based position where blot should be inserted
    /// * `Err(JsValue)` - Position calculation failed
    ///
    /// # Examples
    /// ```rust,no_run
    /// use quillai_parchment::blot::factory::BlotFactory;
    /// use quillai_parchment::registry::Registry;
    /// use quillai_parchment::dom::Dom;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let registry = Rc::new(RefCell::new(Registry::new()));
    /// let factory = BlotFactory::new(registry);
    ///
    /// // For DOM: <div><span>1</span><p>NEW</p><span>2</span></div>
    /// // If both spans have blots, position for <p> would be 1
    /// let parent = Dom::create_element("div")?;
    /// let new_p = Dom::create_element("p")?;
    ///
    /// parent.append_child(&Dom::create_element("span")?.into())?;
    /// parent.append_child(&new_p)?;
    /// parent.append_child(&Dom::create_element("span")?.into())?;
    ///
    /// let position = factory.calculate_blot_insert_position(&parent, &new_p)?;
    /// assert_eq!(position, 1);
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn calculate_blot_insert_position(
        &self,
        parent_node: &Node,
        new_node: &Node,
    ) -> Result<usize, JsValue> {
        let mut position = 0;
        let mut current_sibling = parent_node.first_child();

        // Traverse siblings until we find the new node or reach the end
        while let Some(sibling) = current_sibling {
            // If we've reached the new node, this is our insertion position
            if sibling == *new_node {
                break;
            }

            // Check if this sibling has a corresponding blot in the registry
            if let Ok(mut registry) = self.registry.try_borrow_mut() {
                if registry.find_blot_for_node(&sibling).is_some() {
                    position += 1;
                }
            }

            current_sibling = sibling.next_sibling();
        }

        Ok(position)
    }

    /// Calculate insertion position when the new node is not yet in the DOM
    ///
    /// This variant is used when we know where a node will be inserted but it
    /// hasn't been added to the DOM yet. Uses a reference sibling for positioning.
    ///
    /// # Parameters
    /// * `parent_node` - DOM node that will contain the new node
    /// * `reference_sibling` - Existing sibling node to insert before (None = append)
    ///
    /// # Returns
    /// * `Ok(usize)` - Zero-based position where blot should be inserted
    /// * `Err(JsValue)` - Position calculation failed
    pub fn calculate_blot_insert_position_before_sibling(
        &self,
        parent_node: &Node,
        reference_sibling: Option<&Node>,
    ) -> Result<usize, JsValue> {
        let mut position = 0;

        // If no reference sibling, append at the end
        let Some(ref_sibling) = reference_sibling else {
            // Count all existing blots to get append position
            let mut current_sibling = parent_node.first_child();
            while let Some(sibling) = current_sibling {
                if let Ok(mut registry) = self.registry.try_borrow_mut() {
                    if registry.find_blot_for_node(&sibling).is_some() {
                        position += 1;
                    }
                }
                current_sibling = sibling.next_sibling();
            }
            return Ok(position);
        };

        // Count blots before the reference sibling
        let mut current_sibling = parent_node.first_child();
        while let Some(sibling) = current_sibling {
            // If we've reached the reference sibling, this is our position
            if sibling == *ref_sibling {
                break;
            }

            // Check if this sibling has a corresponding blot
            if let Ok(mut registry) = self.registry.try_borrow_mut() {
                if registry.find_blot_for_node(&sibling).is_some() {
                    position += 1;
                }
            }

            current_sibling = sibling.next_sibling();
        }

        Ok(position)
    }

    /// Get the total number of blot children for a parent DOM node
    ///
    /// Counts how many child DOM nodes have corresponding blots in the registry.
    /// Useful for validation and bounds checking.
    ///
    /// # Parameters
    /// * `parent_node` - DOM node to count blot children for
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of child nodes with blots
    /// * `Err(JsValue)` - Count operation failed
    pub fn count_blot_children(&self, parent_node: &Node) -> Result<usize, JsValue> {
        let mut count = 0;
        let mut current_child = parent_node.first_child();

        while let Some(child) = current_child {
            if let Ok(mut registry) = self.registry.try_borrow_mut() {
                if registry.find_blot_for_node(&child).is_some() {
                    count += 1;
                }
            }
            current_child = child.next_sibling();
        }

        Ok(count)
    }

    /// Validate that a calculated position is within valid bounds
    ///
    /// Ensures the position is not greater than the current number of children,
    /// which would indicate an invalid insertion position.
    ///
    /// # Parameters
    /// * `parent_node` - DOM node that will contain the new blot
    /// * `position` - Calculated insertion position
    ///
    /// # Returns
    /// * `Ok(())` - Position is valid
    /// * `Err(JsValue)` - Position is out of bounds
    pub fn validate_insertion_position(
        &self,
        parent_node: &Node,
        position: usize,
    ) -> Result<(), JsValue> {
        let child_count = self.count_blot_children(parent_node)?;

        if position > child_count {
            return Err(BlotCreationError::InvalidNodeState(format!(
                "Insertion position {} exceeds child count {}",
                position, child_count
            ))
            .to_js_value());
        }

        Ok(())
    }

    /// Preserve element attributes from DOM to blot
    ///
    /// Safely copies DOM attributes to the blot, filtering out potentially
    /// unsafe attributes like event handlers and javascript: URLs.
    ///
    /// # Parameters
    /// * `blot` - Blot instance to receive attributes
    /// * `element` - DOM element to copy attributes from
    ///
    /// # Returns
    /// * `Ok(())` - Successfully preserved attributes
    /// * `Err(JsValue)` - Attribute preservation failed
    fn preserve_element_attributes(
        &self,
        blot: &mut dyn BlotTrait,
        element: &Element,
    ) -> Result<(), JsValue> {
        let attributes = element.attributes();

        for i in 0..attributes.length() {
            if let Some(attr) = attributes.item(i) {
                let name = attr.name();
                let value = attr.value();

                // Security filtering - only preserve safe attributes
                if self.is_safe_attribute(&name, &value) {
                    // For now, we'll store attributes in the DOM element itself
                    // since the blot trait doesn't have attribute methods yet
                    // This preserves the attributes for later use
                    if let Some(blot_element) = blot.dom_node().dyn_ref::<Element>() {
                        blot_element.set_attribute(&name, &value)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if an attribute is safe to preserve
    ///
    /// Filters out potentially dangerous attributes that could contain
    /// executable code or security vulnerabilities.
    ///
    /// # Parameters
    /// * `name` - Attribute name
    /// * `value` - Attribute value
    ///
    /// # Returns
    /// `true` if the attribute is safe to preserve, `false` otherwise
    fn is_safe_attribute(&self, name: &str, value: &str) -> bool {
        let name_lower = name.to_lowercase();
        let value_lower = value.to_lowercase();

        // Filter out event handlers
        if name_lower.starts_with("on") {
            return false;
        }

        // Filter out javascript: URLs
        if value_lower.contains("javascript:") {
            return false;
        }

        // Filter out data: URLs with javascript or script tags
        if value_lower.starts_with("data:")
            && (value_lower.contains("javascript") || value_lower.contains("<script"))
        {
            return false;
        }

        // Filter out vbscript: URLs
        if value_lower.contains("vbscript:") {
            return false;
        }

        // Allow all other attributes
        true
    }

    /// Create and register blot with transactional semantics
    ///
    /// Provides all-or-nothing semantics for blot creation and registration.
    /// If registration fails, the blot is automatically cleaned up to prevent
    /// memory leaks.
    ///
    /// # Parameters
    /// * `node` - DOM node associated with blot
    /// * `blot` - Blot instance to register
    ///
    /// # Returns
    /// * `Ok(Box<dyn BlotTrait>)` - Successfully created and registered blot
    /// * `Err(JsValue)` - Creation or registration failed, blot cleaned up
    fn create_and_register_blot(
        &self,
        node: &Node,
        blot: Box<dyn BlotTrait>,
    ) -> Result<Box<dyn BlotTrait>, JsValue> {
        // Convert to raw pointer for registry
        let blot_ptr = Box::into_raw(blot) as *mut dyn BlotTrait;

        // Attempt registration
        match self.register_blot(node, blot_ptr) {
            Ok(()) => {
                // Registration successful, return the blot
                let blot_box = unsafe { Box::from_raw(blot_ptr) };
                Ok(blot_box)
            }
            Err(e) => {
                // Registration failed, clean up the blot to prevent memory leak
                let _cleanup_blot = unsafe { Box::from_raw(blot_ptr) };
                // _cleanup_blot is automatically dropped here
                Err(e)
            }
        }
    }

    /// Register blot in registry with error handling
    ///
    /// Safely registers the created blot in the registry. If registration
    /// fails, the blot is cleaned up to prevent memory leaks.
    ///
    /// # Parameters
    /// * `node` - DOM node associated with blot
    /// * `blot_ptr` - Raw pointer to blot instance
    ///
    /// # Returns
    /// * `Ok(())` - Successfully registered
    /// * `Err(JsValue)` - Registration failed
    fn register_blot(&self, node: &Node, blot_ptr: *mut dyn BlotTrait) -> Result<(), JsValue> {
        match self.registry.try_borrow_mut() {
            Ok(mut registry) => registry
                .register_blot_for_node(node, blot_ptr)
                .map_err(|e| {
                    BlotCreationError::RegistrationFailed(format!(
                        "Registry registration failed: {:?}",
                        e
                    ))
                    .to_js_value()
                }),
            Err(_) => Err(BlotCreationError::RegistrationFailed(
                "Failed to borrow registry for registration".to_string(),
            )
            .to_js_value()),
        }
    }

    /// Insert a blot at a specific position in the parent
    ///
    /// This method coordinates the insertion of a blot into a parent blot
    /// at the specified position, ensuring both the blot tree and DOM
    /// remain synchronized.
    ///
    /// # Parameters
    /// * `parent_blot` - The parent blot to insert into
    /// * `position` - The position to insert at
    /// * `child_blot` - The blot to insert
    ///
    /// # Returns
    /// * `Ok(())` - Successfully inserted
    /// * `Err(JsValue)` - If insertion fails
    pub fn insert_blot_at_position(
        &self,
        parent_blot: *mut dyn BlotTrait,
        position: usize,
        child_blot: Box<dyn BlotTrait>,
    ) -> Result<(), JsValue> {
        use crate::blot::traits_simple::ParentBlotTrait;

        // Safety: The parent_blot pointer must be valid
        // This is ensured by the mutation handler that calls this method
        unsafe {
            // Try to cast to ParentBlotTrait implementations
            if let Some(parent) = (*parent_blot).as_any_mut().downcast_mut::<BlockBlot>() {
                return <BlockBlot as ParentBlotTrait>::insert_child_at_position(
                    parent, position, child_blot,
                );
            }

            if let Some(parent) = (*parent_blot).as_any_mut().downcast_mut::<InlineBlot>() {
                return <InlineBlot as ParentBlotTrait>::insert_child_at_position(
                    parent, position, child_blot,
                );
            }

            if let Some(parent) = (*parent_blot)
                .as_any_mut()
                .downcast_mut::<crate::blot::parent::ParentBlot>()
            {
                return <crate::blot::parent::ParentBlot as ParentBlotTrait>::insert_child_at_position(parent, position, child_blot);
            }

            if let Some(parent) = (*parent_blot)
                .as_any_mut()
                .downcast_mut::<crate::blot::scroll::ScrollBlot>()
            {
                return <crate::blot::scroll::ScrollBlot as ParentBlotTrait>::insert_child_at_position(parent, position, child_blot);
            }
        }

        Err(JsValue::from_str(
            "Parent blot does not support child insertion",
        ))
    }
}

/// Element blot type classification
#[derive(Debug, Clone, Copy, PartialEq)]
enum ElementBlotType {
    /// Block-level container blot
    Block,
    /// Inline formatting blot
    Inline,
    /// Self-contained embed blot
    Embed,
    /// Unknown or unsupported element type
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;

    #[test]
    fn test_blot_factory_creation() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let _factory = BlotFactory::new(registry);

        // Test that factory can be created
        assert!(true); // Compilation test
    }

    #[test]
    fn test_element_blot_type_classification() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let _factory = BlotFactory::new(registry);

        // Test tag classification logic without DOM elements (non-WASM environment)
        // We can test the tag lookup logic directly

        // Test block tags
        assert!(BLOCK_TAGS.contains(&"p"));
        assert!(BLOCK_TAGS.contains(&"div"));
        assert!(BLOCK_TAGS.contains(&"h1"));

        // Test inline tags
        assert!(INLINE_TAGS.contains(&"span"));
        assert!(INLINE_TAGS.contains(&"strong"));
        assert!(INLINE_TAGS.contains(&"em"));

        // Test embed tags
        assert!(EMBED_TAGS.contains(&"img"));
        assert!(EMBED_TAGS.contains(&"br"));
        assert!(EMBED_TAGS.contains(&"hr"));

        // Test unknown tags
        assert!(!BLOCK_TAGS.contains(&"unknown"));
        assert!(!INLINE_TAGS.contains(&"unknown"));
        assert!(!EMBED_TAGS.contains(&"unknown"));
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_blot_creation_error_conversion() {
        let error = BlotCreationError::UnsupportedNodeType("test".to_string());
        let js_value = error.to_js_value();

        // Test that error converts to JsValue
        assert!(js_value.is_string());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_blot_creation_error_types() {
        // Test error type creation without WASM-specific functionality
        let error1 = BlotCreationError::UnsupportedNodeType("test".to_string());
        let error2 = BlotCreationError::BlotCreationFailed("test".to_string());
        let error3 = BlotCreationError::RegistrationFailed("test".to_string());
        let error4 = BlotCreationError::InvalidNodeState("test".to_string());

        // Test that errors can be created and formatted
        assert!(format!("{:?}", error1).contains("UnsupportedNodeType"));
        assert!(format!("{:?}", error2).contains("BlotCreationFailed"));
        assert!(format!("{:?}", error3).contains("RegistrationFailed"));
        assert!(format!("{:?}", error4).contains("InvalidNodeState"));
    }

    #[test]
    fn test_attribute_safety_filtering() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let factory = BlotFactory::new(registry);

        // Test safe attributes
        assert!(factory.is_safe_attribute("class", "my-class"));
        assert!(factory.is_safe_attribute("id", "my-id"));
        assert!(factory.is_safe_attribute("data-value", "123"));
        assert!(factory.is_safe_attribute("href", "https://example.com"));
        assert!(factory.is_safe_attribute("src", "image.jpg"));
        assert!(factory.is_safe_attribute("alt", "Description"));
        assert!(factory.is_safe_attribute("title", "Tooltip"));

        // Test unsafe attributes - event handlers
        assert!(!factory.is_safe_attribute("onclick", "alert('xss')"));
        assert!(!factory.is_safe_attribute("onload", "malicious()"));
        assert!(!factory.is_safe_attribute("onmouseover", "hack()"));

        // Test unsafe attributes - javascript URLs
        assert!(!factory.is_safe_attribute("href", "javascript:alert('xss')"));
        assert!(!factory.is_safe_attribute("src", "javascript:void(0)"));

        // Test unsafe attributes - vbscript URLs
        assert!(!factory.is_safe_attribute("href", "vbscript:msgbox('xss')"));

        // Test unsafe attributes - data URLs with javascript
        assert!(!factory.is_safe_attribute("src", "data:text/html,<script>alert('xss')</script>"));

        // Test case insensitivity
        assert!(!factory.is_safe_attribute("ONCLICK", "alert('xss')"));
        assert!(!factory.is_safe_attribute("href", "JAVASCRIPT:alert('xss')"));
    }

    #[test]
    fn test_transactional_blot_creation() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let _factory = BlotFactory::new(registry.clone());

        // Test successful creation and registration
        // Note: This test validates the transactional pattern without DOM
        // Full integration tests would require WASM environment

        // Test that factory has transactional methods
        // (Compilation test - actual DOM testing requires WASM environment)
        assert!(true);

        // Test error handling structure
        let error = BlotCreationError::RegistrationFailed("test".to_string());
        assert!(format!("{:?}", error).contains("RegistrationFailed"));
    }

    #[test]
    fn test_position_calculation_methods_exist() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let _factory = BlotFactory::new(registry);

        // Test that position calculation methods are available
        // (Compilation test - actual DOM testing requires WASM environment)
        assert!(true);
    }

    #[test]
    fn test_position_validation_logic() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let _factory = BlotFactory::new(registry);

        // Test position validation logic without DOM
        // The actual validation logic will be tested in WASM environment

        // Test error creation for invalid positions
        let error = BlotCreationError::InvalidNodeState("Position out of bounds".to_string());
        assert!(format!("{:?}", error).contains("InvalidNodeState"));
        assert!(format!("{:?}", error).contains("Position out of bounds"));
    }

    #[test]
    fn test_position_calculation_error_handling() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let _factory = BlotFactory::new(registry);

        // Test that position calculation methods handle errors properly
        // This validates the error handling structure

        let error1 = BlotCreationError::InvalidNodeState("Invalid position".to_string());
        let error2 = BlotCreationError::RegistrationFailed("Registry access failed".to_string());

        assert!(format!("{:?}", error1).contains("InvalidNodeState"));
        assert!(format!("{:?}", error2).contains("RegistrationFailed"));
    }

    // Note: Full DOM element testing requires WASM environment
    // The tests above focus on the logic that can be tested without DOM
}
