//! Event system for the QuillAI editor.
//!
//! This module defines the core event types that represent all possible editor operations.
//! The event-driven architecture allows for clean separation between user input, editor logic,
//! and document state management.
//!
//! # Architecture
//!
//! The event system follows this flow:
//! ```text
//! User Input → InputEvent → Processing → EditorEvent → Delta Operations → Document State
//! ```
//!
//! # Event Types
//!
//! ## Input Events
//! - [`InputEvent`] - Raw user input events (keyboard, mouse, composition, clipboard)
//! - [`InputEvent::KeyDown`] / [`InputEvent::KeyUp`] - Low-level keyboard events
//! - [`InputEvent::KeyPress`] - High-level text input events
//! - [`InputEvent::MouseClick`] / [`InputEvent::MouseSelect`] - Mouse interaction events
//! - [`InputEvent::CompositionStart`] / [`InputEvent::CompositionEnd`] - IME input events
//! - [`InputEvent::Paste`] / [`InputEvent::Cut`] / [`InputEvent::Copy`] - Clipboard events
//!
//! ## Editor Events
//! - [`EditorEvent::TextInsert`] - Represents text being added to the document
//! - [`EditorEvent::TextDelete`] - Represents text being removed from the document
//! - [`EditorEvent::SelectionChange`] - Represents changes to the selected text range
//! - [`EditorEvent::CursorMove`] - Represents cursor position changes (collapsed selection)
//!
//! # Integration with Delta
//!
//! Each `EditorEvent` can be converted into corresponding Delta operations, which are then
//! used to update the document state. This ensures consistency between the UI and the
//! underlying document representation.
//!
//! # Example
//!
//! ```rust
//! use quillai_editor::events::EditorEvent;
//!
//! // Create a text insertion event
//! let insert_event = EditorEvent::TextInsert {
//!     position: 0,
//!     text: "Hello, world!".to_string(),
//! };
//!
//! // Create a selection event
//! let selection_event = EditorEvent::SelectionChange {
//!     start: 0,
//!     end: 5,
//! };
//!
//! // Events can be serialized for transmission or storage
//! let json = serde_json::to_string(&insert_event).unwrap();
//! println!("Serialized event: {}", json);
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// Errors that can occur during event validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The text field is empty when it should contain content.
    EmptyText,
    /// The deletion length is zero.
    ZeroLengthDelete,
    /// The selection start is greater than the end position.
    InvalidSelectionRange { start: usize, end: usize },
    /// Position or range would cause integer overflow.
    PositionOverflow,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::EmptyText => write!(f, "Text content cannot be empty"),
            ValidationError::ZeroLengthDelete => {
                write!(f, "Delete length must be greater than zero")
            }
            ValidationError::InvalidSelectionRange { start, end } => {
                write!(
                    f,
                    "Invalid selection range: start ({}) is greater than end ({})",
                    start, end
                )
            }
            ValidationError::PositionOverflow => {
                write!(f, "Position or range would cause integer overflow")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// EditorEvent represents all semantic editor operations that can occur within the QuillAI editor.
/// These events form the foundation of the event-driven architecture, providing a type-safe
/// way to handle all editor state changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EditorEvent {
    /// Represents text insertion at a specific position in the document.
    ///
    /// This event is triggered when:
    /// - User types characters
    /// - Text is pasted
    /// - Text is inserted programmatically
    ///
    /// # Fields
    /// * `position` - The zero-based index where the text should be inserted (0 to document length)
    /// * `text` - The text content to insert (can be single or multiple characters)
    ///
    /// # Delta Conversion
    /// This event converts to a Delta `insert` operation at the specified position.
    ///
    /// # Example
    /// ```
    /// # use quillai_editor::events::EditorEvent;
    /// let event = EditorEvent::TextInsert {
    ///     position: 5,
    ///     text: "Hello".to_string(),
    /// };
    /// // This would insert "Hello" at position 5 in the document
    /// ```
    TextInsert { position: usize, text: String },

    /// Represents text deletion from the document.
    ///
    /// This event is triggered when:
    /// - User presses Delete or Backspace
    /// - Text is cut or deleted via selection
    /// - Text is removed programmatically
    ///
    /// # Fields
    /// * `start` - The zero-based starting position of the deletion (0 to document length)
    /// * `length` - The number of characters to delete (must be > 0)
    ///
    /// # Delta Conversion
    /// This event converts to a Delta `delete` operation with the specified length.
    ///
    /// # Example
    /// ```
    /// # use quillai_editor::events::EditorEvent;
    /// let event = EditorEvent::TextDelete {
    ///     start: 10,
    ///     length: 5,
    /// };
    /// // This would delete 5 characters starting from position 10
    /// ```
    TextDelete { start: usize, length: usize },

    /// Represents a change in text selection within the editor.
    ///
    /// This event is triggered when:
    /// - User clicks to position cursor
    /// - User drags to select text
    /// - User uses keyboard shortcuts (Shift+Arrow keys)
    /// - Selection is changed programmatically
    ///
    /// # Fields
    /// * `start` - The zero-based starting position of the selection (0 to document length)
    /// * `end` - The zero-based ending position of the selection (inclusive, start <= end)
    ///
    /// # Special Cases
    /// - When `start` equals `end`, this represents a collapsed selection (cursor position)
    /// - The selection is always normalized so that `start` <= `end`
    ///
    /// # Example
    /// ```
    /// # use quillai_editor::events::EditorEvent;
    /// // Select text from position 5 to 10
    /// let event = EditorEvent::SelectionChange {
    ///     start: 5,
    ///     end: 10,
    /// };
    ///
    /// // Collapsed selection (cursor at position 5)
    /// let cursor_event = EditorEvent::SelectionChange {
    ///     start: 5,
    ///     end: 5,
    /// };
    /// ```
    SelectionChange { start: usize, end: usize },

    /// Represents cursor movement to a specific position.
    ///
    /// This event is triggered when:
    /// - User clicks at a specific position
    /// - User navigates with arrow keys (without selection)
    /// - Cursor is moved programmatically
    ///
    /// # Fields
    /// * `position` - The zero-based position where the cursor should move (0 to document length)
    ///
    /// # Relationship to SelectionChange
    /// This is a convenience variant equivalent to `SelectionChange` where `start` equals `end`.
    /// It's provided for clarity when dealing with cursor-only movements.
    ///
    /// # Example
    /// ```
    /// # use quillai_editor::events::EditorEvent;
    /// let event = EditorEvent::CursorMove {
    ///     position: 15,
    /// };
    /// // This is equivalent to:
    /// // EditorEvent::SelectionChange { start: 15, end: 15 }
    /// ```
    CursorMove { position: usize },
}

