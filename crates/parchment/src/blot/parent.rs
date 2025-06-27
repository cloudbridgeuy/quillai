use crate::blot::traits_simple::{create_element_with_class, BlotTrait, ParentBlotTrait};
use crate::collection::linked_list::LinkedList;
use crate::registry::Registry;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, Node};

/// ParentBlot is the base implementation for all parent blots
/// This mirrors the TypeScript ParentBlot class with LinkedList child management
pub struct ParentBlot {
    pub dom_node: Node,
    pub children: LinkedList<Box<dyn BlotTrait>>,
}

impl ParentBlot {
    /// Create a new ParentBlot with the given DOM node
    pub fn new(dom_node: Node) -> Result<Self, JsValue> {
        let blot = ParentBlot {
            dom_node,
            children: LinkedList::new(),
        };

        // Note: Registry registration will be handled by the caller
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

    /// Find child at specific index - mirrors TypeScript find() method
    pub fn find_child(&self, index: usize) -> Option<&dyn BlotTrait> {
        self.children.get(index as i32).map(|child| child.as_ref())
    }

    /// Find child index - helper method
    pub fn find_child_index(&self, child: &dyn BlotTrait) -> Option<usize> {
        let mut current_index = 0;
        let mut current_node = self.children.head;

        while let Some(node_ptr) = current_node {
            unsafe {
                let node_ref = node_ptr.as_ref();
                if std::ptr::eq(node_ref.val.as_ref(), child) {
                    return Some(current_index);
                }
                current_node = node_ref.next;
                current_index += 1;
            }
        }

        None
    }

    /// Helper method to recursively collect descendants
    pub fn collect_descendants(
        &self,
        results: &mut Vec<&dyn BlotTrait>,
        matcher: fn(&dyn BlotTrait) -> bool,
    ) {
        let mut current_node = self.children.head;

        while let Some(node_ptr) = current_node {
            unsafe {
                let node_ref = node_ptr.as_ref();
                let child = node_ref.val.as_ref();

                if matcher(child) {
                    results.push(child);
                }

                // Recursively search in child if it's a parent
                if let Some(parent_child) = child.as_any().downcast_ref::<ParentBlot>() {
                    parent_child.collect_descendants(results, matcher);
                }

                current_node = node_ref.next;
            }
        }
    }
}

impl BlotTrait for ParentBlot {
    fn get_blot_name(&self) -> &'static str {
        "parent"
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
        // Sum of all children lengths
        let mut total_length = 0;
        let mut current_node = self.children.head;

        while let Some(node_ptr) = current_node {
            unsafe {
                let node_ref = node_ptr.as_ref();
                total_length += node_ref.val.length();
                current_node = node_ref.next;
            }
        }

        total_length
    }

    /// Attach this blot and all children - mirrors TypeScript attach()
    fn attach(&mut self) {
        // Attach all children
        let mut current_node = self.children.head;
        while let Some(mut node_ptr) = current_node {
            unsafe {
                let node_ref = node_ptr.as_mut();
                node_ref.val.attach();
                current_node = node_ref.next;
            }
        }
    }

    /// Detach this blot and all children - mirrors TypeScript detach()
    fn detach(&mut self) {
        // Detach all children
        let mut current_node = self.children.head;
        while let Some(mut node_ptr) = current_node {
            unsafe {
                let node_ref = node_ptr.as_mut();
                node_ref.val.detach();
                current_node = node_ref.next;
            }
        }

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
        // TODO: Implement proper deletion logic
        // This requires careful handling of mutable borrowing and child management
    }

    /// Insert at index - mirrors TypeScript insertAt()
    fn insert_at(&mut self, _index: usize, value: &str) {
        // For now, create a text blot and append it
        // TODO: Implement proper index-based insertion
        if let Ok(text_blot) = crate::blot::text::TextBlot::new(value) {
            let _ = self.append_child(Box::new(text_blot));
        }
    }

