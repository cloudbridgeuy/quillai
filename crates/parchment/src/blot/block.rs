use crate::blot::text::TextBlot;
use crate::blot::traits_simple::{BlotTrait, ParentBlotTrait};
use crate::collection::linked_list::LinkedList;
use crate::dom::Dom;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement, Node};

/// BlockBlot represents a block-level element (typically a paragraph)
/// It contains inline content like TextBlots
#[wasm_bindgen]
pub struct BlockBlot {
    /// The underlying DOM element (typically a <p> tag)
    dom_node: Element,
    /// Children collection using LinkedList
    children: LinkedList<Box<dyn BlotTrait>>,
}

#[wasm_bindgen]
impl BlockBlot {
    /// Create a new BlockBlot with optional DOM element
    #[wasm_bindgen(constructor)]
    pub fn new(element: Option<Element>) -> Result<BlockBlot, JsValue> {
        let dom_node = match element {
            Some(el) => el,
            None => Dom::create_element("p")?,
        };

        Ok(BlockBlot {
            dom_node,
            children: LinkedList::new(),
        })
    }

    /// Create a BlockBlot with initial text content
    pub fn with_text(text: &str) -> Result<BlockBlot, JsValue> {
        let mut block = BlockBlot::new(None)?;
        block.append_text(text)?;
        Ok(block)
    }

    /// Get the underlying DOM element
    pub fn dom_element(&self) -> Element {
        self.dom_node.clone()
    }

    /// Convert to generic Node for DOM operations
    pub fn as_node(&self) -> Node {
        self.dom_node.clone().into()
    }

    /// Get the number of child blots
    pub fn children_count(&self) -> usize {
        self.children.length as usize
    }

    /// Check if the block is empty
    pub fn is_empty(&self) -> bool {
        self.children.length == 0
    }

    /// Append text to this block (creates a TextBlot)
    pub fn append_text(&mut self, text: &str) -> Result<(), JsValue> {
        let text_blot = TextBlot::new(text)?;
        let text_node = text_blot.as_node();

        // Append to DOM
        Dom::append_child(&self.as_node(), &text_node)?;

        // Add to children LinkedList
        self.children.insert_at_tail(Box::new(text_blot));

        Ok(())
    }

    /// Insert text at a specific position
    pub fn insert_text_at(&mut self, _index: usize, text: &str) -> Result<(), JsValue> {
        let text_blot = TextBlot::new(text)?;
        let text_node = text_blot.as_node();

        // For now, just append - proper index insertion would require
        // full child management implementation
        Dom::append_child(&self.as_node(), &text_node)?;

        // Add to children LinkedList
        self.children.insert_at_tail(Box::new(text_blot));

        Ok(())
    }

    /// Clear all content from the block
    pub fn clear(&mut self) {
        // Clear DOM children
        while let Some(child) = self.dom_node.first_child() {
            let _ = self.dom_node.remove_child(&child);
        }

        // Clear children LinkedList
        self.children = LinkedList::new();
    }

    /// Get the text content of the entire block
    pub fn text_content(&self) -> String {
        self.dom_node.text_content().unwrap_or_default()
    }

    /// Get the length of text content
    pub fn length(&self) -> usize {
        self.text_content().len()
    }

    /// Find child at specific index - mirrors TypeScript find() method
    fn find_child(&self, index: usize) -> Option<&dyn BlotTrait> {
        self.children.get(index as i32).map(|child| child.as_ref())
    }

    /// Find child index - helper method
    fn find_child_index(&self, child: &dyn BlotTrait) -> Option<usize> {
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
    pub(crate) fn collect_descendants(
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
                if let Some(parent_child) = child
                    .as_any()
                    .downcast_ref::<crate::blot::parent::ParentBlot>()
                {
                    parent_child.collect_descendants(results, matcher);
                } else if let Some(block_child) = child.as_any().downcast_ref::<BlockBlot>() {
                    block_child.collect_descendants(results, matcher);
                } else if let Some(scroll_child) = child
                    .as_any()
                    .downcast_ref::<crate::blot::scroll::ScrollBlot>()
                {
                    scroll_child.collect_descendants(results, matcher);
                }

                current_node = node_ref.next;
            }
        }
    }
}

