use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, Event, EventTarget, HtmlElement, KeyboardEvent, ClipboardEvent,
    Selection, Range, Node, Text as DomText, InputEvent,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// use crate::prelude::*; // commented out due to missing prelude
use quillai_delta::{Delta, Op, AttributeMap};

/// Represents different types of DOM events we need to capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomEventType {
    Input,
    BeforeInput,
    KeyDown,
    KeyUp,
    Paste,
    Cut,
    SelectionChange,
}

/// Captures the selection state before and after an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionState {
    pub start: u32,
    pub end: u32,
    pub direction: String,
}

/// Represents a captured DOM event with all relevant data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedDomEvent {
    pub event_type: DomEventType,
    pub timestamp: f64,
    pub selection_before: Option<SelectionState>,
    pub selection_after: Option<SelectionState>,
    pub content_before: String,
    pub content_after: String,
    pub input_data: Option<String>,
    pub modifier_keys: ModifierKeys,
    pub key_code: Option<String>,
}

/// Tracks modifier key states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifierKeys {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// Represents a detected change in the DOM
#[derive(Debug, Clone)]
pub struct DomChange {
    pub change_type: DomChangeType,
    pub position: u32,
    pub length: u32,
    pub content: Option<String>,
    pub attributes: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub enum DomChangeType {
    Insert,
    Delete,
    Retain,
    FormatChange,
}

/// Main DOM event capture and processing system
pub struct DomEventCapture {
    element: HtmlElement,
    event_listeners: Vec<EventListener>,
    last_content: String,
    last_selection: Option<SelectionState>,
    event_callback: Option<Box<dyn Fn(CapturedDomEvent)>>,
    change_callback: Option<Box<dyn Fn(Vec<DomChange>)>>,
}

/// Wrapper for JavaScript event listeners to ensure proper cleanup
struct EventListener {
    target: EventTarget,
    event_type: String,
    closure: Closure<dyn FnMut(Event)>,
}

impl DomEventCapture {
    /// Creates a new DOM event capture system for the given element
    pub fn new(element: HtmlElement) -> Result<Self, JsValue> {
        let initial_content = Self::get_element_text_content(&element);
        
        Ok(Self {
            element,
            event_listeners: Vec::new(),
            last_content: initial_content,
            last_selection: None,
            event_callback: None,
            change_callback: None,
        })
    }

    /// Sets up all necessary event listeners
    pub fn setup_event_listeners(&mut self) -> Result<(), JsValue> {
        self.setup_input_events()?;
        self.setup_keyboard_events()?;
        self.setup_clipboard_events()?;
        self.setup_selection_events()?;
        Ok(())
    }

    /// Sets up input and beforeinput event listeners
    fn setup_input_events(&mut self) -> Result<(), JsValue> {
        let element_clone = self.element.clone();
        let input_closure = Closure::wrap(Box::new(move |event: Event| {
            if let Ok(input_event) = event.dyn_into::<InputEvent>() {
                Self::handle_input_event(&element_clone, input_event);
            }
        }) as Box<dyn FnMut(Event)>);

        self.element.add_event_listener_with_callback(
            "input",
            input_closure.as_ref().unchecked_ref(),
        )?;

        self.event_listeners.push(EventListener {
            target: self.element.clone().into(),
            event_type: "input".to_string(),
            closure: input_closure,
        });

        // BeforeInput event for better prediction
        let element_clone = self.element.clone();
        let beforeinput_closure = Closure::wrap(Box::new(move |event: Event| {
            if let Ok(input_event) = event.dyn_into::<InputEvent>() {
                Self::handle_beforeinput_event(&element_clone, input_event);
            }
        }) as Box<dyn FnMut(Event)>);

        self.element.add_event_listener_with_callback(
            "beforeinput",
            beforeinput_closure.as_ref().unchecked_ref(),
        )?;

        self.event_listeners.push(EventListener {
            target: self.element.clone().into(),
            event_type: "beforeinput".to_string(),
            closure: beforeinput_closure,
        });

        Ok(())
    }

