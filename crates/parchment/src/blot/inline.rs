//! Inline blot implementation for inline formatting and containers
//!
//! InlineBlot represents inline-level elements that provide formatting or grouping
//! within text content. These blots can contain other inline blots or text blots,
//! creating nested formatting structures like bold italic text or linked content.
//!
//! ## Common Use Cases
//!
//! - **Text Formatting**: Bold (`<strong>`), italic (`<em>`), underline (`<u>`)
//! - **Links**: Anchor elements (`<a>`) with href attributes
//! - **Code**: Inline code spans (`<code>`)
//! - **Custom Formatting**: Spans with CSS classes for custom styling
//!
//! ## Nesting Behavior
//!
//! InlineBlots can be nested to create complex formatting:
//! ```html
//! <strong><em>Bold and italic text</em></strong>
//! <a href="..."><strong>Bold link</strong></a>
//! ```
//!
//! ## Examples
//!
//! ```rust
//! use quillai_parchment::{InlineBlot, TextBlot};
//! 
//! // Create bold formatting
//! let bold = InlineBlot::from_element(create_element("strong")?);
//! let text = TextBlot::new("Bold text")?;
//! bold.append_child(Box::new(text))?;
//! ```

use crate::blot::traits_simple::{BlotTrait, ParentBlotTrait};
use crate::collection::linked_list::LinkedList;
use crate::dom::Dom;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement, Node};

/// Inline blot for inline-level formatting and content containers
///
/// InlineBlot represents inline elements that can contain other inline elements
/// or text content. It provides the foundation for text formatting (bold, italic),
/// links, and other inline structures within the document.
///
/// # Characteristics
///
/// - **Inline Flow**: Flows within text content without creating line breaks
/// - **Nestable**: Can contain other inline blots for complex formatting
/// - **Container**: Implements ParentBlotTrait for child management
/// - **Flexible**: Supports any inline HTML element as the underlying DOM node
///
/// # Examples
///
/// ```rust
/// use quillai_parchment::{InlineBlot, TextBlot};
/// 
/// // Create a bold span
/// let mut bold = InlineBlot::new(None)?;  // Creates <span>
/// bold.dom_element().set_tag_name("strong");
/// 
/// // Add text content
/// let text = TextBlot::new("Bold text")?;
/// bold.append_child(Box::new(text))?;
/// 
/// // Create nested formatting
/// let mut italic = InlineBlot::new(None)?;
/// italic.dom_element().set_tag_name("em");
/// bold.append_child(Box::new(italic))?;
/// ```
#[wasm_bindgen]
pub struct InlineBlot {
    /// The underlying DOM element (span, strong, em, a, etc.)
    dom_node: Element,
    /// Child blots managed in a linked list for efficient operations
    children: LinkedList<Box<dyn BlotTrait>>,
}

#[wasm_bindgen]
impl InlineBlot {
    /// Create a new InlineBlot with optional DOM element
    ///
    /// Creates a new inline blot, either wrapping the provided DOM element
    /// or creating a new `<span>` element if none is provided.
    ///
    /// # Parameters
    /// * `element` - Optional DOM element to wrap, creates `<span>` if None
    ///
    /// # Returns
    /// New InlineBlot instance on success, JsValue error on DOM creation failure
    ///
    /// # Examples
    /// ```javascript
    /// // From JavaScript after WASM init
    /// const span = new InlineBlot();  // Creates <span>
    /// const strong = new InlineBlot(document.createElement('strong'));
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(element: Option<Element>) -> Result<InlineBlot, JsValue> {
        let dom_node = match element {
            Some(el) => el,
            None => Dom::create_element("span")?,
        };

        Ok(InlineBlot {
            dom_node,
            children: LinkedList::new(),
        })
    }

    /// Create an InlineBlot from an existing DOM element
    ///
    /// Wraps an existing DOM element in an InlineBlot. This is typically used
    /// when converting existing DOM content into the Parchment document model.
    ///
    /// # Parameters
    /// * `element` - DOM element to wrap (should be inline-level)
    ///
    /// # Returns
    /// New InlineBlot wrapping the provided element
    pub fn from_element(element: Element) -> InlineBlot {
        InlineBlot {
            dom_node: element,
            children: LinkedList::new(),
        }
    }

    /// Create an InlineBlot with initial text content
    ///
    /// Convenience method that creates a new inline blot and immediately
    /// adds the specified text content as a child TextBlot.
    ///
    /// # Parameters
    /// * `text` - Initial text content to add
    ///
    /// # Returns
    /// New InlineBlot containing the text content
    ///
    /// # Examples
    /// ```rust
    /// let bold_text = InlineBlot::with_text("Bold content")?;
    /// assert_eq!(bold_text.text_content(), "Bold content");
    /// ```
    pub fn with_text(text: &str) -> Result<InlineBlot, JsValue> {
        let mut inline = InlineBlot::new(None)?;
        inline.append_text(text)?;
        Ok(inline)
    }

