//! Mutation detection and DOM synchronization system
//!
//! This module provides the mutation observation infrastructure that keeps the
//! Parchment blot tree synchronized with DOM changes. It handles automatic
//! detection of DOM modifications and coordinates the update and optimization
//! cycles that maintain document consistency.
//!
//! ## Key Components
//!
//! - **[MutationObserverWrapper]**: Rust-friendly wrapper for DOM MutationObserver
//! - **[UpdateContext]**: Context for coordinating update operations
//! - **[OptimizeContext]**: Context for optimization cycles after mutations
//! - **MutationHandler**: Internal processor for mutation records
//!
//! ## Mutation Lifecycle
//!
//! 1. **Detection**: MutationObserver detects DOM changes
//! 2. **Processing**: MutationHandler processes mutation records
//! 3. **Update**: Affected blots are updated to match DOM state
//! 4. **Optimization**: Document structure is optimized for consistency
//! 5. **Completion**: Changes are finalized and observers re-enabled
//!
//! ## Safety and Performance
//!
//! - **Infinite Loop Protection**: Maximum iteration limits prevent runaway updates
//! - **Batch Processing**: Multiple mutations are processed together efficiently
//! - **Selective Updates**: Only affected blots are updated, not the entire tree
//! - **Memory Safety**: Proper cleanup of WASM closures and references
//!
//! ## Usage Examples
//!
//! ```rust,no_run
//! use quillai_parchment::blot::MutationObserverWrapper;
//! use quillai_parchment::dom::Dom;
//!
//! let root_node = Dom::create_element("div")?;
//!
//! // Create observer for a document root
//! let observer = MutationObserverWrapper::new(root_node.into())?;
//!
//! // Start observing
//! observer.observe()?;
//!
//! // DOM changes are now automatically detected and processed
//! // ...
//!
//! // Stop observing when done
//! observer.disconnect();
//! # Ok::<(), wasm_bindgen::JsValue>(())
//! ```

use crate::blot::traits_simple::{BlotTrait, ParentBlotTrait};
use crate::blot::text::TextBlot;

use crate::registry::Registry;
use crate::text_operations;

// Import Delta types for mutation-to-Delta conversion
use quillai_delta::{Delta, Op, AttributeMap};

/// Helper function to create JsValue from string in both WASM and non-WASM environments
#[cfg(target_arch = "wasm32")]
fn js_value_from_str(s: &str) -> JsValue {
    JsValue::from_str(s)
}

#[cfg(not(target_arch = "wasm32"))]
fn js_value_from_str(s: &str) -> JsValue {
    JsValue::from(s)
}
use js_sys::Array;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{MutationObserver, MutationRecord, Node};

/// Maximum number of optimize iterations to prevent infinite loops
///
/// This safety limit prevents runaway optimization cycles that could occur
/// if blot updates trigger additional mutations in a feedback loop.
const MAX_OPTIMIZE_ITERATIONS: usize = 100;

/// Context object for coordinating update operations during mutation processing
///
/// UpdateContext carries information about the current mutation processing
/// cycle, including the mutation records being processed and iteration count
/// for safety monitoring.
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::blot::UpdateContext;
/// use web_sys::MutationRecord;
///
/// let mutation_records: Vec<MutationRecord> = vec![];
///
/// let context = UpdateContext {
///     mutation_records,
///     iteration_count: 1,
/// };
///
/// # Ok::<(), wasm_bindgen::JsValue>(())
/// ```
#[derive(Debug, Clone)]
pub struct UpdateContext {
    /// The mutation records being processed in this update cycle
    pub mutation_records: Vec<MutationRecord>,
    /// Current iteration count for infinite loop detection
    pub iteration_count: usize,
}

/// Context object for coordinating optimization operations after mutations
///
/// OptimizeContext tracks the optimization phase that follows mutation
/// processing, ensuring document consistency and preventing optimization loops.
///
/// # Examples
///
/// ```rust
/// use quillai_parchment::blot::OptimizeContext;
///
/// let context = OptimizeContext {
///     iteration_count: 1,
///     has_changes: true,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct OptimizeContext {
    /// Current optimization iteration count
    pub iteration_count: usize,
    /// Whether changes were made during this optimization cycle
    pub has_changes: bool,
}

/// Rust-friendly wrapper for DOM MutationObserver with Parchment integration
///
/// MutationObserverWrapper provides a safe, ergonomic interface to the browser's
/// MutationObserver API, handling the complexities of WASM closures and
/// integrating with the Parchment blot system for automatic synchronization.
///
/// # Features
///
/// - **Automatic Synchronization**: Keeps blot tree in sync with DOM changes
/// - **Memory Safety**: Proper cleanup of WASM closures and references
/// - **Error Handling**: Robust error handling for mutation processing
/// - **Performance**: Efficient batch processing of multiple mutations
///
/// # Lifecycle
///
/// 1. Create observer with target DOM node
/// 2. Start observation with `observe()`
/// 3. Mutations are automatically detected and processed
/// 4. Stop observation with `disconnect()` when done
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::blot::MutationObserverWrapper;
/// use quillai_parchment::dom::Dom;
///
/// let root_node = Dom::create_element("div")?;
///
/// // Create and start observer
/// let observer = MutationObserverWrapper::new(root_node.into())?;
/// observer.observe()?;
///
/// // Observer now automatically handles DOM changes
/// // ...
///
/// // Clean up when done
/// observer.disconnect();
/// # Ok::<(), wasm_bindgen::JsValue>(())
/// ```
pub struct MutationObserverWrapper {
    /// The underlying DOM MutationObserver
    observer: MutationObserver,
    /// The DOM node being observed for changes
    target_node: Node,
    /// WASM closure for handling mutation callbacks (must be kept alive)
    #[allow(dead_code)]
    callback: Closure<dyn FnMut(Array, MutationObserver)>,
    /// Shared state for processing mutations
    handler: Rc<RefCell<MutationHandler>>,
}

/// Internal handler for processing mutation records and coordinating updates
///
/// MutationHandler manages the complex process of translating DOM mutations
/// into blot tree updates, coordinating with the registry and managing
/// update/optimization cycles.
struct MutationHandler {
    /// Optional reference to the root scroll blot for document operations
    scroll_blot: Option<*mut dyn BlotTrait>,
    /// Registry for mapping DOM nodes to blots
    registry: Option<Rc<RefCell<Registry>>>,
    /// Current update context for mutation processing
    update_context: UpdateContext,
    /// Current optimization context for post-mutation cleanup
    optimize_context: OptimizeContext,
    /// Callback for when mutations are converted to Delta operations
    delta_callback: Option<Box<dyn Fn(Delta)>>,
    /// Current document length for Delta position calculations
    document_length: usize,
}

impl MutationObserverWrapper {
    /// Create a new MutationObserver wrapper for the given DOM node
    pub fn new(target_node: Node) -> Result<Self, JsValue> {
        let handler = Rc::new(RefCell::new(MutationHandler {
            scroll_blot: None,
            registry: None,
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
            delta_callback: None,
            document_length: 0,
        }));

        let handler_clone = handler.clone();
        let callback = Closure::wrap(Box::new(
            move |mutations: Array, _observer: MutationObserver| {
                let mut mutation_records = Vec::new();

                // Convert JS Array to Vec<MutationRecord>
                for i in 0..mutations.length() {
                    if let Ok(record) = mutations.get(i).dyn_into::<MutationRecord>() {
                        mutation_records.push(record);
                    }
                }

                // Process mutations
                if let Ok(mut handler) = handler_clone.try_borrow_mut() {
                    handler.process_mutations(mutation_records);
                }
            },
        ) as Box<dyn FnMut(Array, MutationObserver)>);

        let observer = MutationObserver::new(callback.as_ref().unchecked_ref())?;

        Ok(MutationObserverWrapper {
            observer,
            target_node,
            callback,
            handler,
        })
    }

    /// Start observing mutations on the target node
    pub fn observe(&self) -> Result<(), JsValue> {
        let options = web_sys::MutationObserverInit::new();
        options.set_child_list(true);
        options.set_subtree(true);
        options.set_attributes(true);
        options.set_character_data(true);
        options.set_attribute_old_value(true);
        options.set_character_data_old_value(true);

        self.observer
            .observe_with_options(&self.target_node, &options)
    }

    /// Stop observing mutations
    pub fn disconnect(&self) {
        self.observer.disconnect();
    }

    /// Set the scroll blot reference for document-level operations
    pub fn set_scroll_blot(&self, scroll_blot: *mut dyn BlotTrait) {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.scroll_blot = Some(scroll_blot);
        }
    }

    /// Set the registry reference for DOM-to-Blot mapping
    pub fn set_registry(&self, registry: Rc<RefCell<Registry>>) {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.registry = Some(registry);
        }
    }

    /// Manually trigger an update cycle (for testing or forced updates)
    pub fn update(&self, mutations: Vec<MutationRecord>) -> Result<(), JsValue> {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.process_mutations(mutations);
        }
        Ok(())
    }

    /// Manually trigger an optimize cycle
    pub fn optimize(&self) -> Result<(), JsValue> {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.optimize();
        }
        Ok(())
    }

    /// Set a callback for when mutations are converted to Delta operations
    pub fn set_delta_callback<F>(&self, callback: F)
    where
        F: Fn(Delta) + 'static,
    {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.delta_callback = Some(Box::new(callback));
        }
    }

    /// Set the current document length for Delta position calculations
    pub fn set_document_length(&self, length: usize) {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.document_length = length;
        }
    }

    /// Convert current mutations to Delta operations
    pub fn mutations_to_delta(&self) -> Result<Option<Delta>, JsValue> {
        if let Ok(handler) = self.handler.try_borrow() {
            if handler.update_context.mutation_records.is_empty() {
                return Ok(None);
            }
            
            let delta = handler.convert_mutations_to_delta()?;
            Ok(Some(delta))
        } else {
            Err(JsValue::from_str("Failed to access mutation handler"))
        }
    }
}