    /// Sets up keyboard event listeners
    fn setup_keyboard_events(&mut self) -> Result<(), JsValue> {
        // KeyDown events
        let element_clone = self.element.clone();
        let keydown_closure = Closure::wrap(Box::new(move |event: Event| {
            if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
                Self::handle_keydown_event(&element_clone, keyboard_event);
            }
        }) as Box<dyn FnMut(Event)>);

        self.element.add_event_listener_with_callback(
            "keydown",
            keydown_closure.as_ref().unchecked_ref(),
        )?;

        self.event_listeners.push(EventListener {
            target: self.element.clone().into(),
            event_type: "keydown".to_string(),
            closure: keydown_closure,
        });

        // KeyUp events
        let element_clone = self.element.clone();
        let keyup_closure = Closure::wrap(Box::new(move |event: Event| {
            if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
                Self::handle_keyup_event(&element_clone, keyboard_event);
            }
        }) as Box<dyn FnMut(Event)>);

        self.element.add_event_listener_with_callback(
            "keyup",
            keyup_closure.as_ref().unchecked_ref(),
        )?;

        self.event_listeners.push(EventListener {
            target: self.element.clone().into(),
            event_type: "keyup".to_string(),
            closure: keyup_closure,
        });

        Ok(())
    }

    /// Sets up clipboard event listeners
    fn setup_clipboard_events(&mut self) -> Result<(), JsValue> {
        // Paste events
        let element_clone = self.element.clone();
        let paste_closure = Closure::wrap(Box::new(move |event: Event| {
            if let Ok(clipboard_event) = event.dyn_into::<ClipboardEvent>() {
                Self::handle_paste_event(&element_clone, clipboard_event);
            }
        }) as Box<dyn FnMut(Event)>);

        self.element.add_event_listener_with_callback(
            "paste",
            paste_closure.as_ref().unchecked_ref(),
        )?;

        self.event_listeners.push(EventListener {
            target: self.element.clone().into(),
            event_type: "paste".to_string(),
            closure: paste_closure,
        });

        // Cut events
        let element_clone = self.element.clone();
        let cut_closure = Closure::wrap(Box::new(move |event: Event| {
            if let Ok(clipboard_event) = event.dyn_into::<ClipboardEvent>() {
                Self::handle_cut_event(&element_clone, clipboard_event);
            }
        }) as Box<dyn FnMut(Event)>);

        self.element.add_event_listener_with_callback(
            "cut",
            cut_closure.as_ref().unchecked_ref(),
        )?;

        self.event_listeners.push(EventListener {
            target: self.element.clone().into(),
            event_type: "cut".to_string(),
            closure: cut_closure,
        });

        Ok(())
    }

    /// Sets up selection change event listeners
    fn setup_selection_events(&mut self) -> Result<(), JsValue> {
        let document = web_sys::window()
            .ok_or("No window object")?
            .document()
            .ok_or("No document object")?;

        let element_clone = self.element.clone();
        let selection_closure = Closure::wrap(Box::new(move |_event: Event| {
            Self::handle_selection_change(&element_clone);
        }) as Box<dyn FnMut(Event)>);

        document.add_event_listener_with_callback(
            "selectionchange",
            selection_closure.as_ref().unchecked_ref(),
        )?;

        self.event_listeners.push(EventListener {
            target: document.into(),
            event_type: "selectionchange".to_string(),
            closure: selection_closure,
        });

        Ok(())
    }

    /// Event handler for input events
    fn handle_input_event(element: &HtmlElement, event: InputEvent) {
        let captured_event = CapturedDomEvent {
            event_type: DomEventType::Input,
            timestamp: js_sys::Date::now(),
            selection_before: Self::get_selection_state(element),
            selection_after: Self::get_selection_state(element),
            content_before: Self::get_element_text_content(element),
            content_after: Self::get_element_text_content(element),
            input_data: event.data(),
            modifier_keys: Self::get_modifier_keys(&event),
            key_code: None,
        };

        // Log for debugging
        web_sys::console::log_1(&format!("Input event: {:?}", captured_event).into());
    }

    /// Event handler for beforeinput events
    fn handle_beforeinput_event(element: &HtmlElement, event: InputEvent) {
        let captured_event = CapturedDomEvent {
            event_type: DomEventType::BeforeInput,
            timestamp: js_sys::Date::now(),
            selection_before: Self::get_selection_state(element),
            selection_after: None, // Not available yet
            content_before: Self::get_element_text_content(element),
            content_after: String::new(), // Not available yet
            input_data: event.data(),
            modifier_keys: Self::get_modifier_keys(&event),
            key_code: None,
        };

        web_sys::console::log_1(&format!("BeforeInput event: {:?}", captured_event).into());
    }

    /// Event handler for keydown events
    fn handle_keydown_event(element: &HtmlElement, event: KeyboardEvent) {
        let captured_event = CapturedDomEvent {
            event_type: DomEventType::KeyDown,
            timestamp: js_sys::Date::now(),
            selection_before: Self::get_selection_state(element),
            selection_after: None,
            content_before: Self::get_element_text_content(element),
            content_after: String::new(),
            input_data: None,
            modifier_keys: ModifierKeys {
                ctrl: event.ctrl_key(),
                alt: event.alt_key(),
                shift: event.shift_key(),
                meta: event.meta_key(),
            },
            key_code: Some(event.code()),
        };

        web_sys::console::log_1(&format!("KeyDown event: {:?}", captured_event).into());
    }

    /// Event handler for keyup events
    fn handle_keyup_event(element: &HtmlElement, event: KeyboardEvent) {
        let captured_event = CapturedDomEvent {
            event_type: DomEventType::KeyUp,
            timestamp: js_sys::Date::now(),
            selection_before: Self::get_selection_state(element),
            selection_after: Self::get_selection_state(element),
            content_before: Self::get_element_text_content(element),
            content_after: Self::get_element_text_content(element),
            input_data: None,
            modifier_keys: ModifierKeys {
                ctrl: event.ctrl_key(),
                alt: event.alt_key(),
                shift: event.shift_key(),
                meta: event.meta_key(),
            },
            key_code: Some(event.code()),
        };

        web_sys::console::log_1(&format!("KeyUp event: {:?}", captured_event).into());
    }

    /// Event handler for paste events
    fn handle_paste_event(element: &HtmlElement, event: ClipboardEvent) {
        let clipboard_data = event.clipboard_data()
            .and_then(|data| data.get_data("text/plain").ok());

        let captured_event = CapturedDomEvent {
            event_type: DomEventType::Paste,
            timestamp: js_sys::Date::now(),
            selection_before: Self::get_selection_state(element),
            selection_after: None,
            content_before: Self::get_element_text_content(element),
            content_after: String::new(),
            input_data: clipboard_data,
            modifier_keys: ModifierKeys {
                ctrl: event.ctrl_key(),
                alt: event.alt_key(),
                shift: event.shift_key(),
                meta: event.meta_key(),
            },
            key_code: None,
        };

        web_sys::console::log_1(&format!("Paste event: {:?}", captured_event).into());
    }

    /// Event handler for cut events
    fn handle_cut_event(element: &HtmlElement, event: ClipboardEvent) {
        let captured_event = CapturedDomEvent {
            event_type: DomEventType::Cut,
            timestamp: js_sys::Date::now(),
            selection_before: Self::get_selection_state(element),
            selection_after: None,
            content_before: Self::get_element_text_content(element),
            content_after: String::new(),
            input_data: None,
            modifier_keys: ModifierKeys {
                ctrl: event.ctrl_key(),
                alt: event.alt_key(),
                shift: event.shift_key(),
                meta: event.meta_key(),
            },
            key_code: None,
        };

        web_sys::console::log_1(&format!("Cut event: {:?}", captured_event).into());
    }

    /// Event handler for selection change events
    fn handle_selection_change(element: &HtmlElement) {
        let captured_event = CapturedDomEvent {
            event_type: DomEventType::SelectionChange,
            timestamp: js_sys::Date::now(),
            selection_before: None,
            selection_after: Self::get_selection_state(element),
            content_before: Self::get_element_text_content(element),
            content_after: Self::get_element_text_content(element),
            input_data: None,
            modifier_keys: ModifierKeys {
                ctrl: false,
                alt: false,
                shift: false,
                meta: false,
            },
            key_code: None,
        };

        web_sys::console::log_1(&format!("Selection change: {:?}", captured_event).into());
    }

    /// Gets the current selection state for the element
    fn get_selection_state(element: &HtmlElement) -> Option<SelectionState> {
        let window = web_sys::window()?;
        let selection = window.get_selection().ok()??;
        
        if selection.range_count() == 0 {
            return None;
        }

        let range = selection.get_range_at(0).ok()?;
        
        // Calculate text offsets within the element
        let start_offset = Self::get_text_offset_in_element(element, &range.start_container()?, range.start_offset());
        let end_offset = Self::get_text_offset_in_element(element, &range.end_container()?, range.end_offset());

        Some(SelectionState {
            start: start_offset,
            end: end_offset,
            direction: "forward".to_string(), // Simplified for now
        })
    }

    /// Calculates text offset within the element
    fn get_text_offset_in_element(element: &HtmlElement, node: &Node, offset: u32) -> u32 {
        // This is a simplified implementation
        // In a real implementation, we'd need to traverse the DOM tree
        // and calculate the actual text position
        offset
    }

    /// Gets the text content of the element
    fn get_element_text_content(element: &HtmlElement) -> String {
        element.text_content().unwrap_or_default()
    }

    /// Extracts modifier key states from an input event
    fn get_modifier_keys(event: &InputEvent) -> ModifierKeys {
        ModifierKeys {
            ctrl: event.ctrl_key(),
            alt: event.alt_key(),
            shift: event.shift_key(),
            meta: event.meta_key(),
        }
    }

    /// Detects changes between the current and previous content
    pub fn detect_changes(&mut self) -> Vec<DomChange> {
        let current_content = Self::get_element_text_content(&self.element);
        let changes = self.diff_content(&self.last_content, &current_content);
        self.last_content = current_content;
        changes
    }

    /// Performs a simple diff between old and new content
    fn diff_content(&self, old_content: &str, new_content: &str) -> Vec<DomChange> {
        let mut changes = Vec::new();

        // Simple implementation - in reality we'd use a more sophisticated diff algorithm
        if old_content != new_content {
            if new_content.len() > old_content.len() {
                // Content was inserted
                changes.push(DomChange {
                    change_type: DomChangeType::Insert,
                    position: old_content.len() as u32,
                    length: (new_content.len() - old_content.len()) as u32,
                    content: Some(new_content[old_content.len()..].to_string()),
                    attributes: None,
                });
            } else if new_content.len() < old_content.len() {
                // Content was deleted
                changes.push(DomChange {
                    change_type: DomChangeType::Delete,
                    position: new_content.len() as u32,
                    length: (old_content.len() - new_content.len()) as u32,
                    content: None,
                    attributes: None,
                });
            }
        }

        changes
    }

    /// Converts DOM changes to Delta operations
    pub fn changes_to_delta(&self, changes: Vec<DomChange>) -> Delta {
        let mut delta = Delta::new();

        for change in changes {
            match change.change_type {
                DomChangeType::Insert => {
                    if change.position > 0 {
                        delta = delta.retain(change.position);
                    }
                    if let Some(content) = change.content {
                        delta = delta.insert(&content, None);
                    }
                }
                DomChangeType::Delete => {
                    if change.position > 0 {
                        delta = delta.retain(change.position);
                    }
                    delta = delta.delete(change.length);
                }
                DomChangeType::Retain => {
                    delta = delta.retain(change.length);
                }
                DomChangeType::FormatChange => {
                    // Handle formatting changes
                    if change.position > 0 {
                        delta = delta.retain(change.position);
                    }
                    if let Some(attrs) = change.attributes {
                        let mut attribute_map = AttributeMap::new();
                        for (key, value) in attrs {
                            attribute_map.insert(key, serde_json::Value::String(value));
                        }
                        delta = delta.retain_with_attributes(change.length, attribute_map);
                    } else {
                        delta = delta.retain(change.length);
                    }
                }
            }
        }

        delta
    }

    /// Sets a callback for when events are captured
    pub fn set_event_callback<F>(&mut self, callback: F)
    where
        F: Fn(CapturedDomEvent) + 'static,
    {
        self.event_callback = Some(Box::new(callback));
    }

    /// Sets a callback for when changes are detected
    pub fn set_change_callback<F>(&mut self, callback: F)
    where
        F: Fn(Vec<DomChange>) + 'static,
    {
        self.change_callback = Some(Box::new(callback));
    }

    /// Cleanup all event listeners
    pub fn cleanup(&mut self) {
        for listener in &self.event_listeners {
            let _ = listener.target.remove_event_listener_with_callback(
                &listener.event_type,
                listener.closure.as_ref().unchecked_ref(),
            );
        }
        self.event_listeners.clear();
    }
}

