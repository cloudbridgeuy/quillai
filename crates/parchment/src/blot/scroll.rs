use crate::blot::mutations::MutationObserverWrapper;
use crate::blot::traits_simple::{BlotTrait, ParentBlotTrait};
use crate::collection::linked_list::LinkedList;
use crate::dom::Dom;
use crate::scope::Scope;
use crate::text_operations::{Position, TextMatch, TextSelection, TextStatistics, TextUtils, TextVisitor, TextCollector, TextSearcher};
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement, Node};

/// ScrollBlot is the root blot that represents the entire document
/// It acts as the container for all other blots
#[wasm_bindgen]
pub struct ScrollBlot {
    /// The underlying DOM element (typically a div)
    dom_node: Element,
    /// Children collection using LinkedList
    children: LinkedList<Box<dyn BlotTrait>>,
    /// MutationObserver for tracking DOM changes
    mutation_observer: Option<MutationObserverWrapper>,
}

#[wasm_bindgen]
impl ScrollBlot {
    /// Create a new ScrollBlot with an optional DOM element
    #[wasm_bindgen(constructor)]
    pub fn new(element: Option<Element>) -> Result<ScrollBlot, JsValue> {
        let dom_node = match element {
            Some(el) => el,
            None => Dom::create_element("div")?,
        };

        // Set a class for styling
        dom_node.set_class_name("parchment-scroll");

        Ok(ScrollBlot {
            dom_node,
            children: LinkedList::new(),
            mutation_observer: None,
        })
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

    /// Check if the scroll blot is empty
    pub fn is_empty(&self) -> bool {
        self.children.length == 0
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
                } else if let Some(block_child) = child
                    .as_any()
                    .downcast_ref::<crate::blot::block::BlockBlot>()
                {
                    block_child.collect_descendants(results, matcher);
                } else if let Some(scroll_child) = child.as_any().downcast_ref::<ScrollBlot>() {
                    scroll_child.collect_descendants(results, matcher);
                }

                current_node = node_ref.next;
            }
        }
    }

    /// Insert a paragraph (BlockBlot) with text at the end of the document
    pub fn append_text(&mut self, text: &str) -> Result<(), JsValue> {
        // Create a BlockBlot (paragraph) containing the text
        let block_blot = crate::blot::block::BlockBlot::with_text(text)?;
        let block_node = block_blot.as_node();

        // Append the paragraph to the document
        Dom::append_child(&self.as_node(), &block_node)?;

        // Add to children LinkedList
        self.children.insert_at_tail(Box::new(block_blot));

        Ok(())
    }

    /// Insert a paragraph at a specific position in the document
    pub fn insert_text_at(&mut self, _index: usize, text: &str) -> Result<(), JsValue> {
        // Create a BlockBlot (paragraph) containing the text
        let block_blot = crate::blot::block::BlockBlot::with_text(text)?;
        let block_node = block_blot.as_node();

        // For now, just append - proper index insertion would require
        // full child management implementation
        Dom::append_child(&self.as_node(), &block_node)?;

        // Add to children LinkedList
        self.children.insert_at_tail(Box::new(block_blot));

        Ok(())
    }

    /// Clear all content from the scroll blot
    pub fn clear(&mut self) {
        // Clear DOM children
        while let Some(child) = self.dom_node.first_child() {
            let _ = self.dom_node.remove_child(&child);
        }

        // Clear children LinkedList
        self.children = LinkedList::new();
    }

    /// Get the total text length of all children
    pub fn length(&self) -> usize {
        let mut total_length = 0;
        for child_index in 0..self.children.length {
            if let Some(child) = self.children.get(child_index as i32) {
                total_length += child.length();
            }
        }
        total_length
    }

    /// Get the text content of the entire document
    pub fn text_content(&self) -> String {
        self.dom_node.text_content().unwrap_or_default()
    }

    /// Start observing DOM mutations for this scroll blot
    pub fn start_mutation_observer(&mut self) -> Result<(), JsValue> {
        if self.mutation_observer.is_none() {
            let observer = MutationObserverWrapper::new(self.as_node())?;
            observer.observe()?;
            self.mutation_observer = Some(observer);
        }
        Ok(())
    }

    /// Stop observing DOM mutations
    pub fn stop_mutation_observer(&mut self) {
        if let Some(observer) = &self.mutation_observer {
            observer.disconnect();
        }
        self.mutation_observer = None;
    }

    /// Check if mutation observer is active
    pub fn is_observing_mutations(&self) -> bool {
        self.mutation_observer.is_some()
    }

    // === Document-wide Selection Management ===

    /// Get document-wide selection information
    #[wasm_bindgen]
    pub fn get_document_selection(&self) -> Result<Option<TextSelection>, JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let selection = window.get_selection()?.ok_or("No selection object")?;

        if selection.range_count() == 0 {
            return Ok(None);
        }

        let range = selection.get_range_at(0)?;
        
        // Convert DOM range to our TextSelection format
        let start_path = self.get_path_to_node(&range.start_container()?)?;
        let end_path = self.get_path_to_node(&range.end_container()?)?;

        let text_selection = TextSelection::new(
            start_path,
            range.start_offset()?,
            end_path,
            range.end_offset()?,
        );

        Ok(Some(text_selection))
    }

    /// Set document-wide selection using path-based coordinates
    #[wasm_bindgen]
    pub fn set_document_selection(
        &self,
        start_path: Vec<u32>,
        start_offset: u32,
        end_path: Vec<u32>,
        end_offset: u32,
    ) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let selection = window.get_selection()?.ok_or("No selection object")?;
        let document = window.document().ok_or("No document object")?;

        // Find the nodes at the specified paths
        let start_node = self.get_node_at_path(&start_path)?;
        let end_node = self.get_node_at_path(&end_path)?;

        // Create a new range
        let range = document.create_range()?;
        range.set_start(&start_node, start_offset)?;
        range.set_end(&end_node, end_offset)?;

        // Apply the selection
        selection.remove_all_ranges()?;
        selection.add_range(&range)?;

        Ok(())
    }

    /// Clear current selection
    #[wasm_bindgen]
    pub fn clear_selection(&self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let selection = window.get_selection()?.ok_or("No selection object")?;
        selection.remove_all_ranges()?;
        Ok(())
    }

    /// Get selected text content from the document
    #[wasm_bindgen]
    pub fn get_selected_text(&self) -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let selection = window.get_selection()?.ok_or("No selection object")?;

        if selection.range_count() == 0 {
            return Ok(String::new());
        }

        let range = selection.get_range_at(0)?;
        
        // Extract text content directly from the range
        Ok(range.to_string().into())
    }

    /// Helper method to get path from root to a specific node
    fn get_path_to_node(&self, target_node: &Node) -> Result<Vec<u32>, JsValue> {
        let mut path = Vec::new();
        let mut current_node = target_node.clone();
        let root_node = self.as_node();

        // Traverse up the tree until we reach the root
        while current_node != root_node {
            if let Some(parent) = current_node.parent_node() {
                // Find the index of current_node among its siblings
                let siblings = parent.child_nodes();
                for i in 0..siblings.length() {
                    if let Some(sibling) = siblings.get(i) {
                        if sibling == current_node {
                            path.insert(0, i);
                            break;
                        }
                    }
                }
                current_node = parent;
            } else {
                return Err("Node is not within this ScrollBlot".into());
            }
        }

        Ok(path)
    }

    /// Helper method to get node at a specific path
    fn get_node_at_path(&self, path: &[u32]) -> Result<Node, JsValue> {
        let mut current_node = self.as_node();

        for &index in path {
            let children = current_node.child_nodes();
            if let Some(child) = children.get(index) {
                current_node = child;
            } else {
                return Err(format!("Invalid path: index {} not found", index).into());
            }
        }

        Ok(current_node)
    }

    // === Find and Replace Operations ===

    /// Find all occurrences of a pattern in the document
    #[wasm_bindgen]
    pub fn find_text(&self, pattern: &str, case_sensitive: bool) -> Result<Vec<TextMatch>, JsValue> {
        if pattern.is_empty() {
            return Ok(Vec::new());
        }

        let mut searcher = TextSearcher::new(pattern.to_string(), case_sensitive);
        self.traverse_text_nodes(&mut searcher)?;
        Ok(searcher.matches)
    }

    /// Find next occurrence from current position
    #[wasm_bindgen]
    pub fn find_next(&self, pattern: &str, from_position: Option<Position>) -> Result<Option<TextMatch>, JsValue> {
        let matches = self.find_text(pattern, true)?;
        
        if matches.is_empty() {
            return Ok(None);
        }

        // If no position specified, return first match
        let from_pos = match from_position {
            Some(pos) => pos,
            None => return Ok(matches.into_iter().next()),
        };

        // Find the first match after the specified position
        for text_match in &matches {
            if self.is_position_after(&text_match.start_path(), text_match.start_offset, &from_pos.path(), from_pos.offset) {
                return Ok(Some(text_match.clone()));
            }
        }

        // If no match found after position, wrap around to first match
        Ok(matches.into_iter().next())
    }

    /// Replace single occurrence of pattern
    #[wasm_bindgen]
    pub fn replace_text(&mut self, pattern: &str, replacement: &str, occurrence: Option<u32>) -> Result<bool, JsValue> {
        let matches = self.find_text(pattern, true)?;
        
        if matches.is_empty() {
            return Ok(false);
        }

        let target_index = occurrence.unwrap_or(0) as usize;
        if target_index >= matches.len() {
            return Ok(false);
        }

        let text_match = &matches[target_index];
        self.replace_at_match(text_match, replacement)?;
        Ok(true)
    }

    /// Replace all occurrences of pattern
    #[wasm_bindgen]
    pub fn replace_all(&mut self, pattern: &str, replacement: &str) -> Result<u32, JsValue> {
        let matches = self.find_text(pattern, true)?;
        let count = matches.len() as u32;

        // Replace in reverse order to maintain position validity
        for text_match in matches.iter().rev() {
            self.replace_at_match(text_match, replacement)?;
        }

        Ok(count)
    }

    /// Replace within selection only
    #[wasm_bindgen]
    pub fn replace_in_selection(&mut self, pattern: &str, replacement: &str) -> Result<u32, JsValue> {
        let selection = match self.get_document_selection()? {
            Some(sel) => sel,
            None => return Ok(0),
        };

        let matches = self.find_text(pattern, true)?;
        let mut replaced_count = 0;

        // Filter matches that are within the selection
        for text_match in matches.iter().rev() {
            if self.is_match_in_selection(&text_match, &selection) {
                self.replace_at_match(text_match, replacement)?;
                replaced_count += 1;
            }
        }

        Ok(replaced_count)
    }

    /// Helper method to replace text at a specific match
    fn replace_at_match(&mut self, text_match: &TextMatch, replacement: &str) -> Result<(), JsValue> {
        let node = self.get_node_at_path(&text_match.start_path())?;
        
        if let Some(text_node) = node.dyn_ref::<web_sys::Text>() {
            let current_text = text_node.text_content().unwrap_or_default();
            let start = text_match.start_offset as usize;
            let end = text_match.end_offset as usize;
            
            let mut chars: Vec<char> = current_text.chars().collect();
            
            // Remove the matched text
            for _ in start..end.min(chars.len()) {
                if start < chars.len() {
                    chars.remove(start);
                }
            }
            
            // Insert replacement text
            let replacement_chars: Vec<char> = replacement.chars().collect();
            for (i, &ch) in replacement_chars.iter().enumerate() {
                chars.insert(start + i, ch);
            }
            
            let new_text: String = chars.into_iter().collect();
            text_node.set_text_content(Some(&new_text));
        }

        Ok(())
    }

    /// Helper method to check if a position is after another position
    fn is_position_after(&self, path1: &[u32], offset1: u32, path2: &[u32], offset2: u32) -> bool {
        // Compare paths first
        for (_i, (&p1, &p2)) in path1.iter().zip(path2.iter()).enumerate() {
            if p1 > p2 {
                return true;
            } else if p1 < p2 {
                return false;
            }
        }

        // If paths are equal up to the shorter length, compare by length
        if path1.len() > path2.len() {
            return true;
        } else if path1.len() < path2.len() {
            return false;
        }

        // If paths are identical, compare offsets
        offset1 > offset2
    }

    /// Helper method to check if a match is within a selection
    fn is_match_in_selection(&self, text_match: &TextMatch, selection: &TextSelection) -> bool {
        // Simplified check - in a full implementation, this would need more sophisticated path comparison
        text_match.start_path() == selection.start_path() && 
        text_match.start_offset >= selection.start_offset &&
        text_match.end_offset <= selection.end_offset
    }

    // === Text Statistics ===

    /// Count total words in document
    #[wasm_bindgen]
    pub fn word_count(&self) -> u32 {
        let text = self.collect_all_text();
        TextUtils::count_words(&text)
    }

    /// Count characters with option to include/exclude spaces
    #[wasm_bindgen]
    pub fn character_count(&self, include_spaces: bool) -> u32 {
        let text = self.collect_all_text();
        TextUtils::count_characters(&text, include_spaces)
    }

    /// Count paragraphs (block-level elements)
    #[wasm_bindgen]
    pub fn paragraph_count(&self) -> u32 {
        let mut count = 0;
        let mut current_node = self.children.head;

        while let Some(node_ptr) = current_node {
            unsafe {
                let node_ref = node_ptr.as_ref();
                let child = node_ref.val.as_ref();

                // Count block-level elements as paragraphs
                if matches!(child.get_scope(), Scope::BlockBlot) {
                    count += 1;
                }

                current_node = node_ref.next;
            }
        }

        count.max(1) // At least 1 paragraph even if empty
    }

    /// Get comprehensive text statistics
    #[wasm_bindgen]
    pub fn get_statistics(&self) -> TextStatistics {
        let text = self.collect_all_text();
        
        let words = TextUtils::count_words(&text);
        let characters = TextUtils::count_characters(&text, true);
        let characters_no_spaces = TextUtils::count_characters(&text, false);
        let paragraphs = self.paragraph_count();
        let lines = TextUtils::estimate_lines(&text);
        let sentences = TextUtils::count_sentences(&text);

        TextStatistics::new(
            words,
            characters,
            characters_no_spaces,
            paragraphs,
            lines,
            sentences,
        )
    }

    /// Collect all text content from the document
    fn collect_all_text(&self) -> String {
        let mut collector = TextCollector::new();
        if let Err(_) = self.traverse_text_nodes(&mut collector) {
            // Fallback to DOM text content if traversal fails
            return self.text_content();
        }
        collector.collected_text
    }

    /// Traverse all text nodes in the document using the visitor pattern
    fn traverse_text_nodes(&self, visitor: &mut dyn TextVisitor) -> Result<(), JsValue> {
        self.traverse_text_nodes_recursive(&self.as_node(), &mut Vec::new(), visitor)
    }

    /// Recursive helper for text node traversal
    fn traverse_text_nodes_recursive(
        &self,
        node: &Node,
        current_path: &mut Vec<u32>,
        visitor: &mut dyn TextVisitor,
    ) -> Result<(), JsValue> {
        // Check if this is a text node
        if node.node_type() == Node::TEXT_NODE {
            if let Some(text_content) = node.text_content() {
                if !text_content.trim().is_empty() {
                    visitor.visit_text(&text_content, current_path);
                }
            }
        } else {
            // Traverse child nodes
            let children = node.child_nodes();
            for i in 0..children.length() {
                if let Some(child) = children.get(i) {
                    current_path.push(i);
                    self.traverse_text_nodes_recursive(&child, current_path, visitor)?;
                    current_path.pop();
                }
            }
        }

        Ok(())
    }
}

