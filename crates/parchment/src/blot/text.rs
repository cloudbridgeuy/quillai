//! Text blot implementation for actual text content in documents
//!
//! TextBlot is the fundamental leaf node type that represents actual text content
//! in the Parchment document model. It wraps DOM text nodes and provides rich
//! text manipulation capabilities including splitting, merging, and cursor management.
//!
//! ## Key Features
//!
//! - **Direct DOM Integration**: Wraps native DOM Text nodes
//! - **Unicode Support**: Proper character-based operations for international text
//! - **Text Operations**: Insert, delete, split, and merge operations
//! - **Selection Management**: Cursor positioning and text selection APIs
//! - **Performance Optimized**: Efficient character-level manipulations
//!
//! ## Usage Examples
//!
//! ```rust,no_run
//! use quillai_parchment::TextBlot;
//!
//! // Create new text content
//! let text = TextBlot::new("Hello, world!")?;
//!
//! // Text manipulation
//! text.insert_at(7, "beautiful ")?;  // "Hello, beautiful world!"
//! text.delete_at(0, 7)?;             // "beautiful world!"
//!
//! // Splitting for editing operations
//! let second_part = text.split(9)?;  // Split at "beautiful"
//! # Ok::<(), wasm_bindgen::JsValue>(())
//! ```

use crate::blot::traits_simple::{BlotTrait, LeafBlotTrait};

use crate::dom::Dom;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::{Node, Text};

/// Text blot representing actual text content in the document
///
/// TextBlot is a leaf node in the document tree that contains actual text content.
/// It wraps a DOM Text node and provides comprehensive text manipulation capabilities
/// including character-level operations, splitting/merging, and selection management.
///
/// # Characteristics
///
/// - **Leaf Node**: Cannot contain child blots (terminal content)
/// - **Unicode Aware**: Handles international characters correctly
/// - **DOM Synchronized**: Changes immediately reflect in the DOM
/// - **Selection Support**: Full cursor and selection management
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::TextBlot;
///
/// // Create text content
/// let mut text = TextBlot::new("Hello")?;
///
/// // Basic operations
/// assert_eq!(text.length(), 5);
/// assert_eq!(text.value(), "Hello");
///
/// // Text editing
/// text.insert_at(5, ", world!")?;
/// assert_eq!(text.value(), "Hello, world!");
///
/// // Character access
/// assert_eq!(text.char_at(0), Some('H'));
/// assert_eq!(text.substring(0, 5), "Hello");
/// # Ok::<(), wasm_bindgen::JsValue>(())
/// ```
#[wasm_bindgen]
pub struct TextBlot {
    /// The underlying DOM text node that stores the actual content
    dom_node: Text,
    /// Cached length to avoid repeated DOM reads
    cached_length: std::cell::RefCell<Option<usize>>,
}

#[wasm_bindgen]
impl TextBlot {
    /// Create a new TextBlot with the given text content
    ///
    /// Creates a new DOM text node with the specified content and wraps it
    /// in a TextBlot for use in the Parchment document model.
    ///
    /// # Parameters
    /// * `content` - Initial text content for the blot
    ///
    /// # Returns
    /// New TextBlot instance on success, JsValue error on DOM creation failure
    ///
    /// # Examples
    /// ```javascript
    /// // From JavaScript after WASM init
    /// const text = new TextBlot("Hello, world!");
    /// console.log(text.value); // "Hello, world!"
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(content: &str) -> Result<TextBlot, JsValue> {
        let dom_node = Dom::create_text_node(content)?;
        Ok(TextBlot { 
            dom_node,
            cached_length: std::cell::RefCell::new(None),
        })
    }

    /// Create a TextBlot from an existing DOM text node
    ///
    /// Wraps an existing DOM text node in a TextBlot. This is typically used
    /// when converting existing DOM content into the Parchment document model.
    ///
    /// # Parameters
    /// * `node` - DOM node to wrap (must be a Text node)
    /// * `_content` - Unused parameter for API compatibility
    ///
    /// # Returns
    /// TextBlot wrapping the DOM node, or error if node is not a Text node
    ///
    /// # Errors
    /// Returns error if the provided node is not a DOM Text node
    pub fn from_node(node: Node, _content: &str) -> Result<TextBlot, JsValue> {
        if let Some(text_node) = node.dyn_ref::<Text>() {
            Ok(TextBlot {
                dom_node: text_node.clone(),
                cached_length: std::cell::RefCell::new(None),
            })
        } else {
            Err("Node is not a Text node".into())
        }
    }