impl Drop for DomEventCapture {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Utilities for converting Delta operations to DOM manipulations
pub struct DeltaToDomConverter {
    document: Document,
}

impl DeltaToDomConverter {
    /// Creates a new Delta-to-DOM converter
    pub fn new() -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let document = window.document().ok_or("No document object")?;
        
        Ok(Self { document })
    }

    /// Applies a Delta to a DOM element, updating its content
    pub fn apply_delta_to_element(&self, element: &HtmlElement, delta: &Delta) -> Result<(), JsValue> {
        // Clear existing content
        element.set_inner_html("");
        
        let mut current_position = 0;
        let mut current_element = element.clone();
        
        for operation in delta.ops() {
            match operation {
                Op::Insert { insert, attributes } => {
                    self.apply_insert_operation(&mut current_element, insert, attributes.as_ref())?;
                    current_position += insert.chars().count();
                }
                Op::Retain { retain, attributes } => {
                    // For retain operations, we need to preserve existing content
                    // This is more complex and would require tracking existing content
                    current_position += *retain as usize;
                    
                    if let Some(attrs) = attributes {
                        self.apply_formatting_to_range(&current_element, current_position - *retain as usize, *retain as usize, attrs)?;
                    }
                }
                Op::Delete { delete: _ } => {
                    // Delete operations are handled by not including the content
                    // In a full implementation, we'd need to track what to delete
                }
            }
        }
        
        Ok(())
    }