impl EditorEvent {
    /// Validates that the event contains valid data.
    ///
    /// # Returns
    /// - `Ok(())` if the event is valid
    /// - `Err(ValidationError)` if the event contains invalid data
    ///
    /// # Validation Rules
    /// - `TextInsert`: Text must not be empty
    /// - `TextDelete`: Length must be greater than zero, start + length must not overflow
    /// - `SelectionChange`: Start must be less than or equal to end
    /// - `CursorMove`: No specific validation (all positions are valid)
    ///
    /// # Example
    /// ```
    /// # use quillai_editor::events::{EditorEvent, ValidationError};
    /// let valid_event = EditorEvent::TextInsert {
    ///     position: 0,
    ///     text: "Hello".to_string(),
    /// };
    /// assert!(valid_event.validate().is_ok());
    ///
    /// let invalid_event = EditorEvent::TextInsert {
    ///     position: 0,
    ///     text: "".to_string(),
    /// };
    /// assert_eq!(invalid_event.validate(), Err(ValidationError::EmptyText));
    /// ```
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self {
            EditorEvent::TextInsert { text, .. } => {
                if text.is_empty() {
                    Err(ValidationError::EmptyText)
                } else {
                    Ok(())
                }
            }
            EditorEvent::TextDelete { start, length } => {
                if *length == 0 {
                    Err(ValidationError::ZeroLengthDelete)
                } else if start.checked_add(*length).is_none() {
                    Err(ValidationError::PositionOverflow)
                } else {
                    Ok(())
                }
            }
            EditorEvent::SelectionChange { start, end } => {
                if start > end {
                    Err(ValidationError::InvalidSelectionRange {
                        start: *start,
                        end: *end,
                    })
                } else {
                    Ok(())
                }
            }
            EditorEvent::CursorMove { .. } => {
                // All cursor positions are valid
                Ok(())
            }
        }
    }

    /// Returns the range of text affected by this event.
    ///
    /// # Returns
    /// - `Some((start, end))` for events that affect a text range
    /// - `None` for events that don't affect text (like cursor movement)
    ///
    /// # Example
    /// ```
    /// # use quillai_editor::events::EditorEvent;
    /// let insert = EditorEvent::TextInsert {
    ///     position: 5,
    ///     text: "Hello".to_string(),
    /// };
    /// assert_eq!(insert.get_affected_range(), Some((5, 10)));
    ///
    /// let delete = EditorEvent::TextDelete {
    ///     start: 3,
    ///     length: 4,
    /// };
    /// assert_eq!(delete.get_affected_range(), Some((3, 7)));
    /// ```
    pub fn get_affected_range(&self) -> Option<(usize, usize)> {
        match self {
            EditorEvent::TextInsert { position, text } => Some((*position, position + text.len())),
            EditorEvent::TextDelete { start, length } => Some((*start, start + length)),
            EditorEvent::SelectionChange { start, end } => Some((*start, *end)),
            EditorEvent::CursorMove { .. } => None,
        }
    }

    /// Checks if this event affects a specific position in the document.
    ///
    /// # Arguments
    /// * `pos` - The position to check
    ///
    /// # Returns
    /// `true` if the event affects the given position, `false` otherwise
    ///
    /// # Example
    /// ```
    /// # use quillai_editor::events::EditorEvent;
    /// let event = EditorEvent::TextDelete {
    ///     start: 5,
    ///     length: 10,
    /// };
    /// assert!(event.affects_position(7));  // Within the delete range
    /// assert!(!event.affects_position(20)); // Outside the delete range
    /// ```
    pub fn affects_position(&self, pos: usize) -> bool {
        match self.get_affected_range() {
            Some((start, end)) => pos >= start && pos < end,
            None => false,
        }
    }

    /// Returns a string representation of the event type for logging purposes.
    ///
    /// # Example
    /// ```
    /// # use quillai_editor::events::EditorEvent;
    /// let event = EditorEvent::TextInsert {
    ///     position: 0,
    ///     text: "Hi".to_string(),
    /// };
    /// assert_eq!(event.event_type(), "TextInsert");
    /// ```
    pub fn event_type(&self) -> &'static str {
        match self {
            EditorEvent::TextInsert { .. } => "TextInsert",
            EditorEvent::TextDelete { .. } => "TextDelete",
            EditorEvent::SelectionChange { .. } => "SelectionChange",
            EditorEvent::CursorMove { .. } => "CursorMove",
        }
    }
}

