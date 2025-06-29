//! Main QuillAI Editor component implementation.

use dioxus::prelude::*;
use std::collections::HashMap;
use quillai_delta::Delta;
use super::delta_integration::{text_to_delta, delta_to_text, delta_to_json};
use super::input_handler::{InputHandler, KeyboardEventInfo};
use super::parchment_integration::ParchmentIntegration;

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
/// - **Placeholder support**: Guide users with helpful placeholder text
/// - **Event callbacks**: React to content and selection changes
///
/// # Basic Example
///
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use quillai_editor::QuillAIEditor;
///
/// fn app() -> Element {
///     rsx! {
///         QuillAIEditor {
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
/// ```rust,no_run
/// use dioxus::prelude::*;
/// use quillai_editor::QuillAIEditor;
/// use std::collections::HashMap;
///
/// fn app() -> Element {
///     let mut custom_shortcuts = HashMap::new();
///     custom_shortcuts.insert("ctrl+u".to_string(), "underline".to_string());
///     
///     rsx! {
///         QuillAIEditor {
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
pub fn QuillAIEditor(
    /// Initial content as Delta JSON or plain text.
    ///
    /// This prop allows you to pre-populate the editor with content. The content can be:
    /// - Plain text: Any string that will be inserted as-is
    /// - Delta JSON: A JSON string representing a Delta document (future feature)
    /// 
    /// # Example
    /// ```rust,no_run
    /// use dioxus::prelude::*;
    /// use quillai_editor::QuillAIEditor;
    /// 
    /// fn app() -> Element {
    ///     rsx! {
    ///         QuillAIEditor {
    ///             initial_content: Some("Hello, world!".to_string())
    ///         }
    ///     }
    /// }
    /// ```
    /// 
    /// Note: Delta JSON parsing will be implemented when the Delta crate supports serde.
    #[props(default)]
    initial_content: Option<String>,
    
    /// Whether the editor is read-only.
    ///
    /// When set to `true`, the editor will display content but prevent all editing operations.
    /// Users can still select text and copy content, but cannot modify the document.
    /// This is useful for displaying formatted content or creating read-only previews.
    /// 
    /// # Example
    /// ```rust,ignore
    /// use dioxus::prelude::*;
    /// use quillai_editor::QuillAIEditor;
    /// 
    /// fn app() -> Element {
    ///     rsx! {
    ///         QuillAIEditor {
    ///             readonly: true,
    ///             initial_content: Some("This content cannot be edited".to_string())
    ///         }
    ///     }
    /// }
    /// ```
    #[props(default = false)]
    readonly: bool,
    
    /// Placeholder text when editor is empty.
    ///
    /// This text will be displayed in a muted style when the editor has no content,
    /// providing a hint to users about what they can type. The placeholder disappears
    /// as soon as the user starts typing.
    /// 
    /// # Example
    /// ```rust,no_run
    /// use dioxus::prelude::*;
    /// use quillai_editor::QuillAIEditor;
    /// 
    /// fn app() -> Element {
    ///     rsx! {
    ///         QuillAIEditor {
    ///             placeholder: Some("Start writing your story...".to_string())
    ///         }
    ///     }
    /// }
    /// ```
    #[props(default)]
    placeholder: Option<String>,
    
    /// Custom CSS class for styling.
    ///
    /// This class will be applied to the root editor element, allowing you to customize
    /// the appearance of the editor. You can use this to apply themes, adjust sizing,
    /// or integrate with your application's design system.
    /// 
    /// # Example
    /// ```rust,no_run
    /// use dioxus::prelude::*;
    /// use quillai_editor::QuillAIEditor;
    /// 
    /// fn app() -> Element {
    ///     rsx! {
    ///         QuillAIEditor {
    ///             class: Some("my-custom-editor dark-theme".to_string())
    ///         }
    ///     }
    /// }
    /// ```
    #[props(default)]
    class: Option<String>,
    
    /// Callback when content changes.
    ///
    /// This callback is fired whenever the document content changes,
    /// providing the updated Delta representation as a JSON string.
    /// 
    /// # Example
    /// ```rust,no_run
    /// use dioxus::prelude::*;
    /// use quillai_editor::QuillAIEditor;
    /// 
    /// fn app() -> Element {
    ///     rsx! {
    ///         QuillAIEditor {
    ///             on_change: move |delta_json: String| {
    ///                 println!("Document changed: {}", delta_json);
    ///                 // Parse delta_json if needed for further processing
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    #[props(default)]
    on_change: Option<EventHandler<String>>,
    
    /// Callback when selection changes.
    ///
    /// This callback is fired when the user's text selection or cursor position changes.
    /// The tuple contains (start_index, end_index) where both values represent character
    /// positions in the document. When start_index == end_index, it represents a cursor position.
    /// 
    /// # Example
    /// ```rust,no_run
    /// use dioxus::prelude::*;
    /// use quillai_editor::QuillAIEditor;
    /// 
    /// fn app() -> Element {
    ///     rsx! {
    ///         QuillAIEditor {
    ///             on_selection_change: move |(start, end): (usize, usize)| {
    ///                 if start == end {
    ///                     println!("Cursor at position: {}", start);
    ///                 } else {
    ///                     println!("Selection from {} to {}", start, end);
    ///                 }
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    #[props(default)]
    on_selection_change: Option<EventHandler<(usize, usize)>>,
    
    /// Custom keyboard shortcuts.
    ///
    /// Allows overriding or adding custom keyboard shortcuts beyond the default ones.
    /// The key is a string representation of the shortcut (e.g., "ctrl+b", "cmd+i"),
    /// and the value is the action name (e.g., "bold", "italic").
    /// 
    /// # Shortcut Format
    /// - Use "ctrl" for Control key on Windows/Linux, "cmd" for Command key on Mac
    /// - Use "shift" for Shift key
    /// - Use "alt" for Alt key
    /// - Combine with "+" (e.g., "ctrl+shift+z")
    /// - Single keys: "a", "1", "escape", "enter", "backspace"
    /// 
    /// # Example
    /// ```rust,no_run
    /// use dioxus::prelude::*;
    /// use quillai_editor::QuillAIEditor;
    /// use std::collections::HashMap;
    /// 
    /// fn app() -> Element {
    ///     let mut shortcuts = HashMap::new();
    ///     shortcuts.insert("ctrl+u".to_string(), "underline".to_string());
    ///     shortcuts.insert("ctrl+shift+h".to_string(), "highlight".to_string());
    ///     
    ///     rsx! {
    ///         QuillAIEditor {
    ///             custom_shortcuts: Some(shortcuts)
    ///         }
    ///     }
    /// }
    /// ```
    #[props(default)]
    custom_shortcuts: Option<HashMap<String, String>>,
) -> Element {
    // Initialize editor signals
    let mut document = use_signal(|| {
        if let Some(content) = &initial_content {
            text_to_delta(content)
        } else {
            Delta::new()
        }
    });
    
    let _history = use_signal(|| vec![Delta::new()]);
    let _history_index = use_signal(|| 0usize);
    let mut selection = use_signal(|| (0usize, 0usize));
    let mut focus = use_signal(|| false);
    let readonly_signal = use_signal(|| readonly);
    let placeholder_signal = use_signal(|| placeholder.clone());
    let css_class_signal = use_signal(|| class.clone());
    let _custom_shortcuts_signal = use_signal(|| custom_shortcuts.clone().unwrap_or_default());
    
    // Initialize Parchment integration
    let mut parchment = use_signal(|| ParchmentIntegration::new());
    
    // Initialize input handler
    let mut input_handler = use_signal(|| InputHandler::new(custom_shortcuts.clone()));
    
    // Synchronize Parchment with initial document state
    use_effect(move || {
        let doc = document.read();
        let mut parchment_integration = parchment.write();
        if let Err(_e) = parchment_integration.sync_with_delta(&doc) {
            web_sys::console::warn_1(&"Parchment sync failed".into());
        }
    });
    
    // Get current state values for rendering
    let document_value = document.read();
    let is_empty = document_value.ops().is_empty();
    let placeholder_text = placeholder_signal.read();
    let css_class = css_class_signal.read();
    let is_readonly = readonly_signal.read();
    let has_focus = focus.read();
    
    // Build CSS classes
    let mut classes = vec!["quillai-editor"];
    if let Some(ref custom_class) = css_class.as_ref() {
        classes.push(custom_class);
    }
    if *is_readonly {
        classes.push("readonly");
    }
    if *has_focus {
        classes.push("focused");
    }
    let class_string = classes.join(" ");
    
    // Signals can be captured directly in closures
    
    // Render the editor
    rsx! {
        div {
            class: "{class_string}",
            tabindex: if *is_readonly { -1 } else { 0 },
            
            // Focus handling
            onfocus: move |_| {
                focus.set(true);
            },
            onblur: move |_| {
                focus.set(false);
            },
            
            // Content area
            if is_empty && placeholder_text.is_some() {
                div {
                    class: "quillai-placeholder",
                    "{placeholder_text.as_ref().unwrap()}"
                }
            }
            
            // Document content (simplified for now)
            div {
                class: "quillai-content",
                contenteditable: if *is_readonly { "false" } else { "true" },
                
                // Comprehensive keyboard event handling
                onkeydown: move |event| {
                    let event_data = event.data();
                    let keyboard_event = event_data.downcast::<web_sys::KeyboardEvent>().unwrap();
                    let event_info = KeyboardEventInfo::from_keyboard_event(&keyboard_event);
                    
                    // Get current document content
                    let current_content = delta_to_text(&document.read());
                    
                    // Handle the keyboard event
                    let mut handler = input_handler.write();
                    if let Some(operation) = handler.handle_keyboard_event(&event_info, &current_content) {
                        // Apply the operation to create a new Delta
                        let new_delta = operation.apply_to_delta(&current_content);
                        
                        // Update document state
                        document.set(new_delta.clone());
                        
                        // Update selection state
                        selection.set(handler.selection_range());
                        
                        // Call change handler with Delta JSON representation
                        if let Some(ref handler) = on_change {
                            let delta_json = delta_to_json(&new_delta);
                            handler.call(delta_json);
                        }
                        
                        // Call selection change handler
                        if let Some(ref handler) = on_selection_change {
                            let (start, end) = *selection.read();
                            handler.call((start, end));
                        }
                        
                        // Prevent default browser behavior for handled keys
                        if event_info.is_editing_key() || event_info.is_formatting_shortcut() {
                            keyboard_event.prevent_default();
                        }
                    }
                },
                
                // Text input handling for regular typing
                oninput: move |event| {
                    let new_content = event.value();
                    
                    // For now, we still use the simple approach for regular typing
                    // In a more advanced implementation, we would track the specific
                    // changes and use the input handler for character insertion
                    let new_delta = text_to_delta(&new_content);
                    
                    // Update document state
                    document.set(new_delta.clone());
                    
                    // Update input handler cursor position based on content length
                    let mut handler = input_handler.write();
                    handler.update_selection(new_content.len(), new_content.len());
                    selection.set(handler.selection_range());
                    
                    // Call change handler with Delta JSON representation
                    if let Some(ref handler) = on_change {
                        let delta_json = delta_to_json(&new_delta);
                        handler.call(delta_json);
                    }
                },
                
                // For Phase 1, we'll handle paste through the regular oninput event
                // Advanced paste handling will be implemented in later phases
                
                // Render current document content from Delta
                {
                    delta_to_text(&document_value)
                }
            }
            
            // Debug info (will be removed in later phases)
            if cfg!(debug_assertions) {
                div {
                    class: "quillai-debug",
                    style: "font-size: 10px; color: #666; margin-top: 8px;",
                    "Debug: {document_value.ops().len()} ops, selection: {selection.read().0}-{selection.read().1}"
                }
            }
        }
    }
}

