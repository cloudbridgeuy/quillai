//! Comprehensive text input handling for the QuillAI Editor.
//!
//! This module provides advanced input event handling including keyboard shortcuts,
//! paste operations, text insertion/deletion, and cursor position management.
//! It integrates with the Delta system to maintain document consistency.

use web_sys::KeyboardEvent;
use quillai_delta::Delta;
use std::collections::HashMap;


/// Keyboard event information extracted from DOM events.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardEventInfo {
    /// The key that was pressed (e.g., "Enter", "Backspace", "a")
    pub key: String,
    /// Whether the Ctrl key was held down
    pub ctrl_key: bool,
    /// Whether the Shift key was held down
    pub shift_key: bool,
    /// Whether the Alt key was held down
    pub alt_key: bool,
    /// Whether the Meta key (Cmd on Mac) was held down
    pub meta_key: bool,
    /// The key code for special keys
    pub key_code: u32,
}

impl KeyboardEventInfo {
    /// Create a new KeyboardEventInfo from a web_sys KeyboardEvent.
    pub fn from_keyboard_event(event: &KeyboardEvent) -> Self {
        Self {
            key: event.key(),
            ctrl_key: event.ctrl_key(),
            shift_key: event.shift_key(),
            alt_key: event.alt_key(),
            meta_key: event.meta_key(),
            key_code: event.key_code(),
        }
    }

    /// Check if this represents a formatting shortcut (Ctrl+B, Ctrl+I, etc.).
    pub fn is_formatting_shortcut(&self) -> bool {
        (self.ctrl_key || self.meta_key) && !self.alt_key && matches!(
            self.key.as_str(),
            "b" | "i" | "u" | "s" | "`" | "h"
        )
    }

    /// Check if this is a navigation key (arrow keys, home, end, etc.).
    pub fn is_navigation_key(&self) -> bool {
        matches!(
            self.key.as_str(),
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Home" | "End" | "PageUp" | "PageDown"
        )
    }

    /// Check if this is a text editing key (backspace, delete, enter, etc.).
    pub fn is_editing_key(&self) -> bool {
        matches!(
            self.key.as_str(),
            "Backspace" | "Delete" | "Enter" | "Tab"
        )
    }

    /// Get the shortcut string representation (e.g., "ctrl+b", "cmd+i").
    pub fn to_shortcut_string(&self) -> String {
        let mut parts = Vec::new();
        
        if self.ctrl_key {
            parts.push("ctrl");
        }
        if self.meta_key {
            parts.push("cmd");
        }
        if self.shift_key {
            parts.push("shift");
        }
        if self.alt_key {
            parts.push("alt");
        }
        
        // Convert key to lowercase for consistency
        let lowercase_key = self.key.to_lowercase();
        parts.push(&lowercase_key);
        
        parts.join("+")
    }
}

/// Text input operation types for Delta integration.
#[derive(Debug, Clone, PartialEq)]
pub enum TextOperation {
    /// Insert text at a specific position
    Insert { position: usize, text: String },
    /// Delete text from a specific range
    Delete { start: usize, length: usize },
    /// Replace text in a specific range
    Replace { start: usize, length: usize, text: String },
    /// Format text in a specific range
    Format { start: usize, length: usize, attributes: HashMap<String, String> },
}

impl TextOperation {
    /// Apply this operation to a Delta document.
    /// For Phase 1, we use a simplified approach that recreates the entire document.
    pub fn apply_to_delta(&self, current_text: &str) -> Delta {
        use super::delta_integration::text_to_delta;
        
        match self {
            TextOperation::Insert { position, text } => {
                let mut new_text = current_text.to_string();
                new_text.insert_str(*position, text);
                text_to_delta(&new_text)
            }
            TextOperation::Delete { start, length } => {
                let mut new_text = current_text.to_string();
                let end = (*start + *length).min(new_text.len());
                new_text.drain(*start..end);
                text_to_delta(&new_text)
            }
            TextOperation::Replace { start, length, text } => {
                let mut new_text = current_text.to_string();
                let end = (*start + *length).min(new_text.len());
                new_text.replace_range(*start..end, text);
                text_to_delta(&new_text)
            }
            TextOperation::Format { start: _, length: _, attributes: _ } => {
                // For Phase 1, formatting is not implemented yet
                // Return the original text as delta unchanged
                text_to_delta(current_text)
            }
        }
    }
}

/// Comprehensive input handler for the QuillAI Editor.
pub struct InputHandler {
    /// Custom keyboard shortcuts mapping
    custom_shortcuts: HashMap<String, String>,
    /// Current cursor position
    cursor_position: usize,
    /// Current selection range (start, end)
    selection_range: (usize, usize),
}

impl InputHandler {
    /// Create a new InputHandler with optional custom shortcuts.
    pub fn new(custom_shortcuts: Option<HashMap<String, String>>) -> Self {
        Self {
            custom_shortcuts: custom_shortcuts.unwrap_or_default(),
            cursor_position: 0,
            selection_range: (0, 0),
        }
    }

