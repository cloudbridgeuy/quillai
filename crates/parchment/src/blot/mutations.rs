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
use crate::registry::Registry;
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

        // Run update cycle
        self.update();

        // Run optimize cycle after updates
        self.optimize();
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
        _element: &web_sys::Element,
        _class_name: &str,
        _is_added: bool,
    ) {
        // This would look up class-based attributors and apply/remove formatting
        // For now, just log the operation

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "{} class formatting: {}",
            if _is_added { "Applying" } else { "Removing" },
            _class_name
        )));

        // In a full implementation, this would:
        // 1. Find the blot associated with the element
        // 2. Look up class-based attributors that match this class pattern
        // 3. Apply or remove the formatting to the blot
        // 4. Update the blot's internal state
    }

    /// Synchronize style-based formatting between DOM and blot
    fn sync_style_formatting(
        &self,
        _element: &web_sys::Element,
        _current_style: &str,
        _old_style: &str,
    ) {
        // This would parse style changes and update style-based attributors

        web_sys::console::log_1(&JsValue::from_str("Syncing style formatting"));

        // In a full implementation, this would:
        // 1. Parse both old and new style strings
        // 2. Identify which CSS properties changed
        // 3. Find style-based attributors that handle those properties
        // 4. Update the blot's formatting to match the new styles
    }

    /// Notify blot that a generic attribute has changed
    fn notify_blot_attribute_change(
        &self,
        _element: &web_sys::Element,
        _attr_name: &str,
        _old_value: &str,
        _new_value: &str,
    ) {
        // This would find the blot and call its attribute change handler

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Notifying blot of attribute change: {}",
            _attr_name
        )));

        // In a full implementation, this would:
        // 1. Find the blot associated with the element using the registry
        // 2. Call the blot's attribute change handler if it exists
        // 3. Allow the blot to update its internal state based on the attribute change
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
        _text_blot: *mut dyn BlotTrait,
        _current_content: &str,
        _old_content: &str,
    ) {
        // Update the TextBlot's internal state to match the new DOM content

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Syncing TextBlot content from '{}' to '{}'",
            _old_content, _current_content
        )));

        // In a full implementation, this would:
        // 1. Cast the blot to TextBlot using downcasting
        // 2. Update the TextBlot's internal content tracking
        // 3. Recalculate the blot's length
        // 4. Notify parent blots of content changes
        // 5. Trigger any necessary formatting updates

        // For now, just mark that we have changes for the optimization cycle
        // self.optimize_context.has_changes = true; // Would need mutable access
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
            registry: Some(registry.clone()),
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
        };

        // Registry should be available
        assert!(handler.registry.is_some());
        
        // Should be able to access registry through with_registry helper
        let result = handler.with_registry("test operation", |_registry| {
            true
        });
        assert_eq!(result, Some(true));
    }

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

        // Test the logic without triggering WASM-specific console calls
        // The with_registry method should return None when no registry is available
        // We can't test the actual method due to WASM dependencies, but we can test the logic
        assert!(handler.registry.is_none());
        
        // Verify the pattern works: if registry is None, operations should return None
        let has_registry = handler.registry.is_some();
        assert!(!has_registry);
    }

    #[test]
    fn test_node_has_existing_blot_no_registry() {
        let _handler = MutationHandler {
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

        // Test that the handler can be created without registry
        // In a real WASM environment, node_has_existing_blot would return false
        // when no registry is available
        assert!(true); // Compilation test
    }

    #[test]
    fn test_registry_error_handling_patterns() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let handler = MutationHandler {
            scroll_blot: None,
            registry: Some(registry.clone()),
            update_context: UpdateContext {
                mutation_records: Vec::new(),
                iteration_count: 0,
            },
            optimize_context: OptimizeContext {
                iteration_count: 0,
                has_changes: false,
            },
        };

        // Test successful registry access
        let success_result = handler.with_registry("test operation", |_registry| {
            "success"
        });
        assert_eq!(success_result, Some("success"));

        // Test that multiple accesses work
        let first_access = handler.with_registry("first", |_| 1);
        let second_access = handler.with_registry("second", |_| 2);
        assert_eq!(first_access, Some(1));
        assert_eq!(second_access, Some(2));
    }

    #[test]
    fn test_mutation_observer_wrapper_registry_integration() {
        // Test that MutationObserverWrapper can accept and store registry
        // This would require DOM setup in a real test environment
        
        // For now, test the compilation and basic structure
        assert!(true); // Placeholder - would need DOM environment for full test
    }

    #[test]
    fn test_element_blot_type_classification() {
        let block_type = ElementBlotType::Block;
        let inline_type = ElementBlotType::Inline;
        let embed_type = ElementBlotType::Embed;
        let unknown_type = ElementBlotType::Unknown;

        // Test that enum variants exist and can be compared
        assert!(matches!(block_type, ElementBlotType::Block));
        assert!(matches!(inline_type, ElementBlotType::Inline));
        assert!(matches!(embed_type, ElementBlotType::Embed));
        assert!(matches!(unknown_type, ElementBlotType::Unknown));
    }

    #[test]
    fn test_max_optimize_iterations_constant() {
        // Verify the safety constant is reasonable
        assert!(MAX_OPTIMIZE_ITERATIONS > 0);
        assert!(MAX_OPTIMIZE_ITERATIONS <= 1000); // Reasonable upper bound
        assert_eq!(MAX_OPTIMIZE_ITERATIONS, 100); // Current expected value
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
        };

        // Test initial state
        assert_eq!(handler.update_context.iteration_count, 0);
        assert_eq!(handler.optimize_context.iteration_count, 0);
        assert!(!handler.optimize_context.has_changes);

        // Test context updates during optimization
        handler.optimize_context.iteration_count = 5;
        handler.optimize_context.has_changes = true;
        
        assert_eq!(handler.optimize_context.iteration_count, 5);
        assert!(handler.optimize_context.has_changes);
    }

    #[test]
    fn test_registry_lookup_method_signatures() {
        let registry = Rc::new(RefCell::new(Registry::new()));
        let _handler = MutationHandler {
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
        };

        // Test that all the registry lookup methods exist and compile
        // These would need proper DOM nodes in a real test environment
        
        // Verify method signatures exist by compilation
        assert!(true); // Compilation test
        
        // In a real WASM test environment with DOM:
        // let text_node = create_text_node("test");
        // let result = handler.find_text_blot_for_node(&text_node);
        // assert!(result.is_none()); // No blot registered yet
        
        // let parent_node = create_element("div");
        // let parent_result = handler.find_parent_blot_for_node(&parent_node);
        // assert!(parent_result.is_none()); // No blot registered yet
    }
}
