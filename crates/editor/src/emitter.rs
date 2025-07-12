//! Event emitter system for the QuillAI editor.
//!
//! This module provides the central event emitter that manages event subscriptions,
//! processing, and propagation throughout the editor. It connects raw input events
//! to semantic editor operations through a type-safe, efficient pipeline.
//!
//! # Architecture
//!
//! The event emitter follows a publish-subscribe pattern with the following flow:
//! ```text
//! InputEvent → EventProcessor → EditorEvent → Subscribers → Document Updates
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use quillai_editor::emitter::{EventEmitter, EventProcessor};
//! use quillai_editor::events::{InputEvent, EditorEvent};
//!
//! let mut emitter = EventEmitter::new();
//! let processor = EventProcessor::new();
//!
//! // Subscribe to editor events
//! emitter.on_editor_event(|event| {
//!     match event {
//!         EditorEvent::TextInsert { position, text } => {
//!             println!("Inserting '{}' at position {}", text, position);
//!         }
//!         _ => {}
//!     }
//! });
//!
//! // Process an input event
//! let input = InputEvent::KeyPress {
//!     key: "a".to_string(),
//!     modifiers: Modifiers::none(),
//! };
//!
//! if let Some(editor_event) = processor.process_input(input) {
//!     emitter.emit_editor_event(editor_event);
//! }
//! ```

use crate::events::{EditorEvent, InputEvent, Modifiers, MouseButton};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

/// Type alias for event handler functions that process EditorEvents.
pub type EditorEventHandler = Box<dyn Fn(&EditorEvent) + Send + Sync>;

/// Type alias for event handler functions that process InputEvents.
pub type InputEventHandler = Box<dyn Fn(&InputEvent) + Send + Sync>;

/// Unique identifier for event subscriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(u64);

impl SubscriptionId {
    fn new() -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }
}

/// Context information attached to each event.
#[derive(Debug, Clone)]
pub struct EventContext {
    /// Unique identifier for this event instance.
    pub id: String,
    /// Timestamp when the event was created.
    pub timestamp: u64,
    /// Source of the event (e.g., "user", "system", "api").
    pub source: String,
    /// Optional parent event ID for event chains.
    pub parent_id: Option<String>,
    /// Additional metadata as key-value pairs.
    pub metadata: HashMap<String, String>,
}