    /// Handle a keyboard event and return the appropriate text operation.
    pub fn handle_keyboard_event(
        &mut self,
        event_info: &KeyboardEventInfo,
        current_content: &str,
    ) -> Option<TextOperation> {
        // Check for custom shortcuts first
        let shortcut_string = event_info.to_shortcut_string();
        if let Some(_action) = self.custom_shortcuts.get(&shortcut_string) {
            // For Phase 1, we just recognize custom shortcuts but don't act on them
            // This will be expanded in Phase 2
            return None;
        }

        // Handle built-in shortcuts and editing operations
        match event_info.key.as_str() {
            "Backspace" => self.handle_backspace(current_content),
            "Delete" => self.handle_delete(current_content),
            "Enter" => self.handle_enter(),
            "Tab" => self.handle_tab(event_info),
            _ if event_info.is_formatting_shortcut() => {
                // For Phase 1, formatting shortcuts are recognized but not implemented
                None
            }
            _ if event_info.is_navigation_key() => {
                self.handle_navigation(event_info, current_content);
                None
            }
            _ => None,
        }
    }

    /// Handle backspace key press.
    fn handle_backspace(&mut self, _current_content: &str) -> Option<TextOperation> {
        if self.selection_range.0 != self.selection_range.1 {
            // Delete selected text
            let start = self.selection_range.0.min(self.selection_range.1);
            let end = self.selection_range.0.max(self.selection_range.1);
            self.cursor_position = start;
            self.selection_range = (start, start);
            Some(TextOperation::Delete {
                start,
                length: end - start,
            })
        } else if self.cursor_position > 0 {
            // Delete character before cursor
            self.cursor_position -= 1;
            self.selection_range = (self.cursor_position, self.cursor_position);
            Some(TextOperation::Delete {
                start: self.cursor_position,
                length: 1,
            })
        } else {
            None
        }
    }

    /// Handle delete key press.
    fn handle_delete(&mut self, current_content: &str) -> Option<TextOperation> {
        if self.selection_range.0 != self.selection_range.1 {
            // Delete selected text
            let start = self.selection_range.0.min(self.selection_range.1);
            let end = self.selection_range.0.max(self.selection_range.1);
            self.cursor_position = start;
            self.selection_range = (start, start);
            Some(TextOperation::Delete {
                start,
                length: end - start,
            })
        } else if self.cursor_position < current_content.len() {
            // Delete character after cursor
            Some(TextOperation::Delete {
                start: self.cursor_position,
                length: 1,
            })
        } else {
            None
        }
    }

    /// Handle enter key press.
    fn handle_enter(&mut self) -> Option<TextOperation> {
        let operation = Some(TextOperation::Insert {
            position: self.cursor_position,
            text: "\n".to_string(),
        });
        self.cursor_position += 1;
        self.selection_range = (self.cursor_position, self.cursor_position);
        operation
    }

    /// Handle tab key press.
    fn handle_tab(&mut self, event_info: &KeyboardEventInfo) -> Option<TextOperation> {
        if event_info.shift_key {
            // Shift+Tab for unindent (not implemented in Phase 1)
            None
        } else {
            // Tab for indent
            let operation = Some(TextOperation::Insert {
                position: self.cursor_position,
                text: "\t".to_string(),
            });
            self.cursor_position += 1;
            self.selection_range = (self.cursor_position, self.cursor_position);
            operation
        }
    }

    /// Handle navigation keys (arrow keys, home, end, etc.).
    fn handle_navigation(&mut self, event_info: &KeyboardEventInfo, current_content: &str) {
        match event_info.key.as_str() {
            "ArrowLeft" => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            "ArrowRight" => {
                if self.cursor_position < current_content.len() {
                    self.cursor_position += 1;
                }
            }
            "Home" => {
                self.cursor_position = 0;
            }
            "End" => {
                self.cursor_position = current_content.len();
            }
            _ => {}
        }
        
        if !event_info.shift_key {
            // If shift is not held, clear selection
            self.selection_range = (self.cursor_position, self.cursor_position);
        } else {
            // If shift is held, extend selection
            self.selection_range.1 = self.cursor_position;
        }
    }

    /// Handle text insertion from regular typing.
    pub fn handle_text_input(&mut self, text: &str) -> TextOperation {
        let operation = if self.selection_range.0 != self.selection_range.1 {
            // Replace selected text
            let start = self.selection_range.0.min(self.selection_range.1);
            let end = self.selection_range.0.max(self.selection_range.1);
            self.cursor_position = start + text.len();
            self.selection_range = (self.cursor_position, self.cursor_position);
            TextOperation::Replace {
                start,
                length: end - start,
                text: text.to_string(),
            }
        } else {
            // Insert at cursor position
            let operation = TextOperation::Insert {
                position: self.cursor_position,
                text: text.to_string(),
            };
            self.cursor_position += text.len();
            self.selection_range = (self.cursor_position, self.cursor_position);
            operation
        };
        
        operation
    }

    /// Handle paste operations from clipboard.
    pub fn handle_paste(&mut self, pasted_text: &str) -> TextOperation {
        // Paste is essentially the same as text input
        self.handle_text_input(pasted_text)
    }