    /// Applies an insert operation to the DOM
    fn apply_insert_operation(
        &self,
        element: &mut HtmlElement,
        content: &str,
        attributes: Option<&AttributeMap>,
    ) -> Result<(), JsValue> {
        if let Some(attrs) = attributes {
            // Create formatted element
            let formatted_element = self.create_formatted_element(content, attrs)?;
            element.append_child(&formatted_element)?;
        } else {
            // Create plain text node
            let text_node = self.document.create_text_node(content);
            element.append_child(&text_node)?;
        }
        
        Ok(())
    }

    /// Creates a formatted DOM element based on attributes
    fn create_formatted_element(&self, content: &str, attributes: &AttributeMap) -> Result<Element, JsValue> {
        let mut element: Element = self.document.create_element("span")?;
        
        // Apply text formatting attributes
        if let Some(bold) = attributes.get("bold") {
            if bold.as_bool().unwrap_or(false) {
                let bold_element = self.document.create_element("strong")?;
                bold_element.set_text_content(Some(content));
                element.append_child(&bold_element)?;
                return Ok(element);
            }
        }
        
        if let Some(italic) = attributes.get("italic") {
            if italic.as_bool().unwrap_or(false) {
                let italic_element = self.document.create_element("em")?;
                italic_element.set_text_content(Some(content));
                element.append_child(&italic_element)?;
                return Ok(element);
            }
        }
        
        if let Some(underline) = attributes.get("underline") {
            if underline.as_bool().unwrap_or(false) {
                let underline_element = self.document.create_element("u")?;
                underline_element.set_text_content(Some(content));
                element.append_child(&underline_element)?;
                return Ok(element);
            }
        }
        
        // Handle color attributes
        if let Some(color) = attributes.get("color") {
            if let Some(color_str) = color.as_str() {
                let span = element.dyn_into::<HtmlElement>()?;
                span.style().set_property("color", color_str)?;
                span.set_text_content(Some(content));
                return Ok(span.into());
            }
        }
        
        // Handle background color
        if let Some(background) = attributes.get("background") {
            if let Some(bg_str) = background.as_str() {
                let span = element.dyn_into::<HtmlElement>()?;
                span.style().set_property("background-color", bg_str)?;
                span.set_text_content(Some(content));
                return Ok(span.into());
            }
        }
        
        // Handle font attributes
        if let Some(font) = attributes.get("font") {
            if let Some(font_str) = font.as_str() {
                let span = element.dyn_into::<HtmlElement>()?;
                span.style().set_property("font-family", font_str)?;
                span.set_text_content(Some(content));
                return Ok(span.into());
            }
        }
        
        if let Some(size) = attributes.get("size") {
            if let Some(size_str) = size.as_str() {
                let span = element.dyn_into::<HtmlElement>()?;
                span.style().set_property("font-size", size_str)?;
                span.set_text_content(Some(content));
                return Ok(span.into());
            }
        }
        
        // Handle links
        if let Some(link) = attributes.get("link") {
            if let Some(url) = link.as_str() {
                let link_element = self.document.create_element("a")?;
                link_element.set_attribute("href", url)?;
                link_element.set_text_content(Some(content));
                return Ok(link_element);
            }
        }
        
        // Handle block-level attributes
        if let Some(header) = attributes.get("header") {
            if let Some(level) = header.as_f64() {
                let header_tag = format!("h{}", level as i32);
                let header_element = self.document.create_element(&header_tag)?;
                header_element.set_text_content(Some(content));
                return Ok(header_element);
            }
        }
        
        if let Some(blockquote) = attributes.get("blockquote") {
            if blockquote.as_bool().unwrap_or(false) {
                let blockquote_element = self.document.create_element("blockquote")?;
                blockquote_element.set_text_content(Some(content));
                return Ok(blockquote_element);
            }
        }
        
        if let Some(code_block) = attributes.get("code-block") {
            if code_block.as_bool().unwrap_or(false) {
                let pre_element = self.document.create_element("pre")?;
                let code_element = self.document.create_element("code")?;
                code_element.set_text_content(Some(content));
                pre_element.append_child(&code_element)?;
                return Ok(pre_element);
            }
        }
        
        // Handle list attributes
        if let Some(list) = attributes.get("list") {
            if let Some(list_type) = list.as_str() {
                let list_element = match list_type {
                    "ordered" => self.document.create_element("ol")?,
                    "bullet" => self.document.create_element("ul")?,
                    _ => self.document.create_element("ul")?,
                };
                let li_element = self.document.create_element("li")?;
                li_element.set_text_content(Some(content));
                list_element.append_child(&li_element)?;
                return Ok(list_element);
            }
        }
        
        // Default: create a span with the content
        element.set_text_content(Some(content));
        Ok(element)
    }