    /// Support for downcasting - needed for tree navigation
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Override build_children to implement recursive DOM traversal
    fn build_children(&mut self) -> Result<(), JsValue> {
        // Clear existing children
        self.children = LinkedList::new();

        // Traverse DOM children and create corresponding blots
        let child_nodes = self.dom_node.child_nodes();

        for i in 0..child_nodes.length() {
            if let Some(child_node) = child_nodes.get(i) {
                // Create blot from DOM node using Registry
                match Registry::create_blot_from_node(&child_node) {
                    Ok(mut child_blot) => {
                        // Recursively build children if this is a parent blot
                        child_blot.build_children()?;

                        // Attach the child blot
                        child_blot.attach();

                        // Add to our children LinkedList
                        self.children.insert_at_tail(child_blot);
                    }
                    Err(e) => {
                        // Log warning for unsupported nodes but continue processing
                        #[cfg(debug_assertions)]
                        web_sys::console::warn_2(
                            &JsValue::from_str("Failed to create blot from DOM node:"),
                            &e,
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

impl ParentBlotTrait for ParentBlot {
    fn children(&self) -> &LinkedList<Box<dyn BlotTrait>> {
        &self.children
    }

    fn children_mut(&mut self) -> &mut LinkedList<Box<dyn BlotTrait>> {
        &mut self.children
    }

    fn dom_element(&self) -> &HtmlElement {
        // Safe to unwrap since ParentBlot should always use an Element
        self.dom_node.dyn_ref::<HtmlElement>().unwrap()
    }

    /// Append child - mirrors TypeScript appendChild()
    fn append_child(&mut self, mut child: Box<dyn BlotTrait>) -> Result<(), JsValue> {
        // Attach the child
        child.attach();

        // Add to DOM
        self.dom_node.append_child(child.dom_node())?;

        // Add to LinkedList
        self.children.insert_at_tail(child);

        Ok(())
    }

    /// Insert before - mirrors TypeScript insertBefore()
    fn insert_before(
        &mut self,
        mut child: Box<dyn BlotTrait>,
        ref_blot: Option<&dyn BlotTrait>,
    ) -> Result<(), JsValue> {
        // Attach the child
        child.attach();

        match ref_blot {
            Some(ref_child) => {
                // Find the reference child index
                if let Some(ref_index) = self.find_child_index(ref_child) {
                    // Insert in DOM before reference node
                    self.dom_node
                        .insert_before(child.dom_node(), Some(ref_child.dom_node()))?;

                    // Insert in LinkedList at the correct position
                    self.children.insert_at_ith(ref_index as u32, child);
                } else {
                    return Err("Reference blot not found".into());
                }
            }
            None => {
                // Insert at the end
                return self.append_child(child);
            }
        }

        Ok(())
    }

    /// Remove child - mirrors TypeScript removeChild()
    fn remove_child(&mut self, child: &dyn BlotTrait) -> Result<Box<dyn BlotTrait>, JsValue> {
        if let Some(child_index) = self.find_child_index(child) {
            // Remove from DOM
            self.dom_node.remove_child(child.dom_node())?;

            // Remove from LinkedList
            if let Some(removed_child) = self.children.delete_ith(child_index as u32) {
                return Ok(removed_child);
            }
        }

        Err("Child not found".into())
    }

    /// Find descendant - mirrors TypeScript descendant()
    fn descendant(
        &self,
        matcher: fn(&dyn BlotTrait) -> bool,
        index: Option<usize>,
    ) -> Option<&dyn BlotTrait> {
        let mut current_index = 0;

        // First search direct children
        let mut current_node = self.children.head;
        while let Some(node_ptr) = current_node {
            unsafe {
                let node_ref = node_ptr.as_ref();
                let child = node_ref.val.as_ref();

                if matcher(child) {
                    if let Some(target_index) = index {
                        if current_index == target_index {
                            return Some(child);
                        }
                        current_index += 1;
                    } else {
                        return Some(child);
                    }
                }

                current_node = node_ref.next;
            }
        }

        // Then recursively search in parent children
        let mut current_node = self.children.head;
        while let Some(node_ptr) = current_node {
            unsafe {
                let node_ref = node_ptr.as_ref();
                let child = node_ref.val.as_ref();

                // Try to downcast to ParentBlotTrait
                if let Some(parent_child) = child.as_any().downcast_ref::<ParentBlot>() {
                    if let Some(found) = parent_child
                        .descendant(matcher, index.map(|i| i.saturating_sub(current_index)))
                    {
                        return Some(found);
                    }
                }

                current_node = node_ref.next;
            }
        }

        None
    }

    /// Find descendants - mirrors TypeScript descendants()
    fn descendants(
        &self,
        matcher: fn(&dyn BlotTrait) -> bool,
        index: Option<usize>,
        length: Option<usize>,
    ) -> Vec<&dyn BlotTrait> {
        let mut results = Vec::new();
        self.collect_descendants(&mut results, matcher);

        // Apply index and length filtering
        if let Some(start_index) = index {
            if let Some(take_length) = length {
                results
                    .into_iter()
                    .skip(start_index)
                    .take(take_length)
                    .collect()
            } else {
                results.into_iter().skip(start_index).collect()
            }
        } else {
            results
        }
    }

    /// Get path - mirrors TypeScript path()
    fn path(&self, index: usize) -> Vec<(&dyn BlotTrait, usize)> {
        let mut path = Vec::new();
        let mut current_offset = 0;
        let mut current_index = 0;

        while let Some(child) = self.find_child(current_index) {
            let child_length = child.length();

            if current_offset + child_length > index {
                let child_relative_index = index - current_offset;
                path.push((child, child_relative_index));

                // Recursively get path from child if it's a parent
                if let Some(parent_child) = child.as_any().downcast_ref::<ParentBlot>() {
                    let mut child_path = parent_child.path(child_relative_index);
                    path.append(&mut child_path);
                }

                break;
            }

            current_offset += child_length;
            current_index += 1;
        }

        path
    }

    /// Append text - convenience method
    fn append_text(&mut self, text: &str) -> Result<(), JsValue> {
        if let Ok(text_blot) = crate::blot::text::TextBlot::new(text) {
            self.append_child(Box::new(text_blot))
        } else {
            Err("Failed to create text blot".into())
        }
    }

    /// Clear all children - mirrors TypeScript clear()
    fn clear(&mut self) {
        // Remove all DOM children
        while let Some(child) = self.dom_node.first_child() {
            let _ = self.dom_node.remove_child(&child);
        }

        // Clear LinkedList (Drop will be called automatically)
        self.children = LinkedList::new();
    }

    /// Get text content - mirrors TypeScript textContent()
    fn text_content(&self) -> String {
        self.dom_node.text_content().unwrap_or_default()
    }
}

impl Drop for ParentBlot {
    fn drop(&mut self) {
        // Ensure we unregister from the registry when dropped
        Registry::unregister_blot(&self.dom_node);
    }
}