impl BlotTrait for BlockBlot {
    fn get_blot_name(&self) -> &'static str {
        "block"
    }

    fn get_tag_name(&self) -> &'static str {
        "p"
    }

    fn get_scope(&self) -> Scope {
        Scope::BlockBlot
    }

    fn get_class_name(&self) -> Option<&'static str> {
        None
    }

    fn dom_node(&self) -> &Node {
        self.dom_node.as_ref()
    }

    fn length(&self) -> usize {
        self.text_content().len()
    }

    fn attach(&mut self) {
        // BlockBlot attach logic
    }

    fn detach(&mut self) {
        // BlockBlot detach logic
    }

    fn remove(&mut self) {
        if let Some(parent) = self.dom_node.parent_node() {
            let _ = parent.remove_child(self.dom_node.as_ref());
        }
    }

    fn delete_at(&mut self, index: usize, length: usize) {
        if length == 0 {
            return;
        }

        let mut remaining_length = length;
        let mut current_index = 0;
        let mut operations: Vec<(usize, usize, usize)> = Vec::new(); // (child_index, start_in_child, delete_length)

        // First pass: collect all deletion operations without borrowing
        for child_index in 0..self.children.length {
            if let Some(child) = self.children.get(child_index as i32) {
                let child_length = child.length();

                // Check if deletion starts within this child
                if index >= current_index && index < current_index + child_length {
                    let start_in_child = index - current_index;
                    let delete_in_child =
                        std::cmp::min(remaining_length, child_length - start_in_child);

                    operations.push((child_index as usize, start_in_child, delete_in_child));
                    remaining_length -= delete_in_child;

                    if remaining_length == 0 {
                        break;
                    }
                }
                // Check if deletion continues into this child
                else if index < current_index && remaining_length > 0 {
                    let delete_in_child = std::cmp::min(remaining_length, child_length);

                    operations.push((child_index as usize, 0, delete_in_child));
                    remaining_length -= delete_in_child;

                    if remaining_length == 0 {
                        break;
                    }
                }

                current_index += child_length;
            }
        }

        // Second pass: execute deletions
        let mut children_to_remove: Vec<usize> = Vec::new();
        for (child_index, start_in_child, delete_length) in operations {
            if let Some(child_mut) = self.children.get_mut(child_index as i32) {
                child_mut.delete_at(start_in_child, delete_length);

                // Check if child is now empty after deletion
                if child_mut.length() == 0 {
                    children_to_remove.push(child_index);
                }
            }
        }

        // Remove empty children in reverse order to maintain indices
        for &child_index in children_to_remove.iter().rev() {
            if child_index < self.children.length as usize {
                if let Some(removed_child) = self.children.delete_ith(child_index as u32) {
                    // Remove from DOM as well
                    if let Some(parent) = removed_child.dom_node().parent_node() {
                        let _ = parent.remove_child(removed_child.dom_node());
                    }
                }
            }
        }
    }

    fn insert_at(&mut self, index: usize, value: &str) {
        if value.is_empty() {
            return;
        }

        let mut current_index = 0;

        // Find the appropriate child to insert into
        for child_index in 0..self.children.length {
            if let Some(child) = self.children.get(child_index as i32) {
                let child_length = child.length();

                // Check if insertion point is within this child
                if index >= current_index && index <= current_index + child_length {
                    let insert_in_child = index - current_index;

                    // Get mutable reference for insertion
                    if let Some(child_mut) = self.children.get_mut(child_index as i32) {
                        child_mut.insert_at(insert_in_child, value);
                    }
                    return;
                }

                current_index += child_length;
            }
        }

        // If index is at or beyond the end, append new content
        if index >= current_index {
            let _ = self.append_text(value);
        }
    }

    /// Support for downcasting - needed for tree navigation
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Override build_children to implement recursive DOM traversal for blocks
    fn build_children(&mut self) -> Result<(), JsValue> {
        // Clear existing children
        self.children = LinkedList::new();

        // Traverse DOM children and create corresponding blots
        let child_nodes = self.dom_node.child_nodes();

        for i in 0..child_nodes.length() {
            if let Some(child_node) = child_nodes.get(i) {
                // Create blot from DOM node using Registry
                match crate::registry::Registry::create_blot_from_node(&child_node) {
                    Ok(mut child_blot) => {
                        // Recursively build children
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

impl ParentBlotTrait for BlockBlot {
    fn children(&self) -> &LinkedList<Box<dyn BlotTrait>> {
        &self.children
    }

    fn children_mut(&mut self) -> &mut LinkedList<Box<dyn BlotTrait>> {
        &mut self.children
    }

    fn dom_element(&self) -> &HtmlElement {
        // Safe to unwrap since we know BlockBlot uses an Element
        self.dom_node.dyn_ref::<HtmlElement>().unwrap()
    }

    fn append_child(&mut self, mut child: Box<dyn BlotTrait>) -> Result<(), JsValue> {
        // Attach the child
        child.attach();

        // Add to DOM
        self.dom_node.append_child(child.dom_node())?;

        // Add to LinkedList
        self.children.insert_at_tail(child);

        Ok(())
    }

    fn insert_before(
        &mut self,
        mut child: Box<dyn BlotTrait>,
        ref_blot: Option<&dyn BlotTrait>,
    ) -> Result<(), JsValue> {
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

                // Try to downcast to different parent types
                if let Some(parent_child) = child
                    .as_any()
                    .downcast_ref::<crate::blot::parent::ParentBlot>()
                {
                    if let Some(found) = parent_child
                        .descendant(matcher, index.map(|i| i.saturating_sub(current_index)))
                    {
                        return Some(found);
                    }
                } else if let Some(block_child) = child.as_any().downcast_ref::<BlockBlot>() {
                    if let Some(found) = block_child
                        .descendant(matcher, index.map(|i| i.saturating_sub(current_index)))
                    {
                        return Some(found);
                    }
                } else if let Some(scroll_child) = child
                    .as_any()
                    .downcast_ref::<crate::blot::scroll::ScrollBlot>()
                {
                    if let Some(found) = scroll_child
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

    fn descendants(
        &self,
        matcher: fn(&dyn BlotTrait) -> bool,
        index: Option<usize>,
        length: Option<usize>,
    ) -> Vec<&dyn BlotTrait> {
        let mut results = Vec::new();

        // Collect all descendants using the helper method
        self.collect_descendants(&mut results, matcher);

        // Apply index and length constraints
        match (index, length) {
            (Some(start), Some(len)) => {
                let end = start + len;
                if start < results.len() {
                    results[start..std::cmp::min(end, results.len())].to_vec()
                } else {
                    Vec::new()
                }
            }
            (Some(start), None) => {
                if start < results.len() {
                    results[start..].to_vec()
                } else {
                    Vec::new()
                }
            }
            (None, Some(len)) => results[..std::cmp::min(len, results.len())].to_vec(),
            (None, None) => results,
        }
    }

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
                if let Some(parent_child) = child
                    .as_any()
                    .downcast_ref::<crate::blot::parent::ParentBlot>()
                {
                    let mut child_path = parent_child.path(child_relative_index);
                    path.append(&mut child_path);
                } else if let Some(block_child) = child.as_any().downcast_ref::<BlockBlot>() {
                    let mut child_path = block_child.path(child_relative_index);
                    path.append(&mut child_path);
                } else if let Some(scroll_child) = child
                    .as_any()
                    .downcast_ref::<crate::blot::scroll::ScrollBlot>()
                {
                    let mut child_path = scroll_child.path(child_relative_index);
                    path.append(&mut child_path);
                }

                break;
            }

            current_offset += child_length;
            current_index += 1;
        }

        path
    }

    fn append_text(&mut self, text: &str) -> Result<(), JsValue> {
        let text_blot = TextBlot::new(text)?;
        self.append_child(Box::new(text_blot))
    }

    fn clear(&mut self) {
        // Clear DOM children
        while let Some(child) = self.dom_node.first_child() {
            let _ = self.dom_node.remove_child(&child);
        }

        // Clear children LinkedList
        self.children = LinkedList::new();
    }

    fn text_content(&self) -> String {
        self.dom_node.text_content().unwrap_or_default()
    }
}

impl BlockBlot {
    /// Create a BlockBlot from an existing DOM element
    pub fn from_element(element: Element) -> BlockBlot {
        BlockBlot {
            dom_node: element,
            children: LinkedList::new(),
        }
    }
}