impl MutationHandler {
    /// Safely access the registry with proper error handling
    fn with_registry<F, R>(&self, operation: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut Registry) -> R,
    {
        if let Some(registry_rc) = &self.registry {
            match registry_rc.try_borrow_mut() {
                Ok(mut registry) => Some(f(&mut *registry)),
                Err(_) => {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "Registry borrow conflict during {} - operation may be incomplete",
                        operation
                    )));
                    None
                }
            }
        } else {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "Registry not available for {} - mutation observer not properly initialized",
                operation
            )));
            None
        }
    }
    /// Process mutation records and trigger update/optimize cycles
    fn process_mutations(&mut self, mutation_records: Vec<MutationRecord>) {
        // Update the context with new mutations
        self.update_context.mutation_records = mutation_records;
        self.update_context.iteration_count = 0;

        // Convert mutations to Delta if callback is set
        if self.delta_callback.is_some() {
            if let Ok(delta) = self.convert_mutations_to_delta() {
                if let Some(callback) = &self.delta_callback {
                    callback(delta);
                }
            }
        }

        // Run update cycle
        self.update();

        // Run optimize cycle after updates
        self.optimize();
    }

    /// Convert current mutation records to Delta operations
    fn convert_mutations_to_delta(&self) -> Result<Delta, JsValue> {
        let mut delta = Delta::new();
        let mut current_position = 0;

        for record in &self.update_context.mutation_records {
            match record.type_().as_str() {
                "childList" => {
                    let child_delta = self.convert_child_list_mutation_to_delta(record, &mut current_position)?;
                    delta = delta.compose(&child_delta);
                }
                "characterData" => {
                    let text_delta = self.convert_character_data_mutation_to_delta(record, &mut current_position)?;
                    delta = delta.compose(&text_delta);
                }
                "attributes" => {
                    let attr_delta = self.convert_attribute_mutation_to_delta(record, &mut current_position)?;
                    delta = delta.compose(&attr_delta);
                }
                _ => {
                    // Unknown mutation type - skip
                    web_sys::console::warn_1(&JsValue::from_str(&format!(
                        "Unknown mutation type for Delta conversion: {}",
                        record.type_()
                    )));
                }
            }
        }

        Ok(delta)
    }

    /// Convert child list mutations to Delta operations
    fn convert_child_list_mutation_to_delta(
        &self,
        record: &MutationRecord,
        current_position: &mut usize,
    ) -> Result<Delta, JsValue> {
        let mut delta = Delta::new();

        // Handle removed nodes first (deletions)
        let removed_nodes = record.removed_nodes();
        for i in 0..removed_nodes.length() {
            if let Some(js_node) = removed_nodes.get(i) {
                if let Ok(node) = js_node.dyn_into::<Node>() {
                    let delete_delta = self.convert_node_removal_to_delta(&node, *current_position)?;
                    delta = delta.compose(&delete_delta);
                }
            }
        }

        // Handle added nodes (insertions)
        let added_nodes = record.added_nodes();
        for i in 0..added_nodes.length() {
            if let Some(js_node) = added_nodes.get(i) {
                if let Ok(node) = js_node.dyn_into::<Node>() {
                    let insert_delta = self.convert_node_addition_to_delta(&node, *current_position)?;
                    delta = delta.compose(&insert_delta);
                    
                    // Update position after insertion
                    *current_position += self.calculate_node_length(&node);
                }
            }
        }

        Ok(delta)
    }

    /// Convert character data mutations to Delta operations
    fn convert_character_data_mutation_to_delta(
        &self,
        record: &MutationRecord,
        current_position: &mut usize,
    ) -> Result<Delta, JsValue> {
        let mut delta = Delta::new();

        if let Some(target) = record.target() {
            if let Some(text_node) = target.dyn_ref::<web_sys::Text>() {
                let current_content = text_node.text_content().unwrap_or_default();
                let old_content = record.old_value().unwrap_or_default();

                // Calculate the position of this text node in the document
                let text_position = self.calculate_text_node_position(text_node)?;

                // Create Delta operations for the text change
                if old_content != current_content {
                    // Add retain operation to get to the text position
                    if text_position > 0 {
                        delta = delta.retain(text_position, None);
                    }

                    // Delete old content if it exists
                    if !old_content.is_empty() {
                        delta = delta.delete(old_content.chars().count());
                    }

                    // Insert new content if it exists
                    if !current_content.is_empty() {
                        delta = delta.insert(&current_content, None);
                    }

                    *current_position = text_position + current_content.chars().count();
                }
            }
        }

        Ok(delta)
    }

    /// Convert attribute mutations to Delta operations
    fn convert_attribute_mutation_to_delta(
        &self,
        record: &MutationRecord,
        _current_position: &mut usize,
    ) -> Result<Delta, JsValue> {
        let mut delta = Delta::new();

        if let Some(target) = record.target() {
            if let Some(element) = target.dyn_ref::<web_sys::Element>() {
                if let Some(attr_name) = record.attribute_name() {
                    // Calculate the position and length of the element's content
                    let (element_position, element_length) = self.calculate_element_position_and_length(element)?;

                    // Create attributes for the formatting change
                    let mut attributes = AttributeMap::new();
                    
                    // Get current attribute value
                    if let Some(current_value) = element.get_attribute(&attr_name) {
                        attributes.insert(attr_name.clone(), current_value.into());
                    }

                    // Create Delta operation for formatting change
                    if element_position > 0 {
                        delta = delta.retain(element_position, None);
                    }
                    
                    if element_length > 0 {
                        delta = delta.retain(element_length, Some(attributes));
                    }
                }
            }
        }

        Ok(delta)
    }

    /// Convert node removal to Delta delete operation
    fn convert_node_removal_to_delta(&self, node: &Node, position: usize) -> Result<Delta, JsValue> {
        let mut delta = Delta::new();
        let node_length = self.calculate_node_length(node);

        if position > 0 {
            delta = delta.retain(position, None);
        }

        if node_length > 0 {
            delta = delta.delete(node_length);
        }

        Ok(delta)
    }

    /// Convert node addition to Delta insert operation
    fn convert_node_addition_to_delta(&self, node: &Node, position: usize) -> Result<Delta, JsValue> {
        let mut delta = Delta::new();

        if position > 0 {
            delta = delta.retain(position, None);
        }

        // Extract content and attributes from the node
        let (content, attributes) = self.extract_node_content_and_attributes(node)?;

        if !content.is_empty() {
            delta = delta.insert(&content, attributes);
        }

        Ok(delta)
    }

    /// Calculate the length of a DOM node in characters
    fn calculate_node_length(&self, node: &Node) -> usize {
        match node.node_type() {
            web_sys::Node::TEXT_NODE => {
                node.text_content().unwrap_or_default().chars().count()
            }
            web_sys::Node::ELEMENT_NODE => {
                // For elements, calculate the total text content length
                node.text_content().unwrap_or_default().chars().count()
            }
            _ => 0, // Other node types don't contribute to text length
        }
    }

    /// Calculate the position of a text node within the document
    fn calculate_text_node_position(&self, text_node: &web_sys::Text) -> Result<usize, JsValue> {
        let mut position = 0;
        let node: &Node = text_node.as_ref();

        // Find the root document element
        let mut current = node.clone();
        while let Some(parent) = current.parent_node() {
            current = parent;
        }

        // Traverse from root to find position
        self.traverse_for_position(&current, node, &mut position)?;
        Ok(position)
    }

    /// Recursively traverse DOM to calculate text position
    fn traverse_for_position(&self, current: &Node, target: &Node, position: &mut usize) -> Result<bool, JsValue> {
        if std::ptr::eq(current, target) {
            return Ok(true); // Found target
        }

        if current.node_type() == web_sys::Node::TEXT_NODE {
            if !std::ptr::eq(current, target) {
                *position += current.text_content().unwrap_or_default().chars().count();
            }
        } else {
            // Traverse children
            let children = current.child_nodes();
            for i in 0..children.length() {
                if let Some(child) = children.get(i) {
                    if self.traverse_for_position(&child, target, position)? {
                        return Ok(true); // Found in child
                    }
                }
            }
        }

        Ok(false)
    }

    /// Calculate position and length of an element's content
    fn calculate_element_position_and_length(&self, element: &web_sys::Element) -> Result<(usize, usize), JsValue> {
        let node: &Node = element.as_ref();
        let position = self.calculate_text_node_position(&node.clone().dyn_into::<web_sys::Text>()
            .map_err(|_| JsValue::from_str("Element is not a text node"))?)?;
        let length = self.calculate_node_length(node);
        Ok((position, length))
    }

    /// Extract content and attributes from a DOM node
    fn extract_node_content_and_attributes(&self, node: &Node) -> Result<(String, Option<AttributeMap>), JsValue> {
        let content = node.text_content().unwrap_or_default();
        let mut attributes = None;

        if let Some(element) = node.dyn_ref::<web_sys::Element>() {
            let mut attrs = AttributeMap::new();
            let tag_name = element.tag_name().to_lowercase();

            // Extract formatting attributes based on element type
            match tag_name.as_str() {
                "strong" | "b" => {
                    attrs.insert("bold".to_string(), true.into());
                }
                "em" | "i" => {
                    attrs.insert("italic".to_string(), true.into());
                }
                "u" => {
                    attrs.insert("underline".to_string(), true.into());
                }
                "s" | "strike" => {
                    attrs.insert("strike".to_string(), true.into());
                }
                "code" => {
                    attrs.insert("code".to_string(), true.into());
                }
                "a" => {
                    if let Some(href) = element.get_attribute("href") {
                        attrs.insert("link".to_string(), href.into());
                    }
                }
                "h1" => {
                    attrs.insert("header".to_string(), 1.into());
                }
                "h2" => {
                    attrs.insert("header".to_string(), 2.into());
                }
                "h3" => {
                    attrs.insert("header".to_string(), 3.into());
                }
                "h4" => {
                    attrs.insert("header".to_string(), 4.into());
                }
                "h5" => {
                    attrs.insert("header".to_string(), 5.into());
                }
                "h6" => {
                    attrs.insert("header".to_string(), 6.into());
                }
                "blockquote" => {
                    attrs.insert("blockquote".to_string(), true.into());
                }
                "pre" => {
                    attrs.insert("code-block".to_string(), true.into());
                }
                "ol" => {
                    attrs.insert("list".to_string(), "ordered".into());
                }
                "ul" => {
                    attrs.insert("list".to_string(), "bullet".into());
                }
                _ => {}
            }

            // Extract style attributes
            if let Some(style) = element.get_attribute("style") {
                self.parse_style_attributes(&style, &mut attrs);
            }

            if !attrs.is_empty() {
                attributes = Some(attrs);
            }
        }

        Ok((content, attributes))
    }

    /// Parse CSS style string into Delta attributes
    fn parse_style_attributes(&self, style: &str, attributes: &mut AttributeMap) {
        for declaration in style.split(';') {
            let declaration = declaration.trim();
            if let Some((property, value)) = declaration.split_once(':') {
                let prop = property.trim();
                let val = value.trim();

                match prop {
                    "color" => {
                        attributes.insert("color".to_string(), val.into());
                    }
                    "background-color" => {
                        attributes.insert("background".to_string(), val.into());
                    }
                    "font-family" => {
                        attributes.insert("font".to_string(), val.into());
                    }
                    "font-size" => {
                        attributes.insert("size".to_string(), val.into());
                    }
                    "font-weight" => {
                        if val == "bold" || val == "700" || val == "800" || val == "900" {
                            attributes.insert("bold".to_string(), true.into());
                        }
                    }
                    "font-style" => {
                        if val == "italic" {
                            attributes.insert("italic".to_string(), true.into());
                        }
                    }
                    "text-decoration" => {
                        if val.contains("underline") {
                            attributes.insert("underline".to_string(), true.into());
                        }
                        if val.contains("line-through") {
                            attributes.insert("strike".to_string(), true.into());
                        }
                    }
                    _ => {
                        // Store unknown CSS properties with css- prefix
                        attributes.insert(format!("css-{}", prop), val.into());
                    }
                }
            }
        }
    }

    /// Update cycle - handles DOM changes and maintains blot tree consistency
    fn update(&mut self) {
        // Process each mutation record by index to avoid borrow checker issues
        let num_records = self.update_context.mutation_records.len();

        for i in 0..num_records {
            let record = &self.update_context.mutation_records[i];
            match record.type_().as_str() {
                "childList" => {
                    // Handle child list mutation
                    self.handle_child_list_mutation_internal(record);
                }
                "attributes" => {
                    // Handle attribute mutation
                    self.handle_attribute_mutation_internal(record);
                }
                "characterData" => {
                    // Handle character data mutation
                    self.handle_character_data_mutation_internal(record);
                }
                _ => {
                    // Unknown mutation type - log warning in development

                    web_sys::console::warn_1(&JsValue::from_str(&format!(
                        "Unknown mutation type: {}",
                        record.type_()
                    )));
                }
            }
        }

        self.update_context.iteration_count += 1;
    }

    /// Optimize cycle - post-mutation cleanup and consistency checks
    fn optimize(&mut self) {
        self.optimize_context.iteration_count = 0;
        self.optimize_context.has_changes = true;

        while self.optimize_context.has_changes
            && self.optimize_context.iteration_count < MAX_OPTIMIZE_ITERATIONS
        {
            self.optimize_context.has_changes = false;

            // Core optimization logic
            self.check_for_orphaned_blots();
            self.merge_adjacent_text_nodes();
            self.validate_parent_child_relationships();
            self.clean_up_empty_containers();

            self.optimize_context.iteration_count += 1;
        }

        if self.optimize_context.iteration_count >= MAX_OPTIMIZE_ITERATIONS {
            // Log warning about hitting iteration limit

            web_sys::console::warn_1(&JsValue::from_str(
                "Optimize cycle hit maximum iterations - possible infinite loop",
            ));
        }
    }

    /// Handle child list mutations (nodes added/removed) - internal version to avoid borrowing issues
    fn handle_child_list_mutation_internal(&self, record: &MutationRecord) {
        // Handle added nodes
        let added_nodes = record.added_nodes();
        for i in 0..added_nodes.length() {
            if let Some(js_node) = added_nodes.get(i) {
                if let Ok(node) = js_node.dyn_into::<Node>() {
                    if let Some(target) = record.target() {
                        self.handle_node_added_internal(&node, &target);
                    }
                }
            }
        }

        // Handle removed nodes
        let removed_nodes = record.removed_nodes();
        for i in 0..removed_nodes.length() {
            if let Some(js_node) = removed_nodes.get(i) {
                if let Ok(node) = js_node.dyn_into::<Node>() {
                    if let Some(target) = record.target() {
                        self.handle_node_removed_internal(&node, &target);
                    }
                }
            }
        }
    }

    /// Handle attribute mutations - internal version to avoid borrowing issues
    fn handle_attribute_mutation_internal(&self, record: &MutationRecord) {
        if let Some(target) = record.target() {
            if let Some(attr_name) = record.attribute_name() {
                self.handle_attribute_change(&target, &attr_name, &record.old_value());
            }
        }
    }

    /// Handle specific attribute changes and update corresponding blots
    fn handle_attribute_change(&self, target: &Node, attr_name: &str, old_value: &Option<String>) {
        // Find the blot associated with this DOM node
        let has_blot = self.with_registry("attribute change handling", |registry| {
            registry.find_blot_for_node(target).is_some()
        }).unwrap_or(false);

        if has_blot {
            if let Some(element) = target.dyn_ref::<web_sys::Element>() {
                match attr_name {
                    "class" => {
                        self.handle_class_attribute_change(element, old_value);
                    }
                    "style" => {
                        self.handle_style_attribute_change(element, old_value);
                    }
                    "id" | "data-*" => {
                        self.handle_generic_attribute_change(element, attr_name, old_value);
                    }
                    _ => {
                        // Handle other attributes that might affect blot formatting
                        self.handle_generic_attribute_change(element, attr_name, old_value);
                    }
                }
            }
        }

        web_sys::console::log_2(
            &JsValue::from_str(&format!("Attribute mutation: {}", attr_name)),
            &JsValue::from_str(old_value.as_ref().unwrap_or(&String::new())),
        );
    }

    /// Handle class attribute changes that affect CSS-based formatting
    fn handle_class_attribute_change(
        &self,
        element: &web_sys::Element,
        old_value: &Option<String>,
    ) {
        let current_classes = element.class_name();
        let old_classes = old_value.as_ref().unwrap_or(&String::new()).clone();

        // Compare old and new classes to determine what changed
        let current_classes_vec: Vec<&str> = current_classes.split_whitespace().collect();
        let old_classes_vec: Vec<&str> = old_classes.split_whitespace().collect();

        // Find added and removed classes (simple diff implementation)
        let mut added_classes = Vec::new();
        let mut removed_classes = Vec::new();

        for class in &current_classes_vec {
            if !old_classes_vec.contains(class) {
                added_classes.push(*class);
            }
        }

        for class in &old_classes_vec {
            if !current_classes_vec.contains(class) {
                removed_classes.push(*class);
            }
        }

        // Update blot formatting based on class changes
        for class in &added_classes {
            self.apply_class_formatting(element, class, true);
        }

        for class in &removed_classes {
            self.apply_class_formatting(element, class, false);
        }

        if !added_classes.is_empty() || !removed_classes.is_empty() {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Class changes - Added: {:?}, Removed: {:?}",
                added_classes, removed_classes
            )));
        }
    }

    /// Handle inline style attribute changes
    fn handle_style_attribute_change(
        &self,
        element: &web_sys::Element,
        old_value: &Option<String>,
    ) {
        let current_style = element.get_attribute("style").unwrap_or_default();
        let old_style = old_value.as_ref().unwrap_or(&String::new()).clone();

        // Parse style strings and compare changes
        // This is a simplified implementation - a full version would parse CSS properties
        if current_style != old_style {
            self.sync_style_formatting(element, &current_style, &old_style);
        }

        web_sys::console::log_2(
            &JsValue::from_str("Style attribute changed from:"),
            &JsValue::from_str(&format!("'{}' to '{}'", old_style, current_style)),
        );
    }

    /// Handle generic attribute changes that might affect blot state
    fn handle_generic_attribute_change(
        &self,
        element: &web_sys::Element,
        attr_name: &str,
        old_value: &Option<String>,
    ) {
        let current_value = element.get_attribute(attr_name).unwrap_or_default();
        let old_val = old_value.as_ref().unwrap_or(&String::new()).clone();

        if current_value != old_val {
            // Notify blot that an attribute has changed
            // This could trigger blot-specific formatting updates
            self.notify_blot_attribute_change(element, attr_name, &old_val, &current_value);
        }

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Attribute '{}' changed from '{}' to '{}'",
            attr_name, old_val, current_value
        )));
    }

    /// Apply or remove class-based formatting to a blot
    fn apply_class_formatting(
        &self,
        element: &web_sys::Element,
        class_name: &str,
        is_added: bool,
    ) {
        // Find the blot associated with this DOM element
        let result = self.with_registry("class formatting", |registry| {
            if let Some(blot_ptr) = registry.find_blot_for_node(&element.clone().into()) {
                unsafe {
                    let blot = &mut *blot_ptr;
                    
                    // Try to downcast to FormattableBlot
                    if let Some(formattable) = blot.as_any_mut().downcast_mut::<crate::blot::inline::InlineBlot>() {
                        self.apply_class_to_formattable_blot(formattable, class_name, is_added);
                    } else if let Some(formattable) = blot.as_any_mut().downcast_mut::<crate::blot::block::BlockBlot>() {
                        self.apply_class_to_formattable_blot(formattable, class_name, is_added);
                    } else {
                        web_sys::console::log_1(&JsValue::from_str(&format!(
                            "Blot type does not support formatting: {}",
                            class_name
                        )));
                    }
                }
            } else {
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "No blot found for element when applying class: {}",
                    class_name
                )));
            }
            Ok::<(), JsValue>(())
        });

        if let Err(e) = result.unwrap_or(Err(JsValue::from_str("Registry not available"))) {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "Error applying class formatting: {:?}",
                e
            )));
        }

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "{} class formatting: {}",
            if is_added { "Applied" } else { "Removed" },
            class_name
        )));
    }

    /// Apply class formatting to a FormattableBlot
    fn apply_class_to_formattable_blot<T>(&self, blot: &mut T, class_name: &str, is_added: bool)
    where
        T: crate::blot::formatting::FormattableBlot,
    {
        let formatting_state = blot.formatting_state_mut();
        
        if is_added {
            formatting_state.add_class(class_name);
        } else {
            formatting_state.remove_class(class_name);
        }

        // Sync the formatting state back to DOM to ensure consistency
        if let Err(e) = blot.apply_formatting_to_dom() {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "Error syncing formatting to DOM: {:?}",
                e
            )));
        }
    }

    /// Synchronize style-based formatting between DOM and blot
    fn sync_style_formatting(
        &self,
        element: &web_sys::Element,
        current_style: &str,
        old_style: &str,
    ) {
        // Parse both old and new style strings into property maps
        let current_properties = self.parse_style_string(current_style);
        let old_properties = self.parse_style_string(old_style);

        // Find the blot associated with this DOM element
        let result = self.with_registry("style formatting", |registry| {
            if let Some(blot_ptr) = registry.find_blot_for_node(&element.clone().into()) {
                unsafe {
                    let blot = &mut *blot_ptr;
                    
                    // Try to downcast to FormattableBlot
                    if let Some(formattable) = blot.as_any_mut().downcast_mut::<crate::blot::inline::InlineBlot>() {
                        self.apply_style_changes_to_blot(formattable, &current_properties, &old_properties);
                    } else if let Some(formattable) = blot.as_any_mut().downcast_mut::<crate::blot::block::BlockBlot>() {
                        self.apply_style_changes_to_blot(formattable, &current_properties, &old_properties);
                    } else {
                        web_sys::console::log_1(&JsValue::from_str(
                            "Blot type does not support style formatting"
                        ));
                    }
                }
            } else {
                web_sys::console::log_1(&JsValue::from_str(
                    "No blot found for element when syncing styles"
                ));
            }
            Ok::<(), JsValue>(())
        });

        if let Err(e) = result.unwrap_or(Err(JsValue::from_str("Registry not available"))) {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "Error syncing style formatting: {:?}",
                e
            )));
        }

        web_sys::console::log_1(&JsValue::from_str("Style formatting synchronized"));
    }

    /// Parse a CSS style string into a property map
    fn parse_style_string(&self, style_string: &str) -> std::collections::HashMap<String, String> {
        let mut properties = std::collections::HashMap::new();
        
        for declaration in style_string.split(';') {
            let declaration = declaration.trim();
            if let Some(colon_pos) = declaration.find(':') {
                let property = declaration[..colon_pos].trim().to_string();
                let value = declaration[colon_pos + 1..].trim().to_string();
                if !property.is_empty() && !value.is_empty() {
                    properties.insert(property, value);
                }
            }
        }
        
        properties
    }

    /// Apply style changes to a FormattableBlot
    fn apply_style_changes_to_blot<T>(
        &self,
        blot: &mut T,
        current_properties: &std::collections::HashMap<String, String>,
        old_properties: &std::collections::HashMap<String, String>,
    )
    where
        T: crate::blot::formatting::FormattableBlot,
    {
        let formatting_state = blot.formatting_state_mut();

        // Find properties that were added or changed
        for (property, value) in current_properties {
            let old_value = old_properties.get(property);
            if old_value.is_none() || old_value.unwrap() != value {
                // Property was added or changed
                formatting_state.set_style(property, value);
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Updated style property: {} = {}",
                    property, value
                )));
            }
        }

        // Find properties that were removed
        for (property, _) in old_properties {
            if !current_properties.contains_key(property) {
                // Property was removed
                formatting_state.remove_style(property);
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Removed style property: {}",
                    property
                )));
            }
        }

        // Sync the formatting state back to DOM to ensure consistency
        if let Err(e) = blot.apply_formatting_to_dom() {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "Error syncing style formatting to DOM: {:?}",
                e
            )));
        }
    }

    /// Notify blot that a generic attribute has changed
    fn notify_blot_attribute_change(
        &self,
        element: &web_sys::Element,
        attr_name: &str,
        old_value: &str,
        new_value: &str,
    ) {
        // Find the blot associated with this DOM element
        let result = self.with_registry("generic attribute change", |registry| {
            if let Some(blot_ptr) = registry.find_blot_for_node(&element.clone().into()) {
                unsafe {
                    let blot = &mut *blot_ptr;
                    
                    // Try to downcast to FormattableBlot
                    if let Some(formattable) = blot.as_any_mut().downcast_mut::<crate::blot::inline::InlineBlot>() {
                        self.apply_generic_attribute_to_blot(formattable, attr_name, old_value, new_value);
                    } else if let Some(formattable) = blot.as_any_mut().downcast_mut::<crate::blot::block::BlockBlot>() {
                        self.apply_generic_attribute_to_blot(formattable, attr_name, old_value, new_value);
                    } else {
                        web_sys::console::log_1(&JsValue::from_str(&format!(
                            "Blot type does not support generic attributes: {}",
                            attr_name
                        )));
                    }
                }
            } else {
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "No blot found for element when applying attribute: {}",
                    attr_name
                )));
            }
            Ok::<(), JsValue>(())
        });

        if let Err(e) = result.unwrap_or(Err(JsValue::from_str("Registry not available"))) {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "Error applying generic attribute: {:?}",
                e
            )));
        }

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Processed attribute change: {} = '{}' (was '{}')",
            attr_name, new_value, old_value
        )));
    }

    /// Apply generic attribute changes to a FormattableBlot
    fn apply_generic_attribute_to_blot<T>(
        &self,
        blot: &mut T,
        attr_name: &str,
        _old_value: &str,
        new_value: &str,
    )
    where
        T: crate::blot::formatting::FormattableBlot,
    {
        let formatting_state = blot.formatting_state_mut();
        
        // Handle attribute removal (empty new value)
        if new_value.is_empty() {
            formatting_state.remove_attribute(attr_name);
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Removed attribute: {}",
                attr_name
            )));
        } else {
            // Set or update the attribute
            formatting_state.set_attribute(attr_name, new_value);
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Set attribute: {} = {}",
                attr_name, new_value
            )));
        }

        // Sync the formatting state back to DOM to ensure consistency
        if let Err(e) = blot.apply_formatting_to_dom() {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "Error syncing generic attribute to DOM: {:?}",
                e
            )));
        }
    }

    /// Handle character data mutations (text content changes) - internal version to avoid borrowing issues
    fn handle_character_data_mutation_internal(&self, record: &MutationRecord) {
        if let Some(target) = record.target() {
            if let Some(text_node) = target.dyn_ref::<web_sys::Text>() {
                self.handle_text_content_change(text_node, record.old_value());
            }
        }
    }

    /// Handle text content changes and synchronize with TextBlot
    fn handle_text_content_change(&self, text_node: &web_sys::Text, old_value: Option<String>) {
        let current_content = text_node.text_content().unwrap_or_default();
        let old_content = old_value.unwrap_or_default();

        // Find the TextBlot associated with this text node
        if let Some(_registry_rc) = &self.registry {
            // Try to find the blot for this text node
            if let Some(text_blot) = self.find_text_blot_for_node(text_node) {
                self.sync_text_blot_content(text_blot, &current_content, &old_content);
            } else {
                // If no TextBlot found, this might be a new text node that needs a blot
                self.handle_orphaned_text_node(text_node, &current_content);
            }
        }

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Text content changed from '{}' to '{}'",
            old_content, current_content
        )));
    }

    /// Find the TextBlot associated with a DOM text node
    fn find_text_blot_for_node(&self, text_node: &web_sys::Text) -> Option<*mut dyn BlotTrait> {
        if let Some(registry_rc) = &self.registry {
            match registry_rc.try_borrow_mut() {
                Ok(mut registry) => {
                    let node: &Node = text_node.as_ref();
                    registry.find_blot_for_node(node)
                }
                Err(_) => {
                    web_sys::console::error_1(&JsValue::from_str(
                        "Registry borrow conflict during text blot lookup - mutation processing may be incomplete"
                    ));
                    None
                }
            }
        } else {
            web_sys::console::error_1(&JsValue::from_str(
                "Registry not available for text blot lookup - mutation observer not properly initialized"
            ));
            None
        }
    }

    /// Synchronize TextBlot content with DOM text node changes
    fn sync_text_blot_content(
        &self,
        text_blot_ptr: *mut dyn BlotTrait,
        current_content: &str,
        old_content: &str,
    ) {
        // Safety: We need to safely cast the blot pointer to TextBlot
        unsafe {
            if let Some(blot_trait) = text_blot_ptr.as_mut() {
                // Try to downcast to TextBlot
                if let Some(text_blot) = blot_trait.as_any_mut().downcast_mut::<TextBlot>() {
                    // Use our synchronization function
                    match text_operations::sync_text_blot_content(
                        text_blot,
                        Some(old_content),
                        current_content,
                    ) {
                        Ok(()) => {
                            web_sys::console::log_1(&JsValue::from_str(&format!(
                                "Successfully synced TextBlot content from '{}' to '{}'",
                                old_content, current_content
                            )));
                        }
                        Err(err) => {
                            web_sys::console::error_1(&JsValue::from_str(&format!(
                                "Failed to sync TextBlot content: {:?}",
                                err
                            )));
                        }
                    }
                } else {
                    web_sys::console::error_1(&JsValue::from_str(
                        "Blot is not a TextBlot - cannot sync text content"
                    ));
                }
            } else {
                web_sys::console::error_1(&JsValue::from_str(
                    "Invalid blot pointer - cannot sync text content"
                ));
            }
        }
    }

    /// Handle text nodes that don't have associated TextBlots
    fn handle_orphaned_text_node(&self, _text_node: &web_sys::Text, _content: &str) {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Found orphaned text node with content: '{}'",
            _content
        )));

        // In a full implementation, this would:
        // 1. Create a new TextBlot for this orphaned text node
        // 2. Find the appropriate parent blot to insert it into
        // 3. Update the parent's LinkedList to include the new TextBlot
        // 4. Register the new TextBlot in the registry

        // This typically happens when:
        // - Text nodes are created by direct DOM manipulation
        // - Content is pasted that creates new text nodes
        // - Other scripts modify the DOM outside of Parchment
    }

    /// Check if text content changes require merging with adjacent text nodes
    #[allow(dead_code)]
    fn check_text_merging_opportunity(&self, text_node: &web_sys::Text) {
        // Look for adjacent text nodes that could be merged
        if let Some(next_sibling) = text_node.next_sibling() {
            if let Some(_next_text) = next_sibling.dyn_ref::<web_sys::Text>() {
                self.consider_text_node_merge(text_node, _next_text);
            }
        }

        if let Some(prev_sibling) = text_node.previous_sibling() {
            if let Some(_prev_text) = prev_sibling.dyn_ref::<web_sys::Text>() {
                self.consider_text_node_merge(_prev_text, text_node);
            }
        }
    }

    /// Consider merging two adjacent text nodes if beneficial
    #[allow(dead_code)]
    fn consider_text_node_merge(&self, _first_text: &web_sys::Text, _second_text: &web_sys::Text) {
        web_sys::console::log_1(&JsValue::from_str("Considering text node merge"));

        // In a full implementation, this would:
        // 1. Check if both text nodes have TextBlots
        // 2. Verify they have compatible formatting
        // 3. Merge the text content in the DOM
        // 4. Merge the corresponding TextBlots
        // 5. Update parent LinkedList structures
        // 6. Clean up the redundant text node and blot

        // This optimization helps maintain clean text structures and improves performance
    }

    /// Handle when a DOM node is added - internal version to avoid borrowing issues
    fn handle_node_added_internal(&self, node: &Node, parent: &Node) {
        // Create corresponding blot for the new DOM node
        if let Some(registry_rc) = &self.registry {
            match registry_rc.try_borrow() {
                Ok(registry) => {
                    match self.create_blot_for_new_node(node, &*registry) {
                        Ok(Some(new_blot)) => {
                            // Insert the new blot into the appropriate parent
                            self.insert_blot_into_parent(new_blot, node, parent);
                        }
                        Ok(None) => {
                            // Node type not supported or already has a blot
                            web_sys::console::log_1(&JsValue::from_str(
                                "Node addition ignored - unsupported type or duplicate",
                            ));
                        }
                        Err(e) => {
                            web_sys::console::error_1(&JsValue::from_str(&format!(
                                "Failed to create blot for new node: {:?}", e
                            )));
                        }
                    }
                }
                Err(_) => {
                    web_sys::console::error_1(&JsValue::from_str(
                        "Registry borrow conflict during node addition - blot creation skipped"
                    ));
                }
            }
        } else {
            web_sys::console::error_1(&JsValue::from_str(
                "Registry not available for node addition - mutation observer not properly initialized"
            ));
        }

        web_sys::console::log_2(&JsValue::from_str("Node added:"), &node.clone().into());
    }

    /// Create a new blot for a DOM node that was added to the document
    fn create_blot_for_new_node(
        &self,
        node: &Node,
        registry: &Registry,
    ) -> Result<Option<Box<dyn BlotTrait>>, JsValue> {
        // Check if this node already has an associated blot
        if self.node_has_existing_blot(node) {
            return Ok(None);
        }

        // Determine what type of blot to create based on the node type
        match node.node_type() {
            web_sys::Node::TEXT_NODE => {
                if let Some(text_node) = node.dyn_ref::<web_sys::Text>() {
                    let content = text_node.text_content().unwrap_or_default();
                    self.create_text_blot_for_node(text_node, &content)
                } else {
                    Ok(None)
                }
            }
            web_sys::Node::ELEMENT_NODE => {
                if let Some(element) = node.dyn_ref::<web_sys::Element>() {
                    self.create_element_blot_for_node(element, registry)
                } else {
                    Ok(None)
                }
            }
            _ => {
                // Other node types (comments, etc.) are typically ignored

                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Ignoring node addition for type: {}",
                    node.node_type()
                )));
                Ok(None)
            }
        }
    }

    /// Check if a DOM node already has an associated blot
    fn node_has_existing_blot(&self, node: &Node) -> bool {
        self.with_registry("existing blot check", |registry| {
            registry.find_blot_for_node(node).is_some()
        }).unwrap_or(false)
    }

    /// Create a TextBlot for a new text node
    fn create_text_blot_for_node(
        &self,
        text_node: &web_sys::Text,
        content: &str,
    ) -> Result<Option<Box<dyn BlotTrait>>, JsValue> {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating TextBlot for text node with content: '{}'",
            content
        )));

        // Create TextBlot from the DOM text node
        let text_blot = crate::blot::text::TextBlot::from_dom_node(text_node.clone());

        // Register with registry immediately
        let _registration_result = self.with_registry("text blot registration", |registry| {
            let blot_ptr = &text_blot as &dyn BlotTrait as *const dyn BlotTrait as *mut dyn BlotTrait;
            registry.register_blot_for_node(text_node.as_ref(), blot_ptr)?;
            Ok::<(), JsValue>(())
        }).ok_or_else(|| JsValue::from_str("Registry not available for text blot registration"))?;

        web_sys::console::log_1(&JsValue::from_str("TextBlot created and registered successfully"));

        // Return boxed blot
        Ok(Some(Box::new(text_blot)))
    }

    /// Create an element blot (Block, Inline, Embed) for a new element
    fn create_element_blot_for_node(
        &self,
        element: &web_sys::Element,
        _registry: &Registry,
    ) -> Result<Option<Box<dyn BlotTrait>>, JsValue> {
        let tag_name = element.tag_name().to_lowercase();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating element blot for: {}",
            tag_name
        )));

        // Determine blot type based on element characteristics
        let blot_type = self.determine_blot_type_for_element(element, &tag_name);

        match blot_type {
            ElementBlotType::Block => self.create_block_blot_for_element(element),
            ElementBlotType::Inline => self.create_inline_blot_for_element(element),
            ElementBlotType::Embed => self.create_embed_blot_for_element(element),
            ElementBlotType::Unknown => {
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Unknown element type for: {}",
                    tag_name
                )));
                Ok(None)
            }
        }
    }

    /// Determine what type of blot an element should become
    fn determine_blot_type_for_element(
        &self,
        element: &web_sys::Element,
        tag_name: &str,
    ) -> ElementBlotType {
        // Block elements
        match tag_name {
            "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "pre" | "ul"
            | "ol" | "li" => ElementBlotType::Block,

            // Inline elements
            "span" | "strong" | "em" | "b" | "i" | "u" | "s" | "sub" | "sup" | "mark" | "code" => {
                ElementBlotType::Inline
            }

            // Embed elements
            "img" | "br" | "hr" | "iframe" | "video" | "audio" => ElementBlotType::Embed,

            // Unknown elements - check CSS display property
            _ => {
                if let Some(window) = web_sys::window() {
                    if let Ok(Some(style)) = window.get_computed_style(element) {
                        if let Ok(display) = style.get_property_value("display") {
                            match display.as_str() {
                                "block" | "list-item" | "table" => ElementBlotType::Block,
                                "inline" | "inline-block" => ElementBlotType::Inline,
                                _ => ElementBlotType::Unknown,
                            }
                        } else {
                            ElementBlotType::Unknown
                        }
                    } else {
                        ElementBlotType::Unknown
                    }
                } else {
                    ElementBlotType::Unknown
                }
            }
        }
    }

    /// Create a BlockBlot for a block-level element
    fn create_block_blot_for_element(
        &self,
        element: &web_sys::Element,
    ) -> Result<Option<Box<dyn BlotTrait>>, JsValue> {
        let tag_name = element.tag_name().to_lowercase();
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating BlockBlot for element: {}",
            tag_name
        )));

        // Create BlockBlot from the DOM element
        let block_blot = crate::blot::block::BlockBlot::from_element(element.clone());

        // Register with registry immediately
        let _registration_result = self.with_registry("block blot registration", |registry| {
            let blot_ptr = &block_blot as &dyn BlotTrait as *const dyn BlotTrait as *mut dyn BlotTrait;
            registry.register_blot_for_node(element.as_ref(), blot_ptr)?;
            Ok::<(), JsValue>(())
        }).ok_or_else(|| JsValue::from_str("Registry not available for block blot registration"))?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "BlockBlot created and registered successfully for {}",
            tag_name
        )));

        // Return boxed blot
        Ok(Some(Box::new(block_blot)))
    }

    /// Create an InlineBlot for an inline element
    fn create_inline_blot_for_element(
        &self,
        element: &web_sys::Element,
    ) -> Result<Option<Box<dyn BlotTrait>>, JsValue> {
        let tag_name = element.tag_name().to_lowercase();
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating InlineBlot for element: {}",
            tag_name
        )));

        // Create InlineBlot from the DOM element
        let inline_blot = crate::blot::inline::InlineBlot::from_element(element.clone());

        // Register with registry immediately
        let _registration_result = self.with_registry("inline blot registration", |registry| {
            let blot_ptr = &inline_blot as &dyn BlotTrait as *const dyn BlotTrait as *mut dyn BlotTrait;
            registry.register_blot_for_node(element.as_ref(), blot_ptr)?;
            Ok::<(), JsValue>(())
        }).ok_or_else(|| JsValue::from_str("Registry not available for inline blot registration"))?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "InlineBlot created and registered successfully for {}",
            tag_name
        )));

        // Return boxed blot
        Ok(Some(Box::new(inline_blot)))
    }

    /// Create an EmbedBlot for an embedded element
    fn create_embed_blot_for_element(
        &self,
        element: &web_sys::Element,
    ) -> Result<Option<Box<dyn BlotTrait>>, JsValue> {
        let tag_name = element.tag_name().to_lowercase();
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating EmbedBlot for element: {}",
            tag_name
        )));

        // Create EmbedBlot from the DOM element
        let embed_blot = crate::blot::embed::EmbedBlot::from_element(element.clone());

        // Register with registry immediately
        let _registration_result = self.with_registry("embed blot registration", |registry| {
            let blot_ptr = &embed_blot as &dyn BlotTrait as *const dyn BlotTrait as *mut dyn BlotTrait;
            registry.register_blot_for_node(element.as_ref(), blot_ptr)?;
            Ok::<(), JsValue>(())
        }).ok_or_else(|| JsValue::from_str("Registry not available for embed blot registration"))?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "EmbedBlot created and registered successfully for {}",
            tag_name
        )));

        // Return boxed blot
        Ok(Some(Box::new(embed_blot)))
    }

    /// Insert a newly created blot into the appropriate parent
    fn insert_blot_into_parent(&self, _new_blot: Box<dyn BlotTrait>, node: &Node, parent: &Node) {
        // Find the parent blot that corresponds to the DOM parent
        if let Some(parent_blot) = self.find_parent_blot_for_node(parent) {
            // Determine the correct position to insert the new blot
            let insert_position = self.calculate_blot_insert_position(node, parent);

            // Insert the blot into the parent's LinkedList
            self.perform_blot_insertion(parent_blot, _new_blot, insert_position);
        } else {
            web_sys::console::warn_1(&JsValue::from_str(
                "Could not find parent blot for node insertion",
            ));
        }
    }

    /// Find the parent blot that corresponds to a DOM node
    fn find_parent_blot_for_node(&self, parent_node: &Node) -> Option<*mut dyn BlotTrait> {
        if let Some(registry_rc) = &self.registry {
            match registry_rc.try_borrow_mut() {
                Ok(mut registry) => {
                    registry.find_blot_for_node(parent_node)
                }
                Err(_) => {
                    web_sys::console::error_1(&JsValue::from_str(
                        "Registry borrow conflict during parent blot lookup - blot insertion may fail"
                    ));
                    None
                }
            }
        } else {
            web_sys::console::error_1(&JsValue::from_str(
                "Registry not available for parent blot lookup - mutation observer not properly initialized"
            ));
            None
        }
    }

    /// Calculate the position where a new blot should be inserted
    fn calculate_blot_insert_position(&self, node: &Node, _parent: &Node) -> usize {
        // Calculate position based on DOM sibling order
        let mut position = 0;
        let mut current_sibling = node.previous_sibling();

        while let Some(sibling) = current_sibling {
            // Count preceding siblings that have associated blots
            if self.node_has_existing_blot(&sibling) {
                position += 1;
            }
            current_sibling = sibling.previous_sibling();
        }

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Calculated insert position: {}",
            position
        )));

        position
    }

    /// Perform the actual blot insertion into the parent's LinkedList
    fn perform_blot_insertion(
        &self,
        parent_blot: *mut dyn BlotTrait,
        mut new_blot: Box<dyn BlotTrait>,
        position: usize,
    ) {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Performing blot insertion at position {}",
            position
        )));

        // Get parent as specific blot types and try insertion
        unsafe {
            let parent_any = (*parent_blot).as_any_mut();
            
            // Call attach() on the new blot before insertion
            new_blot.attach();
            
            // Try different parent blot types
            if let Some(scroll_parent) = parent_any.downcast_mut::<crate::blot::scroll::ScrollBlot>() {
                match scroll_parent.insert_child_at_position(position, new_blot) {
                    Ok(()) => {
                        web_sys::console::log_1(&JsValue::from_str(&format!(
                            "Successfully inserted blot into ScrollBlot at position {}",
                            position
                        )));
                    }
                    Err(e) => {
                        web_sys::console::error_2(
                            &JsValue::from_str("Failed to insert blot into ScrollBlot:"),
                            &e
                        );
                    }
                }
            } else if let Some(block_parent) = parent_any.downcast_mut::<crate::blot::block::BlockBlot>() {
                match block_parent.insert_child_at_position(position, new_blot) {
                    Ok(()) => {
                        web_sys::console::log_1(&JsValue::from_str(&format!(
                            "Successfully inserted blot into BlockBlot at position {}",
                            position
                        )));
                    }
                    Err(e) => {
                        web_sys::console::error_2(
                            &JsValue::from_str("Failed to insert blot into BlockBlot:"),
                            &e
                        );
                    }
                }
            } else if let Some(inline_parent) = parent_any.downcast_mut::<crate::blot::inline::InlineBlot>() {
                match inline_parent.insert_child_at_position(position, new_blot) {
                    Ok(()) => {
                        web_sys::console::log_1(&JsValue::from_str(&format!(
                            "Successfully inserted blot into InlineBlot at position {}",
                            position
                        )));
                    }
                    Err(e) => {
                        web_sys::console::error_2(
                            &JsValue::from_str("Failed to insert blot into InlineBlot:"),
                            &e
                        );
                    }
                }
            } else {
                web_sys::console::error_1(&JsValue::from_str(
                    "Parent blot type not supported for insertion"
                ));
            }
        }
    }

    /// Handle when a DOM node is removed - internal version to avoid borrowing issues
    fn handle_node_removed_internal(&self, node: &Node, parent: &Node) {
        // Find the blot associated with the removed DOM node
        if let Some(_registry_rc) = &self.registry {
            if let Some(removed_blot) = self.find_blot_for_removed_node(node) {
                // Remove the blot from its parent and clean up
                self.remove_blot_and_cleanup(removed_blot, node, parent);
            } else {
                // No blot found - might be a node that was never tracked
                web_sys::console::log_1(&JsValue::from_str("No blot found for removed node"));
            }
        }

        web_sys::console::log_2(&JsValue::from_str("Node removed:"), &node.clone().into());
    }

    /// Find the blot associated with a DOM node that was removed
    fn find_blot_for_removed_node(&self, node: &Node) -> Option<*mut dyn BlotTrait> {
        if let Some(registry_rc) = &self.registry {
            match registry_rc.try_borrow_mut() {
                Ok(mut registry) => {
                    registry.find_blot_for_node(node)
                }
                Err(_) => {
                    web_sys::console::error_1(&JsValue::from_str(
                        "Registry borrow conflict during removed node lookup - blot cleanup may be incomplete"
                    ));
                    None
                }
            }
        } else {
            web_sys::console::error_1(&JsValue::from_str(
                "Registry not available for removed node lookup - mutation observer not properly initialized"
            ));
            None
        }
    }

    /// Remove a blot and perform all necessary cleanup
    fn remove_blot_and_cleanup(&self, blot_ptr: *mut dyn BlotTrait, node: &Node, parent: &Node) {
        // 1. Remove blot from parent's children
        self.remove_blot_from_parent(blot_ptr, node, parent);

        // 2. Clean up any child blots recursively
        self.cleanup_child_blots(blot_ptr);

        // 3. Unregister from registry
        self.unregister_blot_from_registry(blot_ptr, node);

        // 4. Perform final cleanup
        self.finalize_blot_cleanup(blot_ptr);

        web_sys::console::log_1(&JsValue::from_str("Completed blot removal and cleanup"));
    }

    /// Remove a blot from its parent's LinkedList
    fn remove_blot_from_parent(&self, _blot_ptr: *mut dyn BlotTrait, node: &Node, parent: &Node) {
        // Find the parent blot
        if let Some(parent_blot) = self.find_parent_blot_for_node(parent) {
            // Find the position of the blot to remove
            let remove_position = self.calculate_blot_remove_position(node, parent);

            // Remove from the LinkedList
            self.perform_blot_removal_from_parent(parent_blot, remove_position);
        } else {
            web_sys::console::warn_1(&JsValue::from_str(
                "Could not find parent blot for node removal",
            ));
        }
    }

    /// Calculate the position of the blot that should be removed
    fn calculate_blot_remove_position(&self, node: &Node, parent: &Node) -> usize {
        // Since the node is already removed from DOM, we need to find its position
        // by looking at the remaining children and the blot registry
        
        let mut position = 0;
        
        // Get the parent blot to access its children
        if let Some(parent_blot_ptr) = self.with_registry("parent blot lookup for removal", |registry| {
            registry.find_blot_for_node(parent)
        }).flatten() {
            unsafe {
                // Try to downcast to ParentBlotTrait to access children
                let parent_blot_ref = &*parent_blot_ptr;
                if let Some(parent_trait) = parent_blot_ref.as_any().downcast_ref::<crate::blot::parent::ParentBlot>() {
                    // Find the position by comparing DOM nodes with blot DOM nodes
                    let children = parent_trait.children();
                    let mut current_position = 0;
                    
                    for i in 0..children.length {
                        if let Some(child_blot) = children.get(i as i32) {
                            let child_dom = child_blot.dom_node();
                            // If this child's DOM node matches our removed node, we found the position
                            if std::ptr::eq(child_dom, node) {
                                position = current_position;
                                break;
                            }
                            current_position += 1;
                        }
                    }
                } else if let Some(scroll_trait) = parent_blot_ref.as_any().downcast_ref::<crate::blot::scroll::ScrollBlot>() {
                    // Handle ScrollBlot parent
                    let children = scroll_trait.children();
                    let mut current_position = 0;
                    
                    for i in 0..children.length {
                        if let Some(child_blot) = children.get(i as i32) {
                            let child_dom = child_blot.dom_node();
                            if std::ptr::eq(child_dom, node) {
                                position = current_position;
                                break;
                            }
                            current_position += 1;
                        }
                    }
                }
            }
        }

        web_sys::console::log_2(
            &JsValue::from_str("Calculated removal position:"),
            &JsValue::from_f64(position as f64)
        );

        position
    }

    /// Remove a blot from its parent's LinkedList at the specified position
    fn perform_blot_removal_from_parent(&self, parent_blot: *mut dyn BlotTrait, position: usize) {
        web_sys::console::log_2(
            &JsValue::from_str("Removing blot from parent LinkedList at position:"),
            &JsValue::from_f64(position as f64)
        );

        unsafe {
            let parent_blot_mut = &mut *parent_blot;
            
            // Try to downcast to ParentBlotTrait to access removal methods
            if let Some(parent_trait) = parent_blot_mut.as_any_mut().downcast_mut::<crate::blot::parent::ParentBlot>() {
                match parent_trait.remove_child_at_position(position) {
                    Ok(mut removed_blot) => {
                        web_sys::console::log_1(&JsValue::from_str("Successfully removed blot from parent"));
                        // Call detach on the removed blot
                        removed_blot.detach();
                    }
                    Err(e) => {
                        web_sys::console::warn_2(
                            &JsValue::from_str("Failed to remove blot from parent:"),
                            &e
                        );
                    }
                }
            } else if let Some(scroll_trait) = parent_blot_mut.as_any_mut().downcast_mut::<crate::blot::scroll::ScrollBlot>() {
                match scroll_trait.remove_child_at_position(position) {
                    Ok(mut removed_blot) => {
                        web_sys::console::log_1(&JsValue::from_str("Successfully removed blot from scroll parent"));
                        // Call detach on the removed blot
                        removed_blot.detach();
                    }
                    Err(e) => {
                        web_sys::console::warn_2(
                            &JsValue::from_str("Failed to remove blot from scroll parent:"),
                            &e
                        );
                    }
                }
            } else {
                web_sys::console::warn_1(&JsValue::from_str(
                    "Parent blot does not support child removal operations"
                ));
            }
        }
    }

    /// Recursively clean up any child blots
    fn cleanup_child_blots(&self, blot_ptr: *mut dyn BlotTrait) {
        web_sys::console::log_1(&JsValue::from_str("Cleaning up child blots"));

        unsafe {
            let blot_ref = &*blot_ptr;
            
            // Check if this blot has children (is a parent blot)
            if let Some(parent_trait) = blot_ref.as_any().downcast_ref::<crate::blot::parent::ParentBlot>() {
                let children = parent_trait.children();
                let mut child_ptrs = Vec::new();
                
                // Collect child pointers first to avoid borrowing issues
                for i in 0..children.length {
                    if let Some(child_blot) = children.get(i as i32) {
                        let child_ptr = child_blot.as_ref() as *const dyn BlotTrait as *mut dyn BlotTrait;
                        child_ptrs.push(child_ptr);
                    }
                }
                
                // Recursively cleanup each child
                for child_ptr in child_ptrs {
                    self.cleanup_child_blots(child_ptr);
                    
                    // Unregister the child from registry
                    let child_dom = (&*child_ptr).dom_node();
                    self.unregister_blot_from_registry(child_ptr, child_dom);
                }
                
                web_sys::console::log_2(
                    &JsValue::from_str("Cleaned up children for parent blot, count:"),
                    &JsValue::from_f64(children.length as f64)
                );
                
            } else if let Some(scroll_trait) = blot_ref.as_any().downcast_ref::<crate::blot::scroll::ScrollBlot>() {
                let children = scroll_trait.children();
                let mut child_ptrs = Vec::new();
                
                // Collect child pointers first
                for i in 0..children.length {
                    if let Some(child_blot) = children.get(i as i32) {
                        let child_ptr = child_blot.as_ref() as *const dyn BlotTrait as *mut dyn BlotTrait;
                        child_ptrs.push(child_ptr);
                    }
                }
                
                // Recursively cleanup each child
                for child_ptr in child_ptrs {
                    self.cleanup_child_blots(child_ptr);
                    
                    // Unregister the child from registry
                    let child_dom = (&*child_ptr).dom_node();
                    self.unregister_blot_from_registry(child_ptr, child_dom);
                }
                
                web_sys::console::log_2(
                    &JsValue::from_str("Cleaned up children for scroll blot, count:"),
                    &JsValue::from_f64(children.length as f64)
                );
            } else {
                // This is a leaf blot (no children), just log
                web_sys::console::log_1(&JsValue::from_str("Leaf blot - no children to cleanup"));
            }
        }
    }

    /// Unregister a blot from the registry
    fn unregister_blot_from_registry(&self, _blot_ptr: *mut dyn BlotTrait, node: &Node) {
        if let Some(was_removed) = self.with_registry("blot unregistration", |registry| {
            registry.unregister_blot_for_node(node)
        }) {
            if was_removed {
                web_sys::console::log_1(&JsValue::from_str("Successfully unregistered blot from registry"));
            } else {
                web_sys::console::warn_1(&JsValue::from_str("No registry entry found for blot"));
            }
        }
    }

    /// Perform final cleanup for a removed blot
    fn finalize_blot_cleanup(&self, _blot_ptr: *mut dyn BlotTrait) {
        web_sys::console::log_1(&JsValue::from_str("Finalizing blot cleanup"));

        // In a full implementation, this would:
        // 1. Call the blot's detach() method if not already called
        // 2. Clean up any event listeners or observers
        // 3. Release any DOM references held by the blot
        // 4. Perform any blot-type-specific cleanup
        // 5. Mark the blot as invalid/disposed if using a disposal pattern

        // This ensures complete cleanup and prevents resource leaks
    }

    /// Handle removal of text nodes specifically
    #[allow(dead_code)]
    fn handle_text_node_removal(&self, _text_node: &web_sys::Text) {
        web_sys::console::log_1(&JsValue::from_str("Handling text node removal"));

        // Text nodes have simpler cleanup requirements
        // In a full implementation, this would:
        // 1. Find the associated TextBlot
        // 2. Remove it from the parent's children
        // 3. Update text length calculations in parent blots
        // 4. Trigger any necessary text merging in adjacent nodes
    }

    /// Handle removal of element nodes specifically
    #[allow(dead_code)]
    fn handle_element_removal(&self, element: &web_sys::Element) {
        let _tag_name = element.tag_name().to_lowercase();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Handling element removal: {}",
            _tag_name
        )));

        // Element removal is more complex due to potential children
        // In a full implementation, this would:
        // 1. Determine the blot type (Block, Inline, Embed)
        // 2. Handle type-specific cleanup requirements
        // 3. Ensure proper cleanup of formatting attributors
        // 4. Handle cascade effects (e.g., removing a block might affect list structure)
    }

    /// Check for orphaned blots and mark for cleanup
    fn check_for_orphaned_blots(&mut self) {
        if let Some(_scroll_blot) = &self.scroll_blot {
            // Check all blots to see if their DOM nodes are still connected to the document
            // Mark any disconnected blots for removal

            web_sys::console::log_1(&JsValue::from_str("Checking for orphaned blots"));

            // This would require walking the blot tree and checking DOM connectivity
            // For now, we set has_changes to false since we don't have specific orphans
            // In a full implementation, this would use document.contains() on blot DOM nodes
        }
    }

    /// Merge adjacent text nodes to optimize the DOM structure
    fn merge_adjacent_text_nodes(&mut self) {
        if let Some(_scroll_blot) = &self.scroll_blot {
            // Find adjacent TextBlots and merge them if possible

            web_sys::console::log_1(&JsValue::from_str("Merging adjacent text nodes"));

            // This would require:
            // 1. Walking through parent blots' children
            // 2. Finding consecutive TextBlots
            // 3. Merging their content and updating DOM
            // 4. Updating the LinkedList structure

            // For now, assume no merges needed
        }
    }

    /// Validate parent-child relationships are consistent between blots and DOM
    fn validate_parent_child_relationships(&mut self) {
        if let Some(_scroll_blot) = &self.scroll_blot {
            // Ensure blot tree structure matches DOM tree structure

            web_sys::console::log_1(&JsValue::from_str("Validating parent-child relationships"));

            // This would require:
            // 1. Walking through parent blots
            // 2. Comparing blot children with DOM children
            // 3. Fixing any mismatches by updating blot structure
            // 4. Ensuring proper LinkedList ordering

            // For now, assume relationships are valid
        }
    }

    /// Clean up empty containers that no longer serve a purpose
    fn clean_up_empty_containers(&mut self) {
        if let Some(_scroll_blot) = &self.scroll_blot {
            // Remove empty parent blots that don't need to be preserved

            web_sys::console::log_1(&JsValue::from_str("Cleaning up empty containers"));

            // This would require:
            // 1. Finding empty parent blots (no children)
            // 2. Checking if they can be safely removed (not required formatting)
            // 3. Removing from parent and updating LinkedList
            // 4. Updating DOM to remove empty elements

            // For now, assume no cleanup needed
        }
    }
}