impl EventContext {
    /// Creates a new event context with the given source.
    pub fn new(source: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            id: format!("evt_{}", uuid::Uuid::new_v4()),
            timestamp,
            source: source.into(),
            parent_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Creates a child context from this context.
    pub fn child(&self, source: impl Into<String>) -> Self {
        let mut child = Self::new(source);
        child.parent_id = Some(self.id.clone());
        child
    }

    /// Adds metadata to the context.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Subscription handle that automatically unsubscribes when dropped.
pub struct Subscription {
    id: SubscriptionId,
    unsubscribe: Arc<dyn Fn(SubscriptionId) + Send + Sync>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        (self.unsubscribe)(self.id);
    }
}

/// Central event emitter for the editor.
///
/// The EventEmitter manages subscriptions and dispatches events to registered handlers.
/// It supports both EditorEvents (semantic operations) and InputEvents (raw input).
pub struct EventEmitter {
    editor_handlers: Arc<Mutex<HashMap<SubscriptionId, Weak<EditorEventHandler>>>>,
    input_handlers: Arc<Mutex<HashMap<SubscriptionId, Weak<InputEventHandler>>>>,
}

impl EventEmitter {
    /// Creates a new event emitter.
    pub fn new() -> Self {
        Self {
            editor_handlers: Arc::new(Mutex::new(HashMap::new())),
            input_handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Subscribes to editor events.
    ///
    /// Returns a Subscription handle that will automatically unsubscribe when dropped.
    ///
    /// # Example
    /// ```rust,ignore
    /// let subscription = emitter.on_editor_event(|event| {
    ///     println!("Editor event: {:?}", event);
    /// });
    /// // Subscription is active until `subscription` is dropped
    /// ```
    pub fn on_editor_event<F>(&self, handler: F) -> Subscription
    where
        F: Fn(&EditorEvent) + Send + Sync + 'static,
    {
        let id = SubscriptionId::new();
        let handler = Arc::new(Box::new(handler) as EditorEventHandler);

        self.editor_handlers
            .lock()
            .unwrap()
            .insert(id, Arc::downgrade(&handler));

        let handlers = Arc::clone(&self.editor_handlers);
        let unsubscribe = Arc::new(move |id: SubscriptionId| {
            handlers.lock().unwrap().remove(&id);
        });

        // Keep the handler alive by storing it in the subscription
        std::mem::forget(handler);

        Subscription { id, unsubscribe }
    }

    /// Subscribes to input events.
    ///
    /// Returns a Subscription handle that will automatically unsubscribe when dropped.
    pub fn on_input_event<F>(&self, handler: F) -> Subscription
    where
        F: Fn(&InputEvent) + Send + Sync + 'static,
    {
        let id = SubscriptionId::new();
        let handler = Arc::new(Box::new(handler) as InputEventHandler);

        self.input_handlers
            .lock()
            .unwrap()
            .insert(id, Arc::downgrade(&handler));

        let handlers = Arc::clone(&self.input_handlers);
        let unsubscribe = Arc::new(move |id: SubscriptionId| {
            handlers.lock().unwrap().remove(&id);
        });

        // Keep the handler alive by storing it in the subscription
        std::mem::forget(handler);

        Subscription { id, unsubscribe }
    }

    /// Emits an editor event to all subscribers.
    pub fn emit_editor_event(&self, event: EditorEvent) {
        let handlers = self.editor_handlers.lock().unwrap();
        let active_handlers: Vec<_> = handlers
            .values()
            .filter_map(|weak| weak.upgrade())
            .collect();

        // Release the lock before calling handlers
        drop(handlers);

        for handler in active_handlers {
            handler(&event);
        }
    }

    /// Emits an input event to all subscribers.
    pub fn emit_input_event(&self, event: InputEvent) {
        let handlers = self.input_handlers.lock().unwrap();
        let active_handlers: Vec<_> = handlers
            .values()
            .filter_map(|weak| weak.upgrade())
            .collect();

        // Release the lock before calling handlers
        drop(handlers);

        for handler in active_handlers {
            handler(&event);
        }
    }

    /// Cleans up expired subscriptions.
    ///
    /// This is automatically called during emit operations, but can be
    /// called manually for more aggressive cleanup.
    pub fn cleanup(&self) {
        let mut editor_handlers = self.editor_handlers.lock().unwrap();
        editor_handlers.retain(|_, weak| weak.strong_count() > 0);

        let mut input_handlers = self.input_handlers.lock().unwrap();
        input_handlers.retain(|_, weak| weak.strong_count() > 0);
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Processes input events into editor events.
///
/// The EventProcessor contains the logic for converting raw input events
/// (keyboard, mouse, etc.) into semantic editor operations.
pub struct EventProcessor {
    /// Current document length for validation.
    document_length: usize,
    /// Current selection state.
    selection: Option<(usize, usize)>,
}

impl EventProcessor {
    /// Creates a new event processor.
    pub fn new() -> Self {
        Self {
            document_length: 0,
            selection: None,
        }
    }

    /// Updates the document length for validation.
    pub fn set_document_length(&mut self, length: usize) {
        self.document_length = length;
    }

    /// Updates the current selection state.
    pub fn set_selection(&mut self, start: usize, end: usize) {
        self.selection = Some((start, end));
    }

    /// Processes an input event and returns the corresponding editor event(s).
    ///
    /// Returns None if the input event doesn't map to an editor operation.
    pub fn process_input(&self, input: &InputEvent) -> Option<EditorEvent> {
        match input {
            // Text input events
            InputEvent::KeyPress { key, modifiers } => {
                if !modifiers.any() && !key.is_empty() {
                    let position = self.selection.map(|(start, _)| start).unwrap_or(0);
                    Some(EditorEvent::TextInsert {
                        position,
                        text: key.clone(),
                    })
                } else {
                    None
                }
            }

            // Keyboard navigation and editing
            InputEvent::KeyDown { key, modifiers, .. } => {
                match key.as_str() {
                    "Backspace" => {
                        if let Some((start, end)) = self.selection {
                            if start == end && start > 0 {
                                // Delete character before cursor
                                Some(EditorEvent::TextDelete {
                                    start: start - 1,
                                    length: 1,
                                })
                            } else if start < end {
                                // Delete selection
                                Some(EditorEvent::TextDelete {
                                    start,
                                    length: end - start,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    "Delete" => {
                        if let Some((start, end)) = self.selection {
                            if start == end && start < self.document_length {
                                // Delete character after cursor
                                Some(EditorEvent::TextDelete { start, length: 1 })
                            } else if start < end {
                                // Delete selection
                                Some(EditorEvent::TextDelete {
                                    start,
                                    length: end - start,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Home" | "End"
                    | "PageUp" | "PageDown" => {
                        // These would update cursor position
                        // For now, we'll skip as we need more context
                        None
                    }
                    _ => None,
                }
            }

            // Mouse events
            InputEvent::MouseClick {
                position,
                button,
                count,
                ..
            } => {
                if *button == MouseButton::Left && *count == 1 {
                    Some(EditorEvent::CursorMove {
                        position: *position,
                    })
                } else {
                    None
                }
            }

            InputEvent::MouseSelect { start, end } => Some(EditorEvent::SelectionChange {
                start: *start,
                end: *end,
            }),

            // Composition events
            InputEvent::CompositionEnd { data } => {
                if !data.is_empty() {
                    let position = self.selection.map(|(start, _)| start).unwrap_or(0);
                    Some(EditorEvent::TextInsert {
                        position,
                        text: data.clone(),
                    })
                } else {
                    None
                }
            }

            // Clipboard events
            InputEvent::Paste { text } => {
                if !text.is_empty() {
                    let position = self.selection.map(|(start, _)| start).unwrap_or(0);
                    Some(EditorEvent::TextInsert {
                        position,
                        text: text.clone(),
                    })
                } else {
                    None
                }
            }

            // Other events don't directly map to editor operations
            _ => None,
        }
    }

    /// Processes a keyboard shortcut and returns the corresponding editor event.
    ///
    /// This handles common keyboard shortcuts like Ctrl+B for bold, etc.
    pub fn process_shortcut(&self, key: &str, modifiers: &Modifiers) -> Option<ShortcutCommand> {
        if !modifiers.primary() {
            return None;
        }

        match key.to_lowercase().as_str() {
            "b" => Some(ShortcutCommand::Bold),
            "i" => Some(ShortcutCommand::Italic),
            "u" => Some(ShortcutCommand::Underline),
            "z" => Some(ShortcutCommand::Undo),
            "y" => Some(ShortcutCommand::Redo),
            "x" => Some(ShortcutCommand::Cut),
            "c" => Some(ShortcutCommand::Copy),
            "v" => Some(ShortcutCommand::Paste),
            "a" => Some(ShortcutCommand::SelectAll),
            _ => None,
        }
    }
}

impl Default for EventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a keyboard shortcut command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutCommand {
    Bold,
    Italic,
    Underline,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

/// Standard event types emitted by the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardEventType {
    /// Content has changed
    TextChange,
    /// Selection has changed
    SelectionChange,
    /// Any editor change (text or selection)
    EditorChange,
    /// Editor has been scrolled
    ScrollUpdate,
}

/// Handler for processing input events through the pipeline.
///
/// This trait allows for custom input processing logic to be plugged into
/// the event system.
pub trait InputHandler: Send + Sync {
    /// Processes an input event and optionally returns editor events.
    fn handle_input(&mut self, input: &InputEvent, context: &EventContext) -> Vec<EditorEvent>;

    /// Returns the priority of this handler (higher = processed first).
    fn priority(&self) -> i32 {
        0
    }
}

/// Default input handler that uses the EventProcessor.
pub struct DefaultInputHandler {
    processor: EventProcessor,
}

impl DefaultInputHandler {
    /// Creates a new default input handler.
    pub fn new() -> Self {
        Self {
            processor: EventProcessor::new(),
        }
    }
}

impl InputHandler for DefaultInputHandler {
    fn handle_input(&mut self, input: &InputEvent, _context: &EventContext) -> Vec<EditorEvent> {
        self.processor.process_input(input).into_iter().collect()
    }
}

/// Manages the complete event processing pipeline.
///
/// The EventPipeline coordinates input handlers, event processing, and
/// event emission in a structured way.
pub struct EventPipeline {
    emitter: EventEmitter,
    handlers: Vec<Box<dyn InputHandler>>,
}

impl EventPipeline {
    /// Creates a new event pipeline with default handlers.
    pub fn new() -> Self {
        Self {
            emitter: EventEmitter::new(),
            handlers: vec![Box::new(DefaultInputHandler::new())],
        }
    }

    /// Adds a custom input handler to the pipeline.
    pub fn add_handler(&mut self, handler: Box<dyn InputHandler>) {
        self.handlers.push(handler);
        // Sort by priority (highest first)
        self.handlers.sort_by_key(|h| -h.priority());
    }

    /// Processes an input event through the pipeline.
    ///
    /// The input event is passed through all registered handlers in priority order,
    /// and the resulting editor events are emitted.
    pub fn process_input(&mut self, input: InputEvent) {
        let context = EventContext::new("user");

        // First emit the raw input event
        self.emitter.emit_input_event(input.clone());

        // Process through handlers
        let mut editor_events = Vec::new();
        for handler in &mut self.handlers {
            let events = handler.handle_input(&input, &context);
            editor_events.extend(events);
        }

        // Emit the resulting editor events
        for event in editor_events {
            self.emitter.emit_editor_event(event);
        }
    }

    /// Gets a reference to the event emitter for subscribing to events.
    pub fn emitter(&self) -> &EventEmitter {
        &self.emitter
    }

    /// Gets a mutable reference to the event emitter.
    pub fn emitter_mut(&mut self) -> &mut EventEmitter {
        &mut self.emitter
    }
}

impl Default for EventPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export uuid for event ID generation
pub(crate) mod uuid {
    pub struct Uuid(u128);

    impl Uuid {
        pub fn new_v4() -> Self {
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            Self(nanos)
        }
    }

    impl std::fmt::Display for Uuid {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:032x}", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::events::KeyLocation;

    #[test]
    fn test_event_emitter_subscription() {
        let emitter = EventEmitter::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = Arc::clone(&received);

        let _subscription = emitter.on_editor_event(move |event| {
            received_clone.lock().unwrap().push(event.clone());
        });

        emitter.emit_editor_event(EditorEvent::TextInsert {
            position: 0,
            text: "Hello".to_string(),
        });

        let events = received.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            EditorEvent::TextInsert { text, .. } => assert_eq!(text, "Hello"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_processor_key_press() {
        let processor = EventProcessor::new();

        let input = InputEvent::KeyPress {
            key: "a".to_string(),
            modifiers: Modifiers::none(),
        };

        let result = processor.process_input(&input);
        assert!(result.is_some());

        match result.unwrap() {
            EditorEvent::TextInsert { position, text } => {
                assert_eq!(position, 0);
                assert_eq!(text, "a");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_processor_backspace() {
        let mut processor = EventProcessor::new();
        processor.set_selection(5, 5);
        processor.set_document_length(10);

        let input = InputEvent::KeyDown {
            key: "Backspace".to_string(),
            code: "Backspace".to_string(),
            modifiers: Modifiers::none(),
            location: KeyLocation::Standard,
            repeat: false,
        };

        let result = processor.process_input(&input);
        assert!(result.is_some());

        match result.unwrap() {
            EditorEvent::TextDelete { start, length } => {
                assert_eq!(start, 4);
                assert_eq!(length, 1);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_shortcut_detection() {
        let processor = EventProcessor::new();

        let mut modifiers = Modifiers::none();
        // Set the primary modifier based on platform
        #[cfg(target_os = "macos")]
        {
            modifiers.meta = true;
        }
        #[cfg(not(target_os = "macos"))]
        {
            modifiers.ctrl = true;
        }

        let result = processor.process_shortcut("b", &modifiers);
        assert_eq!(result, Some(ShortcutCommand::Bold));

        let result = processor.process_shortcut("B", &modifiers);
        assert_eq!(result, Some(ShortcutCommand::Bold));

        let result = processor.process_shortcut("x", &Modifiers::none());
        assert_eq!(result, None);
    }
}