    /// Applies formatting to a specific range in the element
    fn apply_formatting_to_range(
        &self,
        element: &HtmlElement,
        start: usize,
        length: usize,
        attributes: &AttributeMap,
    ) -> Result<(), JsValue> {
        // This is a simplified implementation
        // In a real implementation, we'd need to:
        // 1. Find the text nodes within the range
        // 2. Split text nodes if necessary
        // 3. Wrap the range in appropriate formatting elements
        // 4. Handle overlapping formatting
        
        // For now, we'll just log the operation
        web_sys::console::log_1(&format!(
            "Applying formatting to range {}-{}: {:?}",
            start, start + length, attributes
        ).into());
        
        Ok(())
    }

    /// Converts a DOM element back to a Delta
    pub fn element_to_delta(&self, element: &HtmlElement) -> Result<Delta, JsValue> {
        let mut delta = Delta::new();
        self.traverse_element_for_delta(element, &mut delta)?;
        Ok(delta)
    }

    /// Recursively traverses an element to build a Delta
    fn traverse_element_for_delta(&self, element: &HtmlElement, delta: &mut Delta) -> Result<(), JsValue> {
        let child_nodes = element.child_nodes();
        
        for i in 0..child_nodes.length() {
            if let Some(node) = child_nodes.get(i) {
                if let Ok(text_node) = node.dyn_into::<DomText>() {
                    // Handle text nodes
                    if let Some(text_content) = text_node.text_content() {
                        if !text_content.is_empty() {
                            *delta = delta.clone().insert(&text_content, None);
                        }
                    }
                } else if let Ok(element_node) = node.dyn_into::<HtmlElement>() {
                    // Handle element nodes
                    let attributes = self.extract_attributes_from_element(&element_node)?;
                    
                    if let Some(text_content) = element_node.text_content() {
                        if !text_content.is_empty() {
                            *delta = delta.clone().insert(&text_content, Some(attributes));
                        }
                    }
                    
                    // Recursively process child elements
                    self.traverse_element_for_delta(&element_node, delta)?;
                }
            }
        }
        
        Ok(())
    }