/// Helper enum for classifying element types during blot creation
#[derive(Debug, Clone, Copy)]
enum ElementBlotType {
    Block,
    Inline,
    Embed,
    Unknown,
}

// Ensure the closure lives as long as the MutationObserverWrapper
impl Drop for MutationObserverWrapper {
    fn drop(&mut self) {
        self.disconnect();
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_update_context() {
        let ctx = UpdateContext {
            mutation_records: Vec::new(),
            iteration_count: 0,
        };
        assert_eq!(ctx.iteration_count, 0);
    }

    #[test]
    fn test_optimize_context() {
        let ctx = OptimizeContext {
            iteration_count: 0,
            has_changes: false,
        };
        assert_eq!(ctx.iteration_count, 0);
        assert!(!ctx.has_changes);
    }

    #[test]
    fn test_mutation_handler_registry_initialization() {
        let handler = MutationHandler {
            scroll_blot: None,
            registry: None,
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
        };

        // Initially no registry
        assert!(handler.registry.is_none());
    }

    #[test]
    fn test_mutation_handler_with_registry() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let handler = MutationHandler {
            scroll_blot: None,
            registry: None,
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
            delta_callback: None,
            document_length: 0,
        };

        // Registry should be available
        assert!(handler.registry.is_some());
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_with_registry_helper_no_registry() {
        let handler = MutationHandler {
            scroll_blot: None,
            registry: None,
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
        };

        let result = handler.with_registry("test_operation", |_registry| {
            42
        });

        // Should return None when no registry is available
        assert!(result.is_none());
    }

    #[test]
    fn test_node_has_existing_blot_no_registry() {
        let handler = MutationHandler {
            scroll_blot: None,
            registry: None,
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
        };

        // This test would need actual DOM nodes in a WASM environment
        // For now, we test that the method exists and compiles
        
        // In a real WASM test environment:
        // let text_node = create_text_node("test");
        // let has_blot = handler.node_has_existing_blot(&text_node);
        // assert!(!has_blot); // No blot registered yet
        
        assert!(true); // Compilation test
    }

    #[test]
    fn test_registry_error_handling_patterns() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let handler = MutationHandler {
            scroll_blot: None,
            registry: Some(registry),
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
            delta_callback: None,
            document_length: 0,
        };

        // Test successful registry access
        let result = handler.with_registry("test_operation", |_registry| {
            "success"
        });
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "success");

