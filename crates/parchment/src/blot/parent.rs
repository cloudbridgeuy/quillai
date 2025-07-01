//! Parent blot base implementation for container functionality
//!
//! ParentBlot provides the foundational implementation for all blots that can
//! contain child blots. It handles child management, DOM synchronization, and
//! tree navigation operations that are common to all container blots.
//!
//! ## Core Functionality
//!
//! - **Child Management**: Add, remove, and reorder child blots
//! - **DOM Synchronization**: Keep blot tree synchronized with DOM
//! - **Tree Navigation**: Find descendants and compute paths
//! - **Content Operations**: Aggregate child content and operations
//!
//! ## Usage Pattern
//!
//! ParentBlot is typically used as a base for other container blots:
//! - BlockBlot extends ParentBlot for block-level containers
//! - InlineBlot extends ParentBlot for inline containers
//! - ScrollBlot extends ParentBlot for the root document container
//!
//! ## Examples
//!
//! ```rust,no_run
//! use quillai_parchment::blot::{ParentBlot, TextBlot, ParentBlotTrait};
//! use quillai_parchment::dom::Dom;
//!
//! // Create a parent container
//! let dom_node = Dom::create_element("div")?;
//! let mut parent = ParentBlot::new(dom_node.into())?;
//!
//! // Add child content
//! let text = TextBlot::new("Hello, world!")?;
//! parent.append_child(Box::new(text))?;
//!
//! // Access children
//! assert_eq!(parent.children.length, 1);
//! assert_eq!(parent.text_content(), "Hello, world!");
//!
//! # Ok::<(), wasm_bindgen::JsValue>(())
//! ```

use crate::blot::traits_simple::{create_element_with_class, BlotTrait, ParentBlotTrait};
use crate::collection::linked_list::LinkedList;
use crate::registry::Registry;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, Node};

/// Base implementation for all parent blots that contain children
///
/// ParentBlot provides the common functionality needed by all container blots,
/// including child management, DOM operations, and tree navigation. It serves
/// as the foundation for BlockBlot, InlineBlot, and ScrollBlot implementations.
///
/// # Characteristics
///
/// - **Container**: Can hold and manage child blots
/// - **DOM Synchronized**: Maintains consistency between blot tree and DOM
/// - **Efficient**: Uses LinkedList for O(1) insertions and deletions
/// - **Flexible**: Supports any DOM element as the container
///
/// # Child Management
///
/// - Automatic DOM synchronization when children are added/removed
/// - Efficient linked list storage for large numbers of children
/// - Support for ordered insertion and removal operations
/// - Tree navigation and search capabilities
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::blot::{ParentBlot, TextBlot, ParentBlotTrait};
/// use quillai_parchment::dom::Dom;
///
/// // Create container
/// let dom_element = Dom::create_element("div")?;
/// let mut container = ParentBlot::new(dom_element.into())?;
///
/// // Add children
/// let text1 = TextBlot::new("First ")?;
/// let text2 = TextBlot::new("Second")?;
///
/// container.append_child(Box::new(text1))?;
/// container.append_child(Box::new(text2))?;
///
/// assert_eq!(container.children_count(), 2);
/// assert_eq!(container.text_content(), "First Second");
/// # Ok::<(), wasm_bindgen::JsValue>(())
/// ```
pub struct ParentBlot {
    /// The underlying DOM node that contains child elements
    pub dom_node: Node,
    /// Child blots managed in a linked list for efficient operations
    pub children: LinkedList<Box<dyn BlotTrait>>,
}

impl ParentBlot {
    /// Create a new ParentBlot with the given DOM node
    ///
    /// Initializes a new parent blot that wraps the provided DOM node.
    /// The DOM node should be a container element that can hold child elements.
    ///
    /// # Parameters
    /// * `dom_node` - DOM node to wrap (should be an Element)
    ///
    /// # Returns
    /// New ParentBlot instance on success, JsValue error on failure
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use quillai_parchment::blot::ParentBlot;
    /// # use quillai_parchment::dom::Dom;
    /// let div_element = Dom::create_element("div")?;
    /// let parent = ParentBlot::new(div_element.into())?;
    /// assert_eq!(parent.children.length, 0);
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn new(dom_node: Node) -> Result<Self, JsValue> {
        let blot = ParentBlot {
            dom_node,
            children: LinkedList::new(),
        };

        // Note: Registry registration is handled by the caller to avoid
        // circular dependencies and allow for proper initialization

        Ok(blot)
    }

    /// Create a DOM node for a parent blot with specified properties
    ///
    /// Static method that creates a properly configured DOM element for use
    /// as a parent blot container. Handles tag name, CSS class, and initial value.
    ///
    /// # Parameters
    /// * `tag_name` - HTML tag name for the element (e.g., "div", "p", "span")
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
    /// # use quillai_parchment::blot::ParentBlot;
    /// let node = ParentBlot::create_dom_node("div", Some("container"), None)?;
    /// let parent = ParentBlot::new(node)?;
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

    /// Support for mutable downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
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

    fn insert_child_at_position(&mut self, position: usize, child: Box<dyn BlotTrait>) -> Result<(), JsValue> {
        // Validate position bounds
        let children_count = self.children.length as usize;
        if position > children_count {
            return Err(JsValue::from_str(&format!(
                "Position {} out of bounds for {} children",
                position, children_count
            )));
        }

        // Get the child's DOM node before moving it
        let child_dom = child.dom_node().clone();

        // Update DOM to reflect the insertion
        if position == children_count {
            // Append at the end
            self.dom_node.append_child(&child_dom)?;
        } else {
            // Insert before the node at the target position
            let mut current_index = 0;
            let mut current_child = self.dom_node.first_child();
            
            while let Some(node) = current_child {
                if current_index == position {
                    self.dom_node.insert_before(&child_dom, Some(&node))?;
                    break;
                }
                current_index += 1;
                current_child = node.next_sibling();
            }
        }

        // Insert into LinkedList at the specified position
        self.children.insert_at_ith(position as u32, child);

        // Note: ParentBlot's length() method already calculates length dynamically
        // by summing all children's lengths, so no explicit recalculation needed

        Ok(())
    }
}

impl Drop for ParentBlot {
    fn drop(&mut self) {
        // Ensure we unregister from the registry when dropped
        Registry::unregister_blot(&self.dom_node);
    }
}