    /// Extracts formatting attributes from a DOM element
    fn extract_attributes_from_element(&self, element: &HtmlElement) -> Result<AttributeMap, JsValue> {
        let mut attributes = AttributeMap::new();
        let tag_name = element.tag_name().to_lowercase();
        
        match tag_name.as_str() {
            "strong" | "b" => {
                attributes.insert("bold".to_string(), serde_json::Value::Bool(true));
            }
            "em" | "i" => {
                attributes.insert("italic".to_string(), serde_json::Value::Bool(true));
            }
            "u" => {
                attributes.insert("underline".to_string(), serde_json::Value::Bool(true));
            }
            "a" => {
                if let Ok(href) = element.get_attribute("href") {
                    if let Some(url) = href {
                        attributes.insert("link".to_string(), serde_json::Value::String(url));
                    }
                }
            }
            "h1" => {
                attributes.insert("header".to_string(), serde_json::Value::Number(serde_json::Number::from(1)));
            }
            "h2" => {
                attributes.insert("header".to_string(), serde_json::Value::Number(serde_json::Number::from(2)));
            }
            "h3" => {
                attributes.insert("header".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
            }
            "h4" => {
                attributes.insert("header".to_string(), serde_json::Value::Number(serde_json::Number::from(4)));
            }
            "h5" => {
                attributes.insert("header".to_string(), serde_json::Value::Number(serde_json::Number::from(5)));
            }
            "h6" => {
                attributes.insert("header".to_string(), serde_json::Value::Number(serde_json::Number::from(6)));
            }
            "blockquote" => {
                attributes.insert("blockquote".to_string(), serde_json::Value::Bool(true));
            }
            "pre" => {
                attributes.insert("code-block".to_string(), serde_json::Value::Bool(true));
            }
            "ol" => {
                attributes.insert("list".to_string(), serde_json::Value::String("ordered".to_string()));
            }
            "ul" => {
                attributes.insert("list".to_string(), serde_json::Value::String("bullet".to_string()));
            }
            _ => {}
        }
        
        // Extract style attributes
        if let Ok(style) = element.style().get_property_value("color") {
            if !style.is_empty() {
                attributes.insert("color".to_string(), serde_json::Value::String(style));
            }
        }
        
        if let Ok(style) = element.style().get_property_value("background-color") {
            if !style.is_empty() {
                attributes.insert("background".to_string(), serde_json::Value::String(style));
            }
        }
        
        if let Ok(style) = element.style().get_property_value("font-family") {
            if !style.is_empty() {
                attributes.insert("font".to_string(), serde_json::Value::String(style));
            }
        }
        
        if let Ok(style) = element.style().get_property_value("font-size") {
            if !style.is_empty() {
                attributes.insert("size".to_string(), serde_json::Value::String(style));
            }
        }
        
        Ok(attributes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_dom_change_to_delta() {
        let capture = DomEventCapture {
            element: web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .create_element("div")
                .unwrap()
                .dyn_into::<HtmlElement>()
                .unwrap(),
            event_listeners: Vec::new(),
            last_content: String::new(),
            last_selection: None,
            event_callback: None,
            change_callback: None,
        };

        let changes = vec![DomChange {
            change_type: DomChangeType::Insert,
            position: 0,
            length: 5,
            content: Some("Hello".to_string()),
            attributes: None,
        }];

        let delta = capture.changes_to_delta(changes);
        assert_eq!(delta.ops().len(), 1);
        
        if let Op::Insert { insert, attributes: _ } = &delta.ops()[0] {
            assert_eq!(insert, "Hello");
        } else {
            panic!("Expected insert operation");
        }
    }

    #[wasm_bindgen_test]
    fn test_delta_to_dom_converter() {
        let converter = DeltaToDomConverter::new().unwrap();
        let document = web_sys::window().unwrap().document().unwrap();
        let element = document.create_element("div").unwrap().dyn_into::<HtmlElement>().unwrap();
        
        let mut delta = Delta::new();
        delta = delta.insert("Hello ", None);
        
        let mut bold_attrs = AttributeMap::new();
        bold_attrs.insert("bold".to_string(), serde_json::Value::Bool(true));
        delta = delta.insert("World", Some(bold_attrs));
        
        converter.apply_delta_to_element(&element, &delta).unwrap();
        
        // The element should now contain "Hello " as text and "World" in a <strong> tag
        assert!(element.inner_html().contains("Hello"));
        assert!(element.inner_html().contains("World"));
        assert!(element.inner_html().contains("<strong>"));
    }

    #[wasm_bindgen_test]
    fn test_element_to_delta_conversion() {
        let converter = DeltaToDomConverter::new().unwrap();
        let document = web_sys::window().unwrap().document().unwrap();
        let element = document.create_element("div").unwrap().dyn_into::<HtmlElement>().unwrap();
        
        // Create some HTML content
        element.set_inner_html("Hello <strong>World</strong>");
        
        let delta = converter.element_to_delta(&element).unwrap();
        
        // Should have at least one operation
        assert!(!delta.ops().is_empty());
    }

    #[wasm_bindgen_test]
    fn test_attribute_extraction() {
        let converter = DeltaToDomConverter::new().unwrap();
        let document = web_sys::window().unwrap().document().unwrap();
        let strong_element = document.create_element("strong").unwrap().dyn_into::<HtmlElement>().unwrap();
        
        let attributes = converter.extract_attributes_from_element(&strong_element).unwrap();
        
        assert_eq!(attributes.get("bold"), Some(&serde_json::Value::Bool(true)));
    }
}