    /// Get the underlying DOM element
    pub fn dom_element(&self) -> Element {
        self.dom_node.clone()
    }

    /// Convert to generic Node for DOM operations
    pub fn as_node(&self) -> Node {
        self.dom_node.clone().into()
    }

    /// Static methods for blot identification (WASM-compatible)
    pub fn blot_name() -> String {
        "inline".to_string()
    }

    pub fn tag_name() -> String {
        "SPAN".to_string()
    }

    pub fn scope() -> Scope {
        Scope::InlineBlot
    }

    /// Check if the blot is empty (has no children)
    pub fn is_empty(&self) -> bool {
        self.children.length == 0
    }

    /// Get the number of children
    pub fn children_count(&self) -> usize {
        self.children.length as usize
    }

    /// Create a bold InlineBlot with text content
    #[wasm_bindgen]
    pub fn create_bold(text: String) -> Result<InlineBlot, JsValue> {
        let strong = Dom::create_element("strong")?;
        let text_node = Dom::create_text_node(&text)?;
        strong.append_child(&text_node)?;
        
        let mut inline = InlineBlot {
            dom_node: strong,
            children: LinkedList::new(),
        };
        
        // Build children from the DOM structure
        inline.build_children()?;
        Ok(inline)
    }

    /// Check if this InlineBlot represents bold formatting
    #[wasm_bindgen]
    pub fn is_bold(&self) -> bool {
        let tag_name = self.dom_node.tag_name().to_lowercase();
        tag_name == "strong" || tag_name == "b"
    }

    /// Create an italic InlineBlot with text content
    #[wasm_bindgen]
    pub fn create_italic(text: String) -> Result<InlineBlot, JsValue> {
        let em = Dom::create_element("em")?;
        let text_node = Dom::create_text_node(&text)?;
        em.append_child(&text_node)?;
        
        let mut inline = InlineBlot {
            dom_node: em,
            children: LinkedList::new(),
        };
        
        // Build children from the DOM structure
        inline.build_children()?;
        Ok(inline)
    }

    /// Check if this InlineBlot represents italic formatting
    #[wasm_bindgen]
    pub fn is_italic(&self) -> bool {
        let tag_name = self.dom_node.tag_name().to_lowercase();
        tag_name == "em" || tag_name == "i"
    }

    /// Create an underlined InlineBlot with text content
    #[wasm_bindgen]
    pub fn create_underline(text: String) -> Result<InlineBlot, JsValue> {
        let u = Dom::create_element("u")?;
        let text_node = Dom::create_text_node(&text)?;
        u.append_child(&text_node)?;
        
        let mut inline = InlineBlot {
            dom_node: u,
            children: LinkedList::new(),
        };
        
        // Build children from the DOM structure
        inline.build_children()?;
        Ok(inline)
    }

    /// Check if this InlineBlot represents underlined formatting
    #[wasm_bindgen]
    pub fn is_underlined(&self) -> bool {
        let tag_name = self.dom_node.tag_name().to_lowercase();
        tag_name == "u"
    }

    /// Create a code InlineBlot with text content
    #[wasm_bindgen]
    pub fn create_code(text: String) -> Result<InlineBlot, JsValue> {
        let code = Dom::create_element("code")?;
        let text_node = Dom::create_text_node(&text)?;
        code.append_child(&text_node)?;
        
        let mut inline = InlineBlot {
            dom_node: code,
            children: LinkedList::new(),
        };
        
        // Build children from the DOM structure
        inline.build_children()?;
        Ok(inline)
    }

    /// Check if this InlineBlot represents code formatting
    #[wasm_bindgen]
    pub fn is_code(&self) -> bool {
        let tag_name = self.dom_node.tag_name().to_lowercase();
        tag_name == "code"
    }

    /// Create a strike-through InlineBlot with text content
    #[wasm_bindgen]
    pub fn create_strike(text: String) -> Result<InlineBlot, JsValue> {
        let s = Dom::create_element("s")?;
        let text_node = Dom::create_text_node(&text)?;
        s.append_child(&text_node)?;
        
        let mut inline = InlineBlot {
            dom_node: s,
            children: LinkedList::new(),
        };
        
        // Build children from the DOM structure
        inline.build_children()?;
        Ok(inline)
    }

    /// Check if this InlineBlot represents strike-through formatting
    #[wasm_bindgen]
    pub fn is_strike(&self) -> bool {
        let tag_name = self.dom_node.tag_name().to_lowercase();
        tag_name == "s" || tag_name == "del" || tag_name == "strike"
    }
}