impl BlotTrait for ScrollBlot {
    fn get_blot_name(&self) -> &'static str {
        "scroll"
    }

    fn get_tag_name(&self) -> &'static str {
        "div"
    }

    fn get_scope(&self) -> Scope {
        Scope::BlockBlot
    }

    fn get_class_name(&self) -> Option<&'static str> {
        Some("parchment-scroll")
    }

    fn dom_node(&self) -> &Node {
        self.dom_node.as_ref()
    }

    fn length(&self) -> usize {
        let mut total_length = 0;
        for child_index in 0..self.children.length {
            if let Some(child) = self.children.get(child_index as i32) {
                total_length += child.length();
            }
        }
        total_length
    }

    fn attach(&mut self) {
        // Start mutation observer when ScrollBlot is attached
        if let Err(_e) = self.start_mutation_observer() {
            // Log error in development
            #[cfg(debug_assertions)]
            web_sys::console::warn_2(&JsValue::from_str("Failed to start mutation observer:"), &_e);
        }
    }

    fn detach(&mut self) {
        // Stop mutation observer when ScrollBlot is detached
        self.stop_mutation_observer();
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

    /// Override build_children to implement recursive DOM traversal for scroll blot
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

impl ParentBlotTrait for ScrollBlot {
    fn children(&self) -> &LinkedList<Box<dyn BlotTrait>> {
        &self.children
    }

    fn children_mut(&mut self) -> &mut LinkedList<Box<dyn BlotTrait>> {
        &mut self.children
    }

    fn dom_element(&self) -> &HtmlElement {
        // Safe to unwrap since we know ScrollBlot uses an Element
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
                } else if let Some(block_child) = child
                    .as_any()
                    .downcast_ref::<crate::blot::block::BlockBlot>()
                {
                    if let Some(found) = block_child
                        .descendant(matcher, index.map(|i| i.saturating_sub(current_index)))
                    {
                        return Some(found);
                    }
                } else if let Some(scroll_child) = child.as_any().downcast_ref::<ScrollBlot>() {
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
                } else if let Some(block_child) = child
                    .as_any()
                    .downcast_ref::<crate::blot::block::BlockBlot>()
                {
                    let mut child_path = block_child.path(child_relative_index);
                    path.append(&mut child_path);
                } else if let Some(scroll_child) = child.as_any().downcast_ref::<ScrollBlot>() {
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
        // Create a BlockBlot (paragraph) containing the text
        let block_blot = crate::blot::block::BlockBlot::with_text(text)?;
        self.append_child(Box::new(block_blot))
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

impl ScrollBlot {
    /// Create a ScrollBlot from an existing DOM element
    pub fn from_element(element: Element) -> ScrollBlot {
        ScrollBlot {
            dom_node: element,
            children: LinkedList::new(),
            mutation_observer: None,
        }
    }
}
