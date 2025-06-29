//! Internal state management for the QuillAI Editor.
//!
//! This module provides the core state management functionality for the QuillAI Editor,
//! including document content tracking, selection management, history handling, and
//! integration with external systems.
//!
//! # Architecture
//!
//! The state is managed using Dioxus signals for reactive updates. Each piece of state
//! is independently tracked, allowing for fine-grained reactivity and efficient updates.
//!
//! # State Components
//!
//! - **Document State**: Current content represented as Delta operations
//! - **History State**: Undo/redo stack with position tracking
//! - **Selection State**: Current cursor position and text selection
//! - **UI State**: Focus, readonly mode, placeholder text, and styling
//! - **Configuration State**: Custom shortcuts and editor settings

use dioxus::prelude::*;
use quillai_delta::Delta;
use std::collections::HashMap;

/// Internal state for the QuillAI Editor component.
///
/// This struct manages all the internal state needed for the editor to function,
/// including document content, selection, focus, and integration with external systems.
#[derive(Clone)]
pub struct EditorState {
    /// The document content represented as a Delta.
    pub document: Signal<Delta>,
    
    /// History stack for undo/redo functionality.
    pub history: Signal<Vec<Delta>>,
    
    /// Current position in the history stack.
    pub history_index: Signal<usize>,
    
    /// Current text selection (start, end) indices.
    pub selection: Signal<(usize, usize)>,
    
    /// Whether the editor currently has focus.
    pub focus: Signal<bool>,
    
    /// Whether the editor is in read-only mode.
    pub readonly: Signal<bool>,
    
    /// Placeholder text to display when empty.
    pub placeholder: Signal<Option<String>>,
    
    /// Custom CSS class for styling.
    pub css_class: Signal<Option<String>>,
    
    /// Custom keyboard shortcuts configuration.
    pub custom_shortcuts: Signal<HashMap<String, String>>,
}

impl EditorState {
    /// Create a new EditorState with default values.
    ///
    /// This creates an empty editor state with sensible defaults:
    /// - Empty document
    /// - Single empty delta in history
    /// - No selection (cursor at position 0)
    /// - Not focused, not readonly
    /// - No placeholder or custom styling
    /// - Empty shortcuts configuration
    pub fn new() -> Self {
        let empty_delta = Delta::new();
        Self {
            document: Signal::new(empty_delta.clone()),
            history: Signal::new(vec![empty_delta]),
            history_index: Signal::new(0),
            selection: Signal::new((0, 0)),
            focus: Signal::new(false),
            readonly: Signal::new(false),
            placeholder: Signal::new(None),
            css_class: Signal::new(None),
            custom_shortcuts: Signal::new(HashMap::new()),
        }
    }
    
    /// Initialize state from component props.
    ///
    /// This method creates a new EditorState and configures it based on the
    /// provided component props. It handles initial content parsing, readonly mode,
    /// placeholder text, styling, and custom shortcuts.
    ///
    /// # Arguments
    ///
    /// * `initial_content` - Optional text content to pre-populate the editor
    /// * `readonly` - Whether the editor should be in read-only mode
    /// * `placeholder` - Optional placeholder text for empty editor
    /// * `class` - Optional CSS class for styling
    /// * `custom_shortcuts` - Optional custom keyboard shortcuts
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use quillai_editor::editor::state::EditorState;
    ///
    /// let mut shortcuts = HashMap::new();
    /// shortcuts.insert("ctrl+u".to_string(), "underline".to_string());
    ///
    /// let state = EditorState::from_props(
    ///     Some("Hello, world!".to_string()),
    ///     false,
    ///     Some("Type here...".to_string()),
    ///     Some("my-editor".to_string()),
    ///     Some(shortcuts),
    /// );
    /// ```
    pub fn from_props(
        initial_content: Option<String>,
        readonly: bool,
        placeholder: Option<String>,
        class: Option<String>,
        custom_shortcuts: Option<HashMap<String, String>>,
    ) -> Self {
        let mut state = Self::new();
        
        // Set initial content if provided
        if let Some(content) = initial_content {
            // For now, treat all initial content as plain text
            // TODO: Add Delta JSON parsing support when Delta implements serde
            let delta = Delta::new().insert(content, None);
            
            state.document.set(delta.clone());
            state.history.set(vec![delta]);
        }
        
        // Set other properties
        state.readonly.set(readonly);
        state.placeholder.set(placeholder);
        state.css_class.set(class);
        state.custom_shortcuts.set(custom_shortcuts.unwrap_or_default());
        
        state
    }
    