    /// Get the text content of this blot
    ///
    /// Returns the current text content stored in the underlying DOM text node.
    /// This is the primary method for accessing the actual content.
    ///
    /// # Returns
    /// Current text content as a String
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> String {
        self.dom_node.text_content().unwrap_or_default()
    }

    /// Set the text content of this blot
    ///
    /// Updates the text content of the underlying DOM text node. Changes
    /// are immediately reflected in the DOM and visible to users.
    ///
    /// # Parameters
    /// * `content` - New text content to set
    #[wasm_bindgen(setter)]
    pub fn set_value(&self, content: &str) {
        self.dom_node.set_text_content(Some(content));
        self.invalidate_length_cache();
        let _ = self.notify_parent_of_change(); // Ignore errors in setter
    }

    /// Get the text content directly from the DOM text node
    /// 
    /// This method provides explicit DOM access for synchronization purposes.
    /// It's similar to `value()` but with explicit error handling.
    pub fn get_text_content(&self) -> Result<String, JsValue> {
        Ok(self.dom_node.text_content().unwrap_or_default())
    }

    /// Set the text content directly on the DOM text node
    /// 
    /// This method provides explicit DOM access for synchronization purposes.
    /// It updates the DOM, invalidates the length cache, and notifies the parent.
    pub fn set_text_content(&mut self, text: &str) -> Result<(), JsValue> {
        self.dom_node.set_text_content(Some(text));
        self.invalidate_length_cache();
        self.notify_parent_of_change()?;
        Ok(())
    }

    /// Calculate the length based on actual DOM text content
    /// 
    /// This method reads directly from the DOM to get the current length,
    /// which is important for synchronization scenarios where the DOM
    /// might have been modified externally.
    pub fn calculate_length(&self) -> Result<usize, JsValue> {
        let content = self.get_text_content()?;
        Ok(content.chars().count())
    }

    /// Invalidate the cached length, forcing recalculation on next access
    pub fn invalidate_length_cache(&self) {
        *self.cached_length.borrow_mut() = None;
    }