/// Represents modifier keys that can be pressed during input events.
/// 
/// This struct captures the state of modifier keys at the time an input event occurs.
/// It's designed to work across different platforms while handling platform-specific
/// conventions (e.g., Cmd on macOS vs Ctrl on Windows/Linux).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifiers {
    /// Control key (Ctrl on Windows/Linux, Control on macOS)
    pub ctrl: bool,
    /// Alt key (Alt on Windows/Linux, Option on macOS)
    pub alt: bool,
    /// Shift key
    pub shift: bool,
    /// Meta key (Windows key on Windows, Cmd key on macOS, Super key on Linux)
    pub meta: bool,
}

impl Modifiers {
    /// Creates a new Modifiers instance with all modifiers set to false.
    pub fn none() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }

    /// Checks if any modifier key is pressed.
    pub fn any(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.meta
    }

    /// Returns the primary modifier for the current platform.
    /// On macOS, this is Meta (Cmd), on other platforms it's Ctrl.
    #[cfg(target_os = "macos")]
    pub fn primary(&self) -> bool {
        self.meta
    }

    #[cfg(not(target_os = "macos"))]
    pub fn primary(&self) -> bool {
        self.ctrl
    }
}

impl Default for Modifiers {
    fn default() -> Self {
        Self::none()
    }
}

/// Represents which mouse button was pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MouseButton {
    /// Left mouse button (primary button)
    Left,
    /// Right mouse button (secondary button, typically opens context menu)
    Right,
    /// Middle mouse button (wheel button)
    Middle,
    /// Additional mouse button (e.g., back button)
    Button4,
    /// Additional mouse button (e.g., forward button)
    Button5,
}

/// Represents the location of a key on the keyboard.
/// 
/// This is useful for distinguishing between keys that appear multiple times
/// on a keyboard (e.g., left vs right Shift, numpad vs main keyboard numbers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyLocation {
    /// Key is in the standard location (most keys)
    Standard,
    /// Key is on the left side of the keyboard (e.g., left Shift, left Ctrl)
    Left,
    /// Key is on the right side of the keyboard (e.g., right Shift, right Ctrl)
    Right,
    /// Key is on the numeric keypad
    Numpad,
}

impl Default for KeyLocation {
    fn default() -> Self {
        Self::Standard
    }
}

/// Categories of input events for classification and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEventCategory {
    /// Keyboard-related events (key press, key down, key up)
    Keyboard,
    /// Mouse-related events (click, move, select)
    Mouse,
    /// Text composition events (IME input)
    Composition,
    /// Clipboard-related events (cut, copy, paste)
    Clipboard,
}

/// Errors that can occur during input event validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputValidationError {
    /// The key field is empty when it should contain a value.
    EmptyKey,
    /// The paste text is empty.
    EmptyPasteText,
    /// The composition data is empty when it should contain text.
    EmptyCompositionData,
    /// Invalid mouse selection range (start > end).
    InvalidSelectionRange { start: usize, end: usize },
}