    /// Get the current document length in characters.
    ///
    /// This method calculates the total length of the document by summing
    /// the lengths of all operations in the current Delta.
    pub fn document_length(&self) -> usize {
        self.document.read().length()
    }
    
    /// Check if the document is empty.
    ///
    /// Returns true if the document contains no content (length is 0).
    pub fn is_empty(&self) -> bool {
        self.document_length() == 0
    }
    
    /// Get the current selection range.
    ///
    /// Returns a tuple of (start, end) indices representing the current selection.
    /// If start == end, it represents a cursor position.
    pub fn get_selection(&self) -> (usize, usize) {
        *self.selection.read()
    }
    
    /// Set the current selection range.
    ///
    /// Updates the selection to the specified range. The range will be clamped
    /// to the document bounds.
    ///
    /// # Arguments
    ///
    /// * `start` - Start index of the selection
    /// * `end` - End index of the selection
    pub fn set_selection(&self, start: usize, end: usize) {
        let doc_length = self.document_length();
        let clamped_start = start.min(doc_length);
        let clamped_end = end.min(doc_length);
        self.selection.set((clamped_start, clamped_end));
    }
    
    /// Set the cursor position.
    ///
    /// This is a convenience method for setting the selection to a single point.
    ///
    /// # Arguments
    ///
    /// * `position` - The cursor position (will be clamped to document bounds)
    pub fn set_cursor(&self, position: usize) {
        self.set_selection(position, position);
    }
    
    /// Check if the editor currently has focus.
    pub fn has_focus(&self) -> bool {
        *self.focus.read()
    }
    
    /// Set the focus state of the editor.
    ///
    /// # Arguments
    ///
    /// * `focused` - Whether the editor should be focused
    pub fn set_focus(&self, focused: bool) {
        self.focus.set(focused);
    }
    
    /// Check if the editor is in readonly mode.
    pub fn is_readonly(&self) -> bool {
        *self.readonly.read()
    }
    
    /// Get the current placeholder text.
    pub fn get_placeholder(&self) -> Option<String> {
        self.placeholder.read().clone()
    }
    
    /// Get the current CSS class.
    pub fn get_css_class(&self) -> Option<String> {
        self.css_class.read().clone()
    }
    
    /// Check if we can undo (history index > 0).
    pub fn can_undo(&self) -> bool {
        *self.history_index.read() > 0
    }
    
    /// Check if we can redo (history index < history length - 1).
    pub fn can_redo(&self) -> bool {
        let history_index = *self.history_index.read();
        let history_length = self.history.read().len();
        history_index < history_length.saturating_sub(1)
    }
    
    /// Get the current document as a Delta.
    ///
    /// Returns a clone of the current document Delta.
    pub fn get_document(&self) -> Delta {
        self.document.read().clone()
    }
    
    /// Update the document with a new Delta.
    ///
    /// This method updates the document and adds the change to the history stack.
    /// It also resets the history index to the end of the stack.
    ///
    /// # Arguments
    ///
    /// * `delta` - The new document state
    pub fn update_document(&self, delta: Delta) {
        // Add to history (truncate any redo history)
        let current_index = *self.history_index.read();
        let mut history = self.history.read().clone();
        
        // Remove any redo history
        history.truncate(current_index + 1);
        
        // Add new state
        history.push(delta.clone());
        
        // Update signals
        self.document.set(delta);
        self.history.set(history);
        self.history_index.set(current_index + 1);
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}