    /// Notify parent blot that this text content has changed
    ///
    /// This method finds the parent blot and notifies it of the content change,
    /// allowing the parent to update its own cached state and propagate the
    /// notification up the tree.
    pub fn notify_parent_of_change(&self) -> Result<(), JsValue> {
        // Find the parent DOM node
        if let Some(parent_node) = self.dom_node.parent_node() {
            // For now, we'll add a simple notification mechanism
            // In a full implementation, this would use the registry to find the parent blot
            // and call notify_child_changed on it
            
            // Log the notification for now (this will be replaced with actual parent notification)
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "TextBlot content changed, notifying parent (DOM node: {})",
                parent_node.node_name()
            )));
        }
        Ok(())
    }

    /// Get the character length of the text content
    ///
    /// Returns the number of Unicode characters (not bytes) in the text content.
    /// This is important for proper cursor positioning and text operations.
    /// Uses caching to avoid repeated DOM reads for performance.
    ///
    /// # Returns
    /// Number of characters in the text content
    pub fn length(&self) -> usize {
        // Check if we have a cached value
        if let Some(cached) = *self.cached_length.borrow() {
            return cached;
        }

        // Calculate and cache the length
        let length = match self.calculate_length() {
            Ok(len) => len,
            Err(_) => {
                // Fallback to the old method if DOM read fails
                self.value().chars().count()
            }
        };

        *self.cached_length.borrow_mut() = Some(length);
        length
    }

    /// Get a clone of the underlying DOM node
    pub fn dom_node(&self) -> Text {
        self.dom_node.clone()
    }

    /// Convert to generic Node for DOM operations
    pub fn as_node(&self) -> Node {
        self.dom_node.clone().into()
    }

    /// Insert text at a specific character index
    ///
    /// Inserts new text at the specified character position, shifting existing
    /// content to the right. Uses Unicode-aware character indexing.
    ///
    /// # Parameters
    /// * `index` - Character position to insert at (0-based)
    /// * `text` - Text content to insert
    ///
    /// # Returns
    /// `Ok(())` on success, `Err(JsValue)` on operation failure
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use quillai_parchment::TextBlot;
    /// let text = TextBlot::new("Hello world")?;
    /// text.insert_at(6, "beautiful ")?;
    /// assert_eq!(text.value(), "Hello beautiful world");
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn insert_at(&self, index: usize, text: &str) -> Result<(), JsValue> {
        let current = self.value();
        let mut chars: Vec<char> = current.chars().collect();

        // Insert the new text at the specified index
        let new_chars: Vec<char> = text.chars().collect();
        for (i, &ch) in new_chars.iter().enumerate() {
            chars.insert(index + i, ch);
        }

        let new_content: String = chars.into_iter().collect();
        self.set_value(&new_content);
        Ok(())
    }

    /// Delete text at a specific index with given length
    pub fn delete_at(&self, index: usize, length: usize) -> Result<(), JsValue> {
        let current = self.value();
        let mut chars: Vec<char> = current.chars().collect();

        // Remove characters from index to index + length
        let end_index = std::cmp::min(index + length, chars.len());
        for _ in index..end_index {
            if index < chars.len() {
                chars.remove(index);
            }
        }

        let new_content: String = chars.into_iter().collect();
        self.set_value(&new_content);
        Ok(())
    }

    /// Split this text blot at the given index, returning the second part
    ///
    /// Divides the text content at the specified character index, keeping the first
    /// part in this blot and returning a new TextBlot containing the second part.
    /// The new blot is automatically inserted into the DOM after this one.
    ///
    /// # Parameters
    /// * `index` - Character position to split at
    ///
    /// # Returns
    /// New TextBlot containing the second part of the split
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use quillai_parchment::TextBlot;
    /// let text = TextBlot::new("Hello world")?;
    /// let second = text.split(6)?;  // Split after "Hello "
    /// assert_eq!(text.value(), "Hello ");
    /// assert_eq!(second.value(), "world");
    /// # Ok::<(), wasm_bindgen::JsValue>(())
    /// ```
    pub fn split(&self, index: usize) -> Result<TextBlot, JsValue> {
        let current = self.value();
        let chars: Vec<char> = current.chars().collect();

        // Handle edge cases
        if index == 0 {
            // Return a copy of the entire text if splitting at beginning
            return TextBlot::new(&current);
        }

        if index >= chars.len() {
            // Return an empty text blot if splitting beyond the end
            return TextBlot::new("");
        }

        // Split the text
        let first_part: String = chars.iter().take(index).collect();
        let second_part: String = chars.iter().skip(index).collect();

        // Update this blot with the first part
        self.set_value(&first_part);

        // Create a new text blot with the second part
        let new_blot = TextBlot::new(&second_part)?;

        // If this text node has a parent, insert the new node after this one
        if let Some(parent) = self.dom_node.parent_node() {
            if let Some(next_sibling) = self.dom_node.next_sibling() {
                // Insert before the next sibling
                parent.insert_before(&new_blot.dom_node, Some(&next_sibling))?;
            } else {
                // Append to parent if this is the last child
                parent.append_child(&new_blot.dom_node)?;
            }
        }

        Ok(new_blot)
    }

    /// Split this text blot at the given index with optional force parameter
    /// Force parameter determines whether to create new node even for boundary cases
    pub fn split_with_force(&self, index: usize, force: bool) -> Result<Option<TextBlot>, JsValue> {
        let current = self.value();
        let chars: Vec<char> = current.chars().collect();

        // Handle edge cases based on force parameter
        if index == 0 && !force {
            return Ok(None);
        }

        if index >= chars.len() && !force {
            return Ok(None);
        }

        // Perform the split
        let split_result = self.split(index)?;
        Ok(Some(split_result))
    }

    /// Merge this text blot with another text blot
    /// 
    /// Combines the content of another TextBlot into this one and removes
    /// the other blot from the DOM. This is a simple content-based merge.
    /// 
    /// # Parameters
    /// * `other` - The TextBlot to merge into this one
    /// 
    /// # Returns
    /// * `Ok(true)` if merge was successful
    /// * `Err(JsValue)` if merge failed due to DOM errors
    pub fn merge(&mut self, other: &TextBlot) -> Result<bool, JsValue> {
        let current_value = self.value();
        let other_value = other.value();

        // Combine the text content
        let merged_content = format!("{}{}", current_value, other_value);
        self.set_value(&merged_content);

        // Remove the other blot's DOM node if it has a parent
        if let Some(parent) = other.dom_node.parent_node() {
            parent.remove_child(&other.dom_node)?;
        }

        // Clear cached length since content changed
        *self.cached_length.borrow_mut() = None;
        Ok(true)
    }



    /// Get the cursor position management helpers
    /// Calculate character offset within the text for cursor positioning
    pub fn get_offset_at_position(&self, position: usize) -> usize {
        let chars: Vec<char> = self.value().chars().collect();
        std::cmp::min(position, chars.len())
    }

    /// Get the character at a specific position
    pub fn char_at(&self, position: usize) -> Option<char> {
        let chars: Vec<char> = self.value().chars().collect();
        chars.get(position).copied()
    }

    /// Get a substring of the text content
    pub fn substring(&self, start: usize, end: usize) -> String {
        let chars: Vec<char> = self.value().chars().collect();
        let actual_end = std::cmp::min(end, chars.len());
        let actual_start = std::cmp::min(start, actual_end);
        chars[actual_start..actual_end].iter().collect()
    }

    // === Selection Management Methods ===

    /// Get the current selection range within this text node
    /// Returns None if no selection exists or selection doesn't intersect this node
    #[wasm_bindgen]
    pub fn get_selection_range(&self) -> Result<Option<Vec<u32>>, JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let selection = window.get_selection()?.ok_or("No selection object")?;

        if selection.range_count() == 0 {
            return Ok(None);
        }

        let range = selection.get_range_at(0)?;

        // Check if the selection intersects with this text node
        let text_node: &Node = self.dom_node.as_ref();

        // Check if this text node is within the selection range
        if range.intersects_node(text_node)? {
            let start_offset = if range.start_container()? == *text_node {
                range.start_offset()?
            } else {
                0
            };

            let end_offset = if range.end_container()? == *text_node {
                range.end_offset()?
            } else {
                self.value().len() as u32
            };

            Ok(Some(vec![start_offset, end_offset]))
        } else {
            Ok(None)
        }
    }

    /// Set selection range within this text node
    #[wasm_bindgen]
    pub fn set_selection_range(&self, start: u32, end: u32) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let selection = window.get_selection()?.ok_or("No selection object")?;
        let document = window.document().ok_or("No document object")?;

        // Create a new range
        let range = document.create_range()?;
        let text_node: &Node = self.dom_node.as_ref();

        // Validate offsets
        let text_length = self.value().len() as u32;
        let actual_start = start.min(text_length);
        let actual_end = end.min(text_length).max(actual_start);

        // Set the range to span from start to end within this text node
        range.set_start(text_node, actual_start)?;
        range.set_end(text_node, actual_end)?;

        // Apply the selection
        selection.remove_all_ranges()?;
        selection.add_range(&range)?;

        Ok(())
    }

    /// Get cursor position relative to this text node
    /// Returns None if cursor is not within this text node
    #[wasm_bindgen]
    pub fn get_cursor_position(&self) -> Result<Option<u32>, JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let selection = window.get_selection()?.ok_or("No selection object")?;

        if selection.range_count() == 0 {
            return Ok(None);
        }

        let range = selection.get_range_at(0)?;

        // Check if selection is collapsed (cursor position)
        if !range.collapsed() {
            return Ok(None);
        }

        let text_node: &Node = self.dom_node.as_ref();

        // Check if cursor is within this text node
        if range.start_container()? == *text_node {
            Ok(Some(range.start_offset()?))
        } else {
            Ok(None)
        }
    }

    /// Set cursor position within this text node
    #[wasm_bindgen]
    pub fn set_cursor_position(&self, position: u32) -> Result<(), JsValue> {
        self.set_selection_range(position, position)
    }

    /// Check if this text node contains the current selection
    #[wasm_bindgen]
    pub fn contains_selection(&self) -> bool {
        matches!(self.get_selection_range(), Ok(Some(_)))
    }

    /// Check if this text node contains the current cursor
    #[wasm_bindgen]
    pub fn contains_cursor(&self) -> bool {
        matches!(self.get_cursor_position(), Ok(Some(_)))
    }

    /// Get the selected text within this text node
    #[wasm_bindgen]
    pub fn get_selected_text(&self) -> Result<String, JsValue> {
        if let Some(range) = self.get_selection_range()? {
            let start = range[0] as usize;
            let end = range[1] as usize;
            Ok(self.substring(start, end))
        } else {
            Ok(String::new())
        }
    }
}

