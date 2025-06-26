use crate::blot::traits_simple::{BlotTrait, LeafBlotTrait};
use crate::dom::Dom;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::{Node, Text};

/// TextBlot represents a text node in the document
/// This is the fundamental building block for all text content
#[wasm_bindgen]
pub struct TextBlot {
    /// The underlying DOM text node
    dom_node: Text,
}

#[wasm_bindgen]
impl TextBlot {
    /// Create a new TextBlot with the given text content
    #[wasm_bindgen(constructor)]
    pub fn new(content: &str) -> Result<TextBlot, JsValue> {
        let dom_node = Dom::create_text_node(content)?;
        Ok(TextBlot { dom_node })
    }

    /// Create a TextBlot from an existing DOM node
    pub fn from_node(node: Node, _content: &str) -> Result<TextBlot, JsValue> {
        if let Some(text_node) = node.dyn_ref::<Text>() {
            Ok(TextBlot {
                dom_node: text_node.clone(),
            })
        } else {
            Err("Node is not a Text node".into())
        }
    }

    /// Get the text content of this blot
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> String {
        self.dom_node.text_content().unwrap_or_default()
    }

    /// Set the text content of this blot
    #[wasm_bindgen(setter)]
    pub fn set_value(&self, content: &str) {
        self.dom_node.set_text_content(Some(content));
    }

    /// Get the length of the text content
    pub fn length(&self) -> usize {
        self.value().len()
    }

    /// Get a clone of the underlying DOM node
    pub fn dom_node(&self) -> Text {
        self.dom_node.clone()
    }

    /// Convert to generic Node for DOM operations
    pub fn as_node(&self) -> Node {
        self.dom_node.clone().into()
    }

    /// Insert text at a specific index
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
    /// This creates a proper DOM text node split that can be inserted into the parent
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
    /// Returns true if merge was successful
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

        Ok(true)
    }

    /// Check if this TextBlot can merge with another TextBlot
    /// For now, all TextBlots can merge since they have the same formatting
    pub fn can_merge_with(&self, _other: &TextBlot) -> bool {
        // In a more complex implementation, this would check for compatible formatting
        true
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
    }

    /// Support for downcasting - needed for tree navigation
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl LeafBlotTrait for TextBlot {
    fn value(&self) -> String {
        self.value()
    }

    fn set_value(&mut self, value: &str) {
        self.dom_node.set_text_content(Some(value));
    }
}

impl TextBlot {
    /// Create a TextBlot from an existing DOM text node
    pub fn from_dom_node(node: Text) -> TextBlot {
        TextBlot { dom_node: node }
    }
}