impl fmt::Display for InputValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputValidationError::EmptyKey => write!(f, "Key value cannot be empty"),
            InputValidationError::EmptyPasteText => write!(f, "Paste text cannot be empty"),
            InputValidationError::EmptyCompositionData => write!(f, "Composition data cannot be empty"),
            InputValidationError::InvalidSelectionRange { start, end } => {
                write!(f, "Invalid selection range: start ({}) is greater than end ({})", start, end)
            }
        }
    }
}

impl std::error::Error for InputValidationError {}

/// InputEvent represents raw user input events before they are processed into semantic editor operations.
/// 
/// These events capture the low-level details of user interaction with the editor, including
/// keyboard input, mouse actions, IME composition, and clipboard operations. InputEvents are
/// processed by the editor's event handling system to produce [`EditorEvent`]s that represent
/// the actual changes to be made to the document.
/// 
/// # Event Flow
/// 
/// ```text
/// Browser Event → InputEvent → Event Processor → EditorEvent → Document Update
/// ```
/// 
/// # Platform Considerations
/// 
/// InputEvents are designed to abstract over platform differences while preserving enough
/// information for platform-specific handling when needed. For example, the Modifiers struct
/// captures all modifier keys, allowing the event processor to handle Cmd+C on macOS and
/// Ctrl+C on Windows/Linux appropriately.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum InputEvent {
    /// Fired when a key is pressed down.
    /// 
    /// This event is triggered repeatedly if the key is held down. It includes
    /// all keys, including non-printable keys like arrows, function keys, etc.
    /// 
    /// # Fields
    /// * `key` - The key value (e.g., "a", "Enter", "ArrowLeft", "F1")
    /// * `code` - The physical key code (e.g., "KeyA", "Enter", "ArrowLeft")
    /// * `modifiers` - State of modifier keys when this event occurred
    /// * `location` - Physical location of the key on the keyboard
    /// * `repeat` - Whether this is a repeat event from holding the key
    KeyDown {
        key: String,
        code: String,
        modifiers: Modifiers,
        location: KeyLocation,
        repeat: bool,
    },

    /// Fired when a key is released.
    /// 
    /// This event occurs once when a key is released after being pressed.
    /// 
    /// # Fields
    /// * `key` - The key value (e.g., "a", "Enter", "ArrowLeft", "F1")
    /// * `code` - The physical key code (e.g., "KeyA", "Enter", "ArrowLeft")
    /// * `modifiers` - State of modifier keys when this event occurred
    /// * `location` - Physical location of the key on the keyboard
    KeyUp {
        key: String,
        code: String,
        modifiers: Modifiers,
        location: KeyLocation,
    },

    /// Fired when a key press would insert text.
    /// 
    /// This is a higher-level event than KeyDown, fired only for keys that
    /// produce text input. It respects the keyboard layout and modifier keys.
    /// 
    /// # Fields
    /// * `key` - The character(s) to be inserted
    /// * `modifiers` - State of modifier keys when this event occurred
    /// 
    /// # Note
    /// This event is not fired for non-printable keys or when Ctrl/Cmd is pressed
    /// (except for specific combinations like Ctrl+V for paste).
    KeyPress {
        key: String,
        modifiers: Modifiers,
    },

    /// Fired when a mouse button is pressed down.
    /// 
    /// # Fields
    /// * `position` - The character position in the document where the click occurred
    /// * `button` - Which mouse button was pressed
    /// * `modifiers` - State of modifier keys when this event occurred
    MouseDown {
        position: usize,
        button: MouseButton,
        modifiers: Modifiers,
    },

    /// Fired when a mouse button is released.
    /// 
    /// # Fields
    /// * `position` - The character position in the document where the release occurred
    /// * `button` - Which mouse button was released
    /// * `modifiers` - State of modifier keys when this event occurred
    MouseUp {
        position: usize,
        button: MouseButton,
        modifiers: Modifiers,
    },

    /// Fired when a mouse button is clicked (down + up).
    /// 
    /// This is a convenience event that combines MouseDown and MouseUp.
    /// 
    /// # Fields
    /// * `position` - The character position in the document where the click occurred
    /// * `button` - Which mouse button was clicked
    /// * `modifiers` - State of modifier keys when this event occurred
    /// * `count` - Click count (1 for single click, 2 for double click, etc.)
    MouseClick {
        position: usize,
        button: MouseButton,
        modifiers: Modifiers,
        count: u8,
    },

    /// Fired when the mouse is moved.
    /// 
    /// # Fields
    /// * `position` - The character position in the document where the mouse is
    MouseMove {
        position: usize,
    },

    /// Fired when text is selected with the mouse.
    /// 
    /// This event is typically fired during mouse drag operations.
    /// 
    /// # Fields
    /// * `start` - The starting position of the selection
    /// * `end` - The ending position of the selection
    /// 
    /// # Note
    /// The selection is normalized so that `start` <= `end`.
    MouseSelect {
        start: usize,
        end: usize,
    },

    /// Fired when text composition (IME) starts.
    /// 
    /// This event marks the beginning of text composition, typically for
    /// languages that require multiple keystrokes to form a single character
    /// (e.g., Chinese, Japanese, Korean).
    CompositionStart,

    /// Fired when the composition text is updated.
    /// 
    /// This event provides the current state of the text being composed.
    /// 
    /// # Fields
    /// * `data` - The current composition text
    CompositionUpdate {
        data: String,
    },

    /// Fired when text composition is completed.
    /// 
    /// This event provides the final text to be inserted.
    /// 
    /// # Fields
    /// * `data` - The final composed text to insert
    CompositionEnd {
        data: String,
    },

    /// Fired when text is pasted into the editor.
    /// 
    /// # Fields
    /// * `text` - The text content being pasted
    /// 
    /// # Note
    /// Rich text paste is not yet supported; only plain text is handled.
    Paste {
        text: String,
    },

    /// Fired when the cut command is triggered.
    /// 
    /// This event indicates that the current selection should be cut
    /// (copied to clipboard and deleted).
    Cut,

    /// Fired when the copy command is triggered.
    /// 
    /// This event indicates that the current selection should be copied
    /// to the clipboard.
    Copy,
}