impl BlotTrait for TextBlot {
    fn get_blot_name(&self) -> &'static str {
        "text"
    }

    fn get_tag_name(&self) -> &'static str {
        "#text"
    }

    fn get_scope(&self) -> Scope {
        Scope::InlineBlot
    }

    fn get_class_name(&self) -> Option<&'static str> {
        None
    }

    fn dom_node(&self) -> &Node {
        self.dom_node.as_ref()
    }

    fn length(&self) -> usize {
        self.value().len()
    }

    fn attach(&mut self) {
        // Text blots don't need special attach logic
    }

    fn detach(&mut self) {
        // Text blots don't need special detach logic
    }

    fn remove(&mut self) {
        if let Some(parent) = self.dom_node.parent_node() {
            let _ = parent.remove_child(&self.dom_node);
        }
    }

    fn delete_at(&mut self, index: usize, length: usize) {
        // Delegate to the TextBlot's own delete_at method
        let current = self.value();
        let mut chars: Vec<char> = current.chars().collect();

        // Remove characters from index to index + length
        let end_index = std::cmp::min(index + length, chars.len());
        for _ in index..end_index {
            if index < chars.len() {
                chars.remove(index);
            }
        }

        let new_content: String = chars.into_iter().collect();
        self.dom_node.set_text_content(Some(&new_content));
        self.invalidate_length_cache();
    }

    fn insert_at(&mut self, index: usize, value: &str) {
        // Delegate to the TextBlot's own insert_at method
        let current = self.value();
        let mut chars: Vec<char> = current.chars().collect();

        // Insert the new text at the specified index
        let new_chars: Vec<char> = value.chars().collect();
        for (i, &ch) in new_chars.iter().enumerate() {
            chars.insert(index + i, ch);
        }

        let new_content: String = chars.into_iter().collect();
        self.dom_node.set_text_content(Some(&new_content));
        self.invalidate_length_cache();
    }

    /// Support for downcasting - needed for tree navigation
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Support for mutable downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl LeafBlotTrait for TextBlot {
    fn value(&self) -> String {
        self.value()
    }

    fn set_value(&mut self, value: &str) {
        self.dom_node.set_text_content(Some(value));
        self.invalidate_length_cache();
    }
}

