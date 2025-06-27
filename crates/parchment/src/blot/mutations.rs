use crate::blot::traits_simple::BlotTrait;
use crate::registry::Registry;
use js_sys::Array;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{MutationObserver, MutationRecord, Node};

/// Maximum number of optimize iterations to prevent infinite loops
const MAX_OPTIMIZE_ITERATIONS: usize = 100;

/// Context for update operations during DOM mutations
#[derive(Debug, Clone)]
pub struct UpdateContext {
    pub mutation_records: Vec<MutationRecord>,
    pub iteration_count: usize,
}

/// Context for optimize operations after DOM mutations
#[derive(Debug, Clone)]
pub struct OptimizeContext {
    pub iteration_count: usize,
    pub has_changes: bool,
}

/// Wrapper for MutationObserver that provides Rust-friendly API
/// and integrates with the Parchment blot system
pub struct MutationObserverWrapper {
    observer: MutationObserver,
    target_node: Node,
    /// Closure that handles mutation records - must be kept alive for WASM
    #[allow(dead_code)]
    callback: Closure<dyn FnMut(Array, MutationObserver)>,
    /// Shared state for handling mutations
    handler: Rc<RefCell<MutationHandler>>,
}

/// Internal handler for processing mutation records
struct MutationHandler {
    /// Reference to the scroll blot for document-level operations
    scroll_blot: Option<*mut dyn BlotTrait>,
    /// Registry for finding blots from DOM nodes
    registry: Option<Registry>,
    /// Current update context
    update_context: UpdateContext,
    /// Current optimize context
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
    pub fn set_registry(&self, registry: Registry) {
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
        if let Some(_registry) = &self.registry {
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
        if let Some(_registry) = &self.registry {
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
    fn find_text_blot_for_node(&self, _text_node: &web_sys::Text) -> Option<*mut dyn BlotTrait> {
        // This would use the registry to find the TextBlot associated with this DOM node
        // For now, return None since we don't have the full registry lookup implementation

        web_sys::console::log_1(&JsValue::from_str("Looking up TextBlot for text node"));

        // In a full implementation, this would:
        // 1. Use the registry's WeakMap-like storage to find the blot
        // 2. Verify that the blot is indeed a TextBlot
        // 3. Return a reference to the TextBlot for synchronization
        None
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
        if let Some(registry) = &self.registry {
            match self.create_blot_for_new_node(node, registry) {
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
                    web_sys::console::error_1(&e);
                }
            }
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
    fn node_has_existing_blot(&self, _node: &Node) -> bool {
        // This would check the registry's WeakMap-like storage
        // For now, assume no existing blot to avoid duplicates in this implementation

        web_sys::console::log_1(&JsValue::from_str("Checking for existing blot"));

        // In a full implementation, this would:
        // 1. Look up the node in the registry's DOM-to-Blot mapping
        // 2. Return true if a blot already exists for this node
        // 3. This prevents creating duplicate blots for the same DOM node
        false
    }

    /// Create a TextBlot for a new text node
    fn create_text_blot_for_node(
        &self,
        _text_node: &web_sys::Text,
        _content: &str,
    ) -> Result<Option<Box<dyn BlotTrait>>, JsValue> {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating TextBlot for text node with content: '{}'",
            _content
        )));

        // In a full implementation, this would:
        // 1. Create a new TextBlot instance
        // 2. Associate it with the DOM text node
        // 3. Set up the proper content and length tracking
        // 4. Return the boxed blot for insertion

        // For now, return None since we don't have full TextBlot creation
        Ok(None)
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
        _element: &web_sys::Element,
    ) -> Result<Option<Box<dyn BlotTrait>>, JsValue> {
        web_sys::console::log_1(&JsValue::from_str("Creating BlockBlot"));

        // In a full implementation, this would create a new BlockBlot instance
        Ok(None)
    }

    /// Create an InlineBlot for an inline element
    fn create_inline_blot_for_element(
        &self,
        _element: &web_sys::Element,
    ) -> Result<Option<Box<dyn BlotTrait>>, JsValue> {
        web_sys::console::log_1(&JsValue::from_str("Creating InlineBlot"));

        // In a full implementation, this would create a new InlineBlot instance
        Ok(None)
    }

    /// Create an EmbedBlot for an embedded element
    fn create_embed_blot_for_element(
        &self,
        _element: &web_sys::Element,
    ) -> Result<Option<Box<dyn BlotTrait>>, JsValue> {
        web_sys::console::log_1(&JsValue::from_str("Creating EmbedBlot"));

        // In a full implementation, this would create a new EmbedBlot instance
        Ok(None)
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
    fn find_parent_blot_for_node(&self, _parent_node: &Node) -> Option<*mut dyn BlotTrait> {
        // This would use the registry to find the blot associated with the parent DOM node

        web_sys::console::log_1(&JsValue::from_str("Looking up parent blot"));

        // In a full implementation, this would:
        // 1. Use the registry's DOM-to-Blot mapping
        // 2. Verify the found blot implements ParentBlotTrait
        // 3. Return a mutable reference for child insertion
        None
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
        _parent_blot: *mut dyn BlotTrait,
        _new_blot: Box<dyn BlotTrait>,
        _position: usize,
    ) {
        web_sys::console::log_1(&JsValue::from_str("Performing blot insertion"));

        // In a full implementation, this would:
        // 1. Get mutable access to the parent blot
        // 2. Insert the new blot at the calculated position
        // 3. Update the LinkedList structure
        // 4. Call attach() on the new blot
        // 5. Update any necessary indices or references
    }

    /// Handle when a DOM node is removed - internal version to avoid borrowing issues
    fn handle_node_removed_internal(&self, node: &Node, parent: &Node) {
        // Find the blot associated with the removed DOM node
        if let Some(_registry) = &self.registry {
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
    fn find_blot_for_removed_node(&self, _node: &Node) -> Option<*mut dyn BlotTrait> {
        // Look up the blot in the registry before it gets cleaned up

        web_sys::console::log_1(&JsValue::from_str("Looking up blot for removed node"));

        // In a full implementation, this would:
        // 1. Check the registry's DOM-to-Blot mapping
        // 2. Return the associated blot if found
        // 3. This needs to happen before the registry entry is cleaned up
        None
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
    fn calculate_blot_remove_position(&self, _node: &Node, _parent: &Node) -> usize {
        // This is more complex than insertion because we need to account for
        // the fact that the DOM node is already removed but the blot might still exist
        let position = 0;

        // We need to look at the next sibling (if any) and count backwards
        // This is because the removed node is no longer in the DOM tree

        web_sys::console::log_1(&JsValue::from_str(
            "Calculating removal position (simplified)",
        ));

        // In a full implementation, this would:
        // 1. Use the previous/next sibling information from before removal
        // 2. Count the blot position based on remaining sibling blots
        // 3. Handle edge cases where it's the first, last, or only child

        position
    }

    /// Remove a blot from its parent's LinkedList at the specified position
    fn perform_blot_removal_from_parent(&self, _parent_blot: *mut dyn BlotTrait, _position: usize) {
        web_sys::console::log_1(&JsValue::from_str("Removing blot from parent LinkedList"));

        // In a full implementation, this would:
        // 1. Get mutable access to the parent blot
        // 2. Remove the child at the specified position from LinkedList
        // 3. Update any necessary indices or references
        // 4. Call detach() on the removed blot
    }

    /// Recursively clean up any child blots
    fn cleanup_child_blots(&self, _blot_ptr: *mut dyn BlotTrait) {
        web_sys::console::log_1(&JsValue::from_str("Cleaning up child blots"));

        // In a full implementation, this would:
        // 1. Check if the blot is a parent blot (has children)
        // 2. Recursively remove and cleanup all child blots
        // 3. Ensure proper cleanup order (children first, then parent)
        // 4. Handle complex nested structures properly

        // This is important for preventing memory leaks and maintaining consistency
    }

    /// Unregister a blot from the registry
    fn unregister_blot_from_registry(&self, _blot_ptr: *mut dyn BlotTrait, _node: &Node) {
        web_sys::console::log_1(&JsValue::from_str("Unregistering blot from registry"));

        // In a full implementation, this would:
        // 1. Remove the DOM-to-Blot mapping from the registry
        // 2. Remove any other registry entries for this blot
        // 3. Clean up any cached references or indices
        // 4. Ensure the blot can be properly garbage collected

        // This prevents stale references and memory leaks
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

    #[test]
    fn test_mutation_observer_creation() {
        // This test would need to run in a browser context
        // For now, just test that the structure compiles
    }

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
}