impl InputEvent {
    /// Validates that the input event contains valid data.
    /// 
    /// # Returns
    /// - `Ok(())` if the event is valid
    /// - `Err(InputValidationError)` if the event contains invalid data
    /// 
    /// # Validation Rules
    /// - Key events must have non-empty key values
    /// - Paste events must have non-empty text
    /// - Composition events must have non-empty data (except CompositionStart)
    /// - Mouse selection must have valid range (start <= end)
    pub fn validate(&self) -> Result<(), InputValidationError> {
        match self {
            InputEvent::KeyDown { key, .. } | 
            InputEvent::KeyUp { key, .. } |
            InputEvent::KeyPress { key, .. } => {
                if key.is_empty() {
                    Err(InputValidationError::EmptyKey)
                } else {
                    Ok(())
                }
            }
            InputEvent::Paste { text } => {
                if text.is_empty() {
                    Err(InputValidationError::EmptyPasteText)
                } else {
                    Ok(())
                }
            }
            InputEvent::CompositionUpdate { data } |
            InputEvent::CompositionEnd { data } => {
                if data.is_empty() {
                    Err(InputValidationError::EmptyCompositionData)
                } else {
                    Ok(())
                }
            }
            InputEvent::MouseSelect { start, end } => {
                if start > end {
                    Err(InputValidationError::InvalidSelectionRange { 
                        start: *start, 
                        end: *end 
                    })
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    /// Returns the category of this input event.
    /// 
    /// This is useful for filtering or routing events based on their type.
    pub fn event_category(&self) -> InputEventCategory {
        match self {
            InputEvent::KeyDown { .. } |
            InputEvent::KeyUp { .. } |
            InputEvent::KeyPress { .. } => InputEventCategory::Keyboard,
            
            InputEvent::MouseDown { .. } |
            InputEvent::MouseUp { .. } |
            InputEvent::MouseClick { .. } |
            InputEvent::MouseMove { .. } |
            InputEvent::MouseSelect { .. } => InputEventCategory::Mouse,
            
            InputEvent::CompositionStart |
            InputEvent::CompositionUpdate { .. } |
            InputEvent::CompositionEnd { .. } => InputEventCategory::Composition,
            
            InputEvent::Paste { .. } |
            InputEvent::Cut |
            InputEvent::Copy => InputEventCategory::Clipboard,
        }
    }

    /// Checks if this event represents text input.
    /// 
    /// Text input events are those that would normally result in text being
    /// inserted into the document.
    /// 
    /// # Returns
    /// `true` for KeyPress, CompositionEnd, and Paste events; `false` otherwise.
    pub fn is_text_input(&self) -> bool {
        matches!(
            self,
            InputEvent::KeyPress { .. } |
            InputEvent::CompositionEnd { .. } |
            InputEvent::Paste { .. }
        )
    }

    /// Checks if this event has any modifier keys pressed.
    /// 
    /// # Returns
    /// `true` if any modifier key (Ctrl, Alt, Shift, Meta) is pressed; `false` otherwise.
    pub fn has_modifiers(&self) -> bool {
        match self {
            InputEvent::KeyDown { modifiers, .. } |
            InputEvent::KeyUp { modifiers, .. } |
            InputEvent::KeyPress { modifiers, .. } |
            InputEvent::MouseDown { modifiers, .. } |
            InputEvent::MouseUp { modifiers, .. } |
            InputEvent::MouseClick { modifiers, .. } => modifiers.any(),
            _ => false,
        }
    }

    /// Determines if this event should prevent the default browser behavior.
    /// 
    /// This is important for keyboard shortcuts and other events where we want
    /// to handle the event ourselves rather than letting the browser handle it.
    /// 
    /// # Returns
    /// `true` if the default behavior should be prevented; `false` otherwise.
    /// 
    /// # Examples
    /// - Ctrl+B for bold should prevent default
    /// - Arrow keys for navigation should prevent default
    /// - Regular text input should not prevent default
    pub fn should_prevent_default(&self) -> bool {
        match self {
            InputEvent::KeyDown { key, modifiers, .. } => {
                // Prevent default for keyboard shortcuts
                if modifiers.primary() {
                    matches!(
                        key.as_str(),
                        "b" | "i" | "u" | "z" | "y" | "x" | "c" | "v" | "a" |
                        "B" | "I" | "U" | "Z" | "Y" | "X" | "C" | "V" | "A"
                    )
                } else {
                    // Prevent default for navigation keys
                    matches!(
                        key.as_str(),
                        "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" |
                        "Home" | "End" | "PageUp" | "PageDown" |
                        "Tab" | "Enter" | "Backspace" | "Delete"
                    )
                }
            }
            InputEvent::Cut | InputEvent::Copy | InputEvent::Paste { .. } => true,
            _ => false,
        }
    }

    /// Returns a string representation of the event type for logging.
    /// 
    /// This provides a human-readable name for the event type.
    pub fn event_type(&self) -> &'static str {
        match self {
            InputEvent::KeyDown { .. } => "KeyDown",
            InputEvent::KeyUp { .. } => "KeyUp",
            InputEvent::KeyPress { .. } => "KeyPress",
            InputEvent::MouseDown { .. } => "MouseDown",
            InputEvent::MouseUp { .. } => "MouseUp",
            InputEvent::MouseClick { .. } => "MouseClick",
            InputEvent::MouseMove { .. } => "MouseMove",
            InputEvent::MouseSelect { .. } => "MouseSelect",
            InputEvent::CompositionStart => "CompositionStart",
            InputEvent::CompositionUpdate { .. } => "CompositionUpdate",
            InputEvent::CompositionEnd { .. } => "CompositionEnd",
            InputEvent::Paste { .. } => "Paste",
            InputEvent::Cut => "Cut",
            InputEvent::Copy => "Copy",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_text_insert_serialization() {
        let event = EditorEvent::TextInsert {
            position: 5,
            text: "Hello".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: EditorEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            EditorEvent::TextInsert { position, text } => {
                assert_eq!(position, 5);
                assert_eq!(text, "Hello");
            }
            _ => panic!("Wrong event type after deserialization"),
        }

        // Check JSON format
        assert!(json.contains("\"type\":\"textInsert\""));
        assert!(json.contains("\"position\":5"));
        assert!(json.contains("\"text\":\"Hello\""));
    }

    #[test]
    fn test_text_delete_serialization() {
        let event = EditorEvent::TextDelete {
            start: 10,
            length: 5,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: EditorEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            EditorEvent::TextDelete { start, length } => {
                assert_eq!(start, 10);
                assert_eq!(length, 5);
            }
            _ => panic!("Wrong event type after deserialization"),
        }

        assert!(json.contains("\"type\":\"textDelete\""));
    }

    #[test]
    fn test_selection_change_serialization() {
        let event = EditorEvent::SelectionChange { start: 5, end: 10 };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: EditorEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            EditorEvent::SelectionChange { start, end } => {
                assert_eq!(start, 5);
                assert_eq!(end, 10);
            }
            _ => panic!("Wrong event type after deserialization"),
        }

        assert!(json.contains("\"type\":\"selectionChange\""));
    }

    #[test]
    fn test_cursor_move_serialization() {
        let event = EditorEvent::CursorMove { position: 15 };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: EditorEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            EditorEvent::CursorMove { position } => {
                assert_eq!(position, 15);
            }
            _ => panic!("Wrong event type after deserialization"),
        }

        assert!(json.contains("\"type\":\"cursorMove\""));
    }

    #[test]
    fn test_json_format_readability() {
        let event = EditorEvent::TextInsert {
            position: 0,
            text: "Test".to_string(),
        };

        let json = serde_json::to_string_pretty(&event).unwrap();

        // Verify the JSON is readable and well-formatted
        assert!(json.contains("{\n"));
        assert!(json.contains("  \"type\": \"textInsert\""));
        assert!(json.contains("  \"position\": 0"));
        assert!(json.contains("  \"text\": \"Test\""));
    }

    #[test]
    fn test_text_insert_validation() {
        // Valid text insert
        let valid = EditorEvent::TextInsert {
            position: 0,
            text: "Hello".to_string(),
        };
        assert!(valid.validate().is_ok());

        // Invalid: empty text
        let invalid = EditorEvent::TextInsert {
            position: 0,
            text: "".to_string(),
        };
        assert_eq!(invalid.validate(), Err(ValidationError::EmptyText));
    }

    #[test]
    fn test_text_delete_validation() {
        // Valid delete
        let valid = EditorEvent::TextDelete {
            start: 0,
            length: 5,
        };
        assert!(valid.validate().is_ok());

        // Invalid: zero length
        let invalid_zero = EditorEvent::TextDelete {
            start: 0,
            length: 0,
        };
        assert_eq!(
            invalid_zero.validate(),
            Err(ValidationError::ZeroLengthDelete)
        );

        // Invalid: overflow
        let invalid_overflow = EditorEvent::TextDelete {
            start: usize::MAX - 1,
            length: 2,
        };
        assert_eq!(
            invalid_overflow.validate(),
            Err(ValidationError::PositionOverflow)
        );
    }

    #[test]
    fn test_selection_change_validation() {
        // Valid selection
        let valid = EditorEvent::SelectionChange { start: 5, end: 10 };
        assert!(valid.validate().is_ok());

        // Valid: collapsed selection
        let valid_collapsed = EditorEvent::SelectionChange { start: 5, end: 5 };
        assert!(valid_collapsed.validate().is_ok());

        // Invalid: start > end
        let invalid = EditorEvent::SelectionChange { start: 10, end: 5 };
        assert_eq!(
            invalid.validate(),
            Err(ValidationError::InvalidSelectionRange { start: 10, end: 5 })
        );
    }

    #[test]
    fn test_cursor_move_validation() {
        // All cursor positions are valid
        let cursor_at_zero = EditorEvent::CursorMove { position: 0 };
        assert!(cursor_at_zero.validate().is_ok());

        let cursor_at_max = EditorEvent::CursorMove {
            position: usize::MAX,
        };
        assert!(cursor_at_max.validate().is_ok());
    }

    #[test]
    fn test_validation_error_display() {
        assert_eq!(
            ValidationError::EmptyText.to_string(),
            "Text content cannot be empty"
        );
        assert_eq!(
            ValidationError::ZeroLengthDelete.to_string(),
            "Delete length must be greater than zero"
        );
        assert_eq!(
            ValidationError::InvalidSelectionRange { start: 10, end: 5 }.to_string(),
            "Invalid selection range: start (10) is greater than end (5)"
        );
        assert_eq!(
            ValidationError::PositionOverflow.to_string(),
            "Position or range would cause integer overflow"
        );
    }

    #[test]
    fn test_get_affected_range() {
        // TextInsert
        let insert = EditorEvent::TextInsert {
            position: 5,
            text: "Hello".to_string(),
        };
        assert_eq!(insert.get_affected_range(), Some((5, 10)));

        // TextDelete
        let delete = EditorEvent::TextDelete {
            start: 3,
            length: 4,
        };
        assert_eq!(delete.get_affected_range(), Some((3, 7)));

        // SelectionChange
        let selection = EditorEvent::SelectionChange { start: 10, end: 20 };
        assert_eq!(selection.get_affected_range(), Some((10, 20)));

        // CursorMove (no affected range)
        let cursor = EditorEvent::CursorMove { position: 15 };
        assert_eq!(cursor.get_affected_range(), None);
    }

    #[test]
    fn test_affects_position() {
        let event = EditorEvent::TextDelete {
            start: 5,
            length: 10,
        };

        // Positions within the range
        assert!(event.affects_position(5)); // Start position
        assert!(event.affects_position(7)); // Middle
        assert!(event.affects_position(14)); // End - 1

        // Positions outside the range
        assert!(!event.affects_position(4)); // Before start
        assert!(!event.affects_position(15)); // At end (exclusive)
        assert!(!event.affects_position(20)); // After end

        // CursorMove doesn't affect any position
        let cursor = EditorEvent::CursorMove { position: 10 };
        assert!(!cursor.affects_position(10));
        assert!(!cursor.affects_position(0));
    }

    #[test]
    fn test_event_type() {
        let insert = EditorEvent::TextInsert {
            position: 0,
            text: "test".to_string(),
        };
        assert_eq!(insert.event_type(), "TextInsert");

        let delete = EditorEvent::TextDelete {
            start: 0,
            length: 1,
        };
        assert_eq!(delete.event_type(), "TextDelete");

        let selection = EditorEvent::SelectionChange { start: 0, end: 5 };
        assert_eq!(selection.event_type(), "SelectionChange");

        let cursor = EditorEvent::CursorMove { position: 0 };
        assert_eq!(cursor.event_type(), "CursorMove");
    }

    #[test]
    fn test_utility_methods_edge_cases() {
        // Empty text insert
        let empty_insert = EditorEvent::TextInsert {
            position: 10,
            text: "".to_string(),
        };
        assert_eq!(empty_insert.get_affected_range(), Some((10, 10)));
        assert!(!empty_insert.affects_position(10)); // Empty range doesn't affect position

        // Zero-width selection
        let collapsed_selection = EditorEvent::SelectionChange { start: 5, end: 5 };
        assert_eq!(collapsed_selection.get_affected_range(), Some((5, 5)));
        assert!(!collapsed_selection.affects_position(5)); // Collapsed selection doesn't affect position
    }

    #[test]
    fn test_clone_trait() {
        let original = EditorEvent::TextInsert {
            position: 42,
            text: "cloned".to_string(),
        };
        let cloned = original.clone();

        match (original, cloned) {
            (
                EditorEvent::TextInsert {
                    position: p1,
                    text: t1,
                },
                EditorEvent::TextInsert {
                    position: p2,
                    text: t2,
                },
            ) => {
                assert_eq!(p1, p2);
                assert_eq!(t1, t2);
            }
            _ => panic!("Clone didn't preserve event type"),
        }
    }

    #[test]
    fn test_debug_trait() {
        let event = EditorEvent::CursorMove { position: 123 };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("CursorMove"));
        assert!(debug_str.contains("123"));
    }

    #[test]
    fn test_unicode_text_handling() {
        // Test with Unicode characters
        let unicode_event = EditorEvent::TextInsert {
            position: 0,
            text: "Hello 世界 🌍".to_string(),
        };

        // Validate it works
        assert!(unicode_event.validate().is_ok());

        // Check serialization
        let json = serde_json::to_string(&unicode_event).unwrap();
        let deserialized: EditorEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            EditorEvent::TextInsert { text, .. } => {
                assert_eq!(text, "Hello 世界 🌍");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_large_position_values() {
        // Test with large but valid position values
        let large_pos = usize::MAX / 2;

        let event = EditorEvent::CursorMove {
            position: large_pos,
        };
        assert!(event.validate().is_ok());
        assert_eq!(event.get_affected_range(), None);

        // Test delete that would overflow
        let overflow_delete = EditorEvent::TextDelete {
            start: usize::MAX - 5,
            length: 10,
        };
        assert_eq!(
            overflow_delete.validate(),
            Err(ValidationError::PositionOverflow)
        );

        // Test valid large delete
        let valid_large_delete = EditorEvent::TextDelete {
            start: large_pos,
            length: 100,
        };
        assert!(valid_large_delete.validate().is_ok());
    }

    #[test]
    fn test_event_equality() {
        // Test that events can be compared
        let event1 = EditorEvent::TextInsert {
            position: 5,
            text: "test".to_string(),
        };
        let event2 = EditorEvent::TextInsert {
            position: 5,
            text: "test".to_string(),
        };
        let event3 = EditorEvent::TextInsert {
            position: 5,
            text: "different".to_string(),
        };

        // Clone should produce equal events
        let cloned = event1.clone();
        match (&event1, &cloned) {
            (
                EditorEvent::TextInsert {
                    position: p1,
                    text: t1,
                },
                EditorEvent::TextInsert {
                    position: p2,
                    text: t2,
                },
            ) => {
                assert_eq!(p1, p2);
                assert_eq!(t1, t2);
            }
            _ => panic!("Unexpected event types"),
        }
    }

    #[test]
    fn test_multiline_text() {
        let multiline = EditorEvent::TextInsert {
            position: 0,
            text: "Line 1\nLine 2\nLine 3".to_string(),
        };

        assert!(multiline.validate().is_ok());
        assert_eq!(multiline.get_affected_range(), Some((0, 20))); // Length includes newlines

        // Test serialization preserves newlines
        let json = serde_json::to_string(&multiline).unwrap();
        assert!(json.contains("Line 1\\nLine 2\\nLine 3"));
    }

    #[test]
    fn test_error_trait_implementation() {
        use std::error::Error;

        let error = ValidationError::EmptyText;

        // Verify Error trait is implemented
        let _: &dyn Error = &error;

        // Display should work
        assert!(!error.to_string().is_empty());
    }
}