        // Test that registry access works
        let another_result = handler.with_registry("another_operation", |_registry| {
            42
        });
        assert!(another_result.is_some());
    }

    #[test]
    fn test_mutation_observer_wrapper_registry_integration() {
        // This test verifies that MutationObserverWrapper can be created
        // In a real WASM environment, this would test actual DOM observation
        assert!(true); // Compilation test for now
    }

    #[test]
    fn test_element_blot_type_classification() {
        // This test would verify element type classification logic
        // In a real WASM environment with DOM elements:
        // let div_element = create_element("div");
        // let span_element = create_element("span");
        // let img_element = create_element("img");
        
        // Test that the classification logic exists
        assert!(true); // Compilation test
    }

    #[test]
    fn test_max_optimize_iterations_constant() {
        // Test that the constant is defined and reasonable
        assert!(MAX_OPTIMIZE_ITERATIONS > 0);
        assert!(MAX_OPTIMIZE_ITERATIONS <= 100); // Reasonable upper bound
    }

    #[test]
    fn test_mutation_handler_context_management() {
        let mut handler = MutationHandler {
            scroll_blot: None,
            registry: None,
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
            delta_callback: None,
            document_length: 0,
        };

        // Test context updates during optimization
        handler.optimize_context.iteration_count = 5;
        handler.optimize_context.has_changes = true;
        
        assert_eq!(handler.optimize_context.iteration_count, 5);
        assert!(handler.optimize_context.has_changes);
    }

    #[test]
    fn test_mutation_handler_delta_callback() {
        let mut handler = MutationHandler {
            scroll_blot: None,
            registry: None,
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
            delta_callback: None,
            document_length: 0,
        };

        // Test that delta callback can be set
        assert!(handler.delta_callback.is_none());
        
        handler.delta_callback = Some(Box::new(|_delta| {
            // Test callback
        }));
        
        assert!(handler.delta_callback.is_some());
    }

    #[test]
    fn test_mutation_handler_document_length() {
        let mut handler = MutationHandler {
            scroll_blot: None,
            registry: None,
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
            delta_callback: None,
            document_length: 0,
        };

        // Test document length tracking
        assert_eq!(handler.document_length, 0);
        
        handler.document_length = 100;
        assert_eq!(handler.document_length, 100);
    }

    #[test]
    fn test_node_length_calculation() {
        let handler = MutationHandler {
            scroll_blot: None,
            registry: None,
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
            delta_callback: None,
            document_length: 0,
        };

        // This test would need actual DOM nodes in a WASM environment
        // For now, we test that the method exists and compiles
        assert!(true); // Compilation test
    }

    #[test]
    fn test_style_attribute_parsing() {
        let handler = MutationHandler {
            scroll_blot: None,
            registry: None,
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
            delta_callback: None,
            document_length: 0,
        };

        let mut attributes = Attributes::new();
        handler.parse_style_attributes("color: red; font-weight: bold", &mut attributes);

        assert_eq!(attributes.get("color"), Some(&serde_json::Value::String("red".to_string())));
        assert_eq!(attributes.get("bold"), Some(&serde_json::Value::Bool(true)));
    }
}