impl BlotTrait for InlineBlot {
    fn get_blot_name(&self) -> &'static str {
        "inline"
    }

    fn get_tag_name(&self) -> &'static str {
        "SPAN"
    }

    fn get_scope(&self) -> Scope {
        Scope::InlineBlot
    }

    fn dom_node(&self) -> &Node {
        self.dom_node.as_ref()
    }

    fn length(&self) -> usize {
        let mut total = 0;
        for i in 0..self.children.length {
            if let Some(child) = self.children.get(i as i32) {
                total += child.length();
            }
        }
        total
    }

    fn attach(&mut self) {
        // Inline blot attachment logic - register with registry
        // This is called when the blot is added to the document
    }

    fn detach(&mut self) {
        // Inline blot detachment logic - unregister from registry
        // This is called when the blot is removed from the document
    }

    fn remove(&mut self) {
        self.detach();
        if let Some(parent) = self.dom_node.parent_node() {
            let _ = parent.remove_child(&self.dom_node);
        }
    }

    fn delete_at(&mut self, index: usize, length: usize) {
        // Delegate to children based on index
        let mut current_index = 0;
        let mut operations_to_perform = Vec::new();

        for child_index in 0..self.children.length {
            if let Some(child) = self.children.get(child_index as i32) {
                let child_length = child.length();

                if current_index + child_length > index {
                    let child_start = index.saturating_sub(current_index);
                    let child_end = std::cmp::min(child_length, child_start + length);
                    let child_delete_length = child_end - child_start;

                    operations_to_perform.push((child_index, child_start, child_delete_length));

                    if current_index + child_length >= index + length {
                        break;
                    }
                }

                current_index += child_length;
            }
        }

        // Execute operations in reverse order to maintain indices
        for (child_index, child_start, child_delete_length) in operations_to_perform.iter().rev() {
            if let Some(child) = self.children.get_mut(*child_index as i32) {
                child.delete_at(*child_start, *child_delete_length);
            }
        }
    }

    fn insert_at(&mut self, index: usize, value: &str) {
        // For inline blots, we typically insert text at the specified position
        if let Ok(text_node) = Dom::create_text(value) {
            if let Ok(text_blot) = crate::blot::text::TextBlot::from_node(text_node, value) {
                // Find the correct position to insert
                let mut current_index = 0;

                // First pass: find the insertion point
                let mut insert_at_index = None;
                let mut insert_within_child = None;

                for child_index in 0..self.children.length {
                    if let Some(child) = self.children.get(child_index as i32) {
                        let child_length = child.length();

                        if current_index + child_length >= index {
                            if index == current_index {
                                insert_at_index = Some(child_index as i32);
                                break;
                            } else {
                                insert_within_child =
                                    Some((child_index as i32, index - current_index));
                                break;
                            }
                        }

                        current_index += child_length;
                    }
                }

                // Second pass: execute the insertion with proper mutable access
                if let Some(index) = insert_at_index {
                    self.children.insert(index, Box::new(text_blot));
                    return;
                } else if let Some((child_index, offset)) = insert_within_child {
                    if let Some(child) = self.children.get_mut(child_index) {
                        child.insert_at(offset, value);
                        return;
                    }
                }

                // If we reach here, insert at the end
                self.children.push(Box::new(text_blot));
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn build_children(&mut self) -> Result<(), JsValue> {
        // Build children from existing DOM nodes
        let child_nodes = self.dom_node.child_nodes();
        let length = child_nodes.length();

        for i in 0..length {
            if let Some(child_node) = child_nodes.get(i) {
                if let Ok(child_blot) =
                    crate::registry::Registry::create_blot_from_node(&child_node)
                {
                    self.children.push(child_blot);
                }
            }
        }

        Ok(())
    }
}

impl ParentBlotTrait for InlineBlot {
    fn children(&self) -> &LinkedList<Box<dyn BlotTrait>> {
        &self.children
    }

    fn children_mut(&mut self) -> &mut LinkedList<Box<dyn BlotTrait>> {
        &mut self.children
    }

    fn dom_element(&self) -> &HtmlElement {
        self.dom_node.unchecked_ref::<HtmlElement>()
    }

    fn append_child(&mut self, child: Box<dyn BlotTrait>) -> Result<(), JsValue> {
        // Add to DOM
        self.dom_node.append_child(child.dom_node())?;

        // Add to children collection
        self.children.push(child);

        Ok(())
    }

    fn insert_before(
        &mut self,
        child: Box<dyn BlotTrait>,
        ref_blot: Option<&dyn BlotTrait>,
    ) -> Result<(), JsValue> {
        match ref_blot {
            Some(ref_node) => {
                // Insert before the reference node in DOM
                self.dom_node
                    .insert_before(child.dom_node(), Some(ref_node.dom_node()))?;

                // Find the position in children and insert
                if let Some(index) = self.find_child_index(ref_node) {
                    self.children.insert(index, child);
                } else {
                    self.children.push(child);
                }
            }
            None => {
                // Insert at the end (same as append_child)
                return self.append_child(child);
            }
        }

        Ok(())
    }

    fn remove_child(&mut self, child: &dyn BlotTrait) -> Result<Box<dyn BlotTrait>, JsValue> {
        // Remove from DOM
        self.dom_node.remove_child(child.dom_node())?;

        // Remove from children collection
        if let Some(index) = self.find_child_index(child) {
            if let Some(removed_child) = self.children.remove(index) {
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
        let target_index = index.unwrap_or(0);
        let mut found_count = 0;

        for i in 0..self.children.length {
            if let Some(child) = self.children.get(i as i32) {
                if matcher(child.as_ref()) {
                    if found_count == target_index {
                        return Some(child.as_ref());
                    }
                    found_count += 1;
                }

                // Recursively search in parent blots
                if let Some(parent_child) = child.as_any().downcast_ref::<InlineBlot>() {
                    if let Some(found) =
                        parent_child.descendant(matcher, Some(target_index - found_count))
                    {
                        return Some(found);
                    }
                }
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
        let start_index = index.unwrap_or(0);
        let max_length = length.unwrap_or(usize::MAX);

        self.collect_descendants(&mut results, matcher, start_index, max_length);
        results
    }

    fn path(&self, index: usize) -> Vec<(&dyn BlotTrait, usize)> {
        let mut path = vec![(self as &dyn BlotTrait, index)];
        let mut current_index = 0;

        for i in 0..self.children.length {
            if let Some(child) = self.children.get(i as i32) {
                let child_length = child.length();

                if current_index + child_length > index {
                    let child_offset = index - current_index;

                    // If child is a parent, get its path recursively
                    if let Some(parent_child) = child.as_any().downcast_ref::<InlineBlot>() {
                        let child_path = parent_child.path(child_offset);
                        path.extend(child_path);
                    } else {
                        path.push((child.as_ref(), child_offset));
                    }
                    break;
                }

                current_index += child_length;
            }
        }

        path
    }

    fn append_text(&mut self, text: &str) -> Result<(), JsValue> {
        if let Ok(text_node) = Dom::create_text(text) {
            if let Ok(text_blot) = crate::blot::text::TextBlot::from_node(text_node, text) {
                self.append_child(Box::new(text_blot))?;
            }
        }
        Ok(())
    }

    fn clear(&mut self) {
        while self.children.pop().is_some() {
            // Children are automatically dropped
        }

        // Clear DOM children
        while let Some(child) = self.dom_node.first_child() {
            let _ = self.dom_node.remove_child(&child);
        }
    }

    fn text_content(&self) -> String {
        let mut text_parts = Vec::new();

        for i in 0..self.children.length {
            if let Some(child) = self.children.get(i as i32) {
                if let Some(text_blot) =
                    child.as_any().downcast_ref::<crate::blot::text::TextBlot>()
                {
                    text_parts.push(text_blot.value());
                }
            }
        }

        text_parts.join("")
    }
}

impl InlineBlot {
    /// Helper method to find child index by reference
    pub(crate) fn find_child_index(&self, target: &dyn BlotTrait) -> Option<i32> {
        for index in 0..self.children.length {
            if let Some(child) = self.children.get(index as i32) {
                if std::ptr::eq(child.as_ref(), target) {
                    return Some(index as i32);
                }
            }
        }
        None
    }

    /// Helper method to collect descendants recursively
    pub(crate) fn collect_descendants<'a>(
        &'a self,
        results: &mut Vec<&'a dyn BlotTrait>,
        matcher: fn(&dyn BlotTrait) -> bool,
        start_index: usize,
        max_length: usize,
    ) {
        if results.len() >= max_length {
            return;
        }

        let mut current_count = 0;

        for i in 0..self.children.length {
            if let Some(child) = self.children.get(i as i32) {
                if current_count >= start_index
                    && results.len() < max_length
                    && matcher(child.as_ref())
                {
                    results.push(child.as_ref());
                }
                current_count += 1;

                // Recursively collect from parent children
                if let Some(parent_child) = child.as_any().downcast_ref::<InlineBlot>() {
                    let remaining_start = start_index.saturating_sub(current_count);
                    let remaining_length = max_length - results.len();
                    parent_child.collect_descendants(
                        results,
                        matcher,
                        remaining_start,
                        remaining_length,
                    );
                }
            }
        }
    }
}
