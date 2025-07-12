#![allow(unused_variables)]
use dioxus::prelude::*;
use std::collections::HashMap;

pub mod emitter;
pub mod events;
pub mod handlers;

// Re-export commonly used types for convenience
pub use emitter::{
    DefaultInputHandler, EventContext, EventEmitter, EventPipeline, EventProcessor, InputHandler,
    ShortcutCommand, StandardEventType, Subscription,
};
pub use events::{
    EditorEvent, InputEvent, InputEventCategory, InputValidationError, KeyLocation, Modifiers,
    MouseButton, ValidationError,
};
pub use handlers::{EventHandlers, platform};

/// The main QuillAI Editor component.
///
/// This component provides a rich text editing interface with keyboard-driven formatting.
/// It integrates with the Delta format for document representation and Parchment for
/// DOM state management.
///
/// # Features
///
/// - **Keyboard-driven formatting**: Bold (Ctrl+B), Italic (Ctrl+I), and more
/// - **Real-time change tracking**: Delta-based document representation
/// - **Customizable shortcuts**: Override or add custom keyboard shortcuts
/// - **Read-only mode**: Display content without allowing edits
/// - **Placeholder support**: (Currently not implemented)
/// - **Event callbacks**: React to content and selection changes
///
/// # Basic Example
///
/// ```rust,ignore
/// use dioxus::prelude::*;
/// use quillai_editor::component as quillai_editor_component;
///
/// fn app() -> Element {
///     rsx! {
///         quillai_editor_component {
///             placeholder: Some("Start writing...".to_string()),
///             on_change: move |delta_json: String| {
///                 println!("Document updated: {}", delta_json);
///             },
///             on_selection_change: move |(start, end): (usize, usize)| {
///                 println!("Selection: {} to {}", start, end);
///             }
///         }
///     }
/// }
/// ```
///
/// # Advanced Example
///
/// ```rust,ignore
/// use dioxus::prelude::*;
/// use quillai_editor::component as quillai_editor_component;
/// use std::collections::HashMap;
///
/// fn app() -> Element {
///     let mut custom_shortcuts = HashMap::new();
///     custom_shortcuts.insert("ctrl+u".to_string(), "underline".to_string());
///
///     rsx! {
///         quillai_editor_component {
///             initial_content: Some("Welcome to QuillAI!".to_string()),
///             placeholder: Some("Type your message here...".to_string()),
///             class: Some("my-editor".to_string()),
///             custom_shortcuts: Some(custom_shortcuts),
///             on_change: move |delta| {
///                 // Handle content changes
///                 println!("Document updated: {}", delta);
///             },
///             on_selection_change: move |(start, end)| {
///                 // Handle selection changes
///                 println!("Selection: {} to {}", start, end);
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn component(
    initial_content: Option<String>,
    readonly: Option<bool>,
    placeholder: Option<String>,
    class: Option<String>,
    on_change: Option<EventHandler<String>>,
    on_selection_change: Option<EventHandler<(usize, usize)>>,
    custom_shortcuts: Option<HashMap<String, String>>,
) -> Element {
    // Determine if editor should be readonly
    let is_readonly = readonly.unwrap_or(false);

    // Store initial content
    let initial_text = initial_content.clone().unwrap_or_default();

    // Build the class string
    let class_str = {
        let mut classes = vec!["quillai-editor"];
        if let Some(custom_class) = class.as_ref() {
            classes.push(custom_class);
        }
        classes.join(" ")
    };

    // Default styles for the editor
    let default_styles = r#"
        min-height: 200px;
        padding: 12px;
        border: 1px solid #ddd;
        border-radius: 4px;
        outline: none;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        font-size: 16px;
        line-height: 24px;
        white-space: pre-wrap;
        word-wrap: break-word;
        overflow-wrap: break-word;
        position: relative;
    "#;

    // Additional styles for readonly mode
    let readonly_styles = if is_readonly {
        "background-color: #f5f5f5; cursor: default;"
    } else {
        "background-color: white; cursor: text;"
    };

    // Combine all styles
    let combined_styles = format!("{}{}", default_styles, readonly_styles);

    rsx! {
        div {
            class: "{class_str}",
            style: "{combined_styles}",
            contenteditable: if is_readonly { "false" } else { "true" },

            // Handle input events
            oninput: move |event| {
                let data = event.value();

                // Call the on_change handler if provided
                if let Some(handler) = on_change.as_ref() {
                    handler.call(data);
                }
            },

            // Set initial content
            "{initial_text}"
        }
    }
}

