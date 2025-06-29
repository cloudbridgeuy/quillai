//! Internal state management for the QuillAI Editor.

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
    pub fn new() -> Self {
        Self {
            document: Signal::new(Delta::new()),
            history: Signal::new(vec![Delta::new()]),
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
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}