    /// Update cursor position and selection from external sources.
    pub fn update_selection(&mut self, start: usize, end: usize) {
        self.cursor_position = end;
        self.selection_range = (start, end);
    }

    /// Get the current cursor position.
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Get the current selection range.
    pub fn selection_range(&self) -> (usize, usize) {
        self.selection_range
    }

    /// Check if there is currently selected text.
    pub fn has_selection(&self) -> bool {
        self.selection_range.0 != self.selection_range.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_event_info_creation() {
        // Test shortcut string generation
        let event_info = KeyboardEventInfo {
            key: "b".to_string(),
            ctrl_key: true,
            shift_key: false,
            alt_key: false,
            meta_key: false,
            key_code: 66,
        };
        
        assert_eq!(event_info.to_shortcut_string(), "ctrl+b");
        assert!(event_info.is_formatting_shortcut());
        assert!(!event_info.is_navigation_key());
        assert!(!event_info.is_editing_key());
    }

    #[test]
    fn test_input_handler_creation() {
        let handler = InputHandler::new(None);
        assert_eq!(handler.cursor_position(), 0);
        assert_eq!(handler.selection_range(), (0, 0));
        assert!(!handler.has_selection());
    }

    #[test]
    fn test_text_input_handling() {
        let mut handler = InputHandler::new(None);
        let operation = handler.handle_text_input("Hello");
        
        match operation {
            TextOperation::Insert { position, text } => {
                assert_eq!(position, 0);
                assert_eq!(text, "Hello");
            }
            _ => panic!("Expected Insert operation"),
        }
        
        assert_eq!(handler.cursor_position(), 5);
        assert_eq!(handler.selection_range(), (5, 5));
    }

    #[test]
    fn test_backspace_handling() {
        let mut handler = InputHandler::new(None);
        handler.cursor_position = 5;
        handler.selection_range = (5, 5);
        
        let operation = handler.handle_backspace("Hello");
        
        match operation {
            Some(TextOperation::Delete { start, length }) => {
                assert_eq!(start, 4);
                assert_eq!(length, 1);
            }
            _ => panic!("Expected Delete operation"),
        }
        
        assert_eq!(handler.cursor_position(), 4);
    }

    #[test]
    fn test_selection_deletion() {
        let mut handler = InputHandler::new(None);
        handler.cursor_position = 3;
        handler.selection_range = (1, 4); // "ell" selected in "Hello"
        
        let operation = handler.handle_backspace("Hello");
        
        match operation {
            Some(TextOperation::Delete { start, length }) => {
                assert_eq!(start, 1);
                assert_eq!(length, 3);
            }
            _ => panic!("Expected Delete operation"),
        }
        
        assert_eq!(handler.cursor_position(), 1);
        assert_eq!(handler.selection_range(), (1, 1));
    }

    #[test]
    fn test_enter_handling() {
        let mut handler = InputHandler::new(None);
        handler.cursor_position = 5;
        
        let operation = handler.handle_enter();
        
        match operation {
            Some(TextOperation::Insert { position, text }) => {
                assert_eq!(position, 5);
                assert_eq!(text, "\n");
            }
            _ => panic!("Expected Insert operation"),
        }
        
        assert_eq!(handler.cursor_position(), 6);
    }

    #[test]
    fn test_navigation_handling() {
        let mut handler = InputHandler::new(None);
        handler.cursor_position = 2;
        handler.selection_range = (2, 2);
        
        let event_info = KeyboardEventInfo {
            key: "ArrowRight".to_string(),
            ctrl_key: false,
            shift_key: false,
            alt_key: false,
            meta_key: false,
            key_code: 39,
        };
        
        handler.handle_navigation(&event_info, "Hello");
        
        assert_eq!(handler.cursor_position(), 3);
        assert_eq!(handler.selection_range(), (3, 3));
    }

    #[test]
    fn test_text_operation_delta_application() {
        let current_text = "Hello";
        
        let insert_op = TextOperation::Insert {
            position: 5,
            text: " World".to_string(),
        };
        
        let new_delta = insert_op.apply_to_delta(current_text);
        // The operation should create a new delta
        assert!(!new_delta.ops().is_empty());
    }

    #[test]
    fn test_paste_handling() {
        let mut handler = InputHandler::new(None);
        let operation = handler.handle_paste("Pasted text");
        
        match operation {
            TextOperation::Insert { position, text } => {
                assert_eq!(position, 0);
                assert_eq!(text, "Pasted text");
            }
            _ => panic!("Expected Insert operation"),
        }
        
        assert_eq!(handler.cursor_position(), 11);
    }

    #[test]
    fn test_custom_shortcuts() {
        let mut shortcuts = HashMap::new();
        shortcuts.insert("ctrl+u".to_string(), "underline".to_string());
        
        let mut handler = InputHandler::new(Some(shortcuts));
        
        let event_info = KeyboardEventInfo {
            key: "u".to_string(),
            ctrl_key: true,
            shift_key: false,
            alt_key: false,
            meta_key: false,
            key_code: 85,
        };
        
        // Custom shortcuts are recognized but don't produce operations in Phase 1
        let operation = handler.handle_keyboard_event(&event_info, "Hello");
        assert!(operation.is_none());
    }
}