impl TextBlot {
    /// Create a TextBlot from an existing DOM text node
    pub fn from_dom_node(node: Text) -> TextBlot {
        TextBlot { 
            dom_node: node,
            cached_length: std::cell::RefCell::new(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_operations;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_text_content_getter_setter() {
        let text_blot = TextBlot::new("Hello, world!").expect("Failed to create TextBlot");
        
        // Test getter
        let content = text_blot.get_text_content().expect("Failed to get text content");
        assert_eq!(content, "Hello, world!");
        
        // Test value method
        assert_eq!(text_blot.value(), "Hello, world!");
    }

    #[wasm_bindgen_test]
    fn test_length_calculation_and_caching() {
        let text_blot = TextBlot::new("Hello").expect("Failed to create TextBlot");
        
        // Test initial length
        assert_eq!(text_blot.length(), 5);
        
        // Test that multiple calls don't recalculate (cached)
        assert_eq!(text_blot.length(), 5);
        assert_eq!(text_blot.length(), 5);
        
        // Test cache invalidation after content change
        text_blot.set_value("Hello, world!");
        assert_eq!(text_blot.length(), 13);
    }

    #[wasm_bindgen_test]
    fn test_text_synchronization() {
        let mut text_blot = TextBlot::new("old content").expect("Failed to create TextBlot");
        
        // Test synchronization function
        let result = text_operations::sync_text_blot_content(
            &mut text_blot,
            Some("old content"),
            "new content"
        );
        
        assert!(result.is_ok());
        assert_eq!(text_blot.value(), "new content");
        assert_eq!(text_blot.length(), 11);
    }

    #[wasm_bindgen_test]
    fn test_sync_no_change_needed() {
        let mut text_blot = TextBlot::new("same content").expect("Failed to create TextBlot");
        
        // Test that sync with same content doesn't cause issues
        let result = text_operations::sync_text_blot_content(
            &mut text_blot,
            Some("same content"),
            "same content"
        );
        
        assert!(result.is_ok());
        assert_eq!(text_blot.value(), "same content");
    }

    #[wasm_bindgen_test]
    fn test_empty_text_handling() {
        let mut text_blot = TextBlot::new("").expect("Failed to create TextBlot");
        
        assert_eq!(text_blot.length(), 0);
        assert_eq!(text_blot.value(), "");
        
        // Test sync with empty content
        let result = text_operations::sync_text_blot_content(
            &mut text_blot,
            None,
            ""
        );
        
        assert!(result.is_ok());
        assert_eq!(text_blot.length(), 0);
    }

    #[wasm_bindgen_test]
    fn test_unicode_text_handling() {
        let unicode_text = "Hello 世界! 🌍";
        let mut text_blot = TextBlot::new(unicode_text).expect("Failed to create TextBlot");
        
        // Test that Unicode characters are counted correctly
        let expected_length = unicode_text.chars().count();
        assert_eq!(text_blot.length(), expected_length);
        
        // Test sync with Unicode content
        let new_unicode = "新しい内容 🚀";
        let result = text_operations::sync_text_blot_content(
            &mut text_blot,
            Some(unicode_text),
            new_unicode
        );
        
        assert!(result.is_ok());
        assert_eq!(text_blot.value(), new_unicode);
        assert_eq!(text_blot.length(), new_unicode.chars().count());
    }
}
