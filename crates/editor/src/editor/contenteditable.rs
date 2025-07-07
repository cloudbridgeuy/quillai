use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement, Event, KeyboardEvent};
use dioxus::prelude::*;
use std::collections::HashMap;
use crate::editor::dom_integration::{DomEventCapture, CapturedDomEvent, DomEventType};
use crate::editor::delta_operations::DeltaOperationConverter;
use quillai_delta::Delta;

/// ContentEditable state management and browser compatibility layer
///
/// This component provides a robust wrapper around contenteditable functionality,
/// handling browser-specific quirks, IME input, and providing a consistent
/// interface for text editing operations.
#[derive(Clone, PartialEq)]
pub struct ContentEditableManager {
    /// Whether the element is currently editable
    pub is_editable: bool,
    /// Whether the element is in readonly mode
    pub is_readonly: bool,
    /// Whether the element currently has focus
    pub has_focus: bool,
    /// Current IME composition state
    pub ime_state: ImeState,
    /// Browser-specific configuration
    pub browser_config: BrowserConfig,
    /// Event handlers configuration
    pub event_config: EventConfig,
}

/// IME (Input Method Editor) state tracking
#[derive(Clone, PartialEq)]
pub struct ImeState {
    /// Whether IME composition is currently active
    pub is_composing: bool,
    /// Current composition text
    pub composition_text: String,
    /// Composition start position
    pub composition_start: usize,
    /// Composition end position
    pub composition_end: usize,
}

/// Browser-specific configuration and workarounds
#[derive(Clone, PartialEq)]
pub struct BrowserConfig {
    /// Detected browser type
    pub browser_type: BrowserType,
    /// Whether to use modern input events
    pub use_input_events: bool,
    /// Whether to use beforeinput events
    pub use_beforeinput_events: bool,
    /// Whether to apply Safari-specific workarounds
    pub apply_safari_fixes: bool,
    /// Whether to apply Firefox-specific workarounds
    pub apply_firefox_fixes: bool,
    /// Whether to apply Chrome-specific workarounds
    pub apply_chrome_fixes: bool,
}

/// Event handling configuration
#[derive(Clone, PartialEq)]
pub struct EventConfig {
    /// Whether to capture input events
    pub capture_input: bool,
    /// Whether to capture keyboard events
    pub capture_keyboard: bool,
    /// Whether to capture composition events
    pub capture_composition: bool,
    /// Whether to capture focus events
    pub capture_focus: bool,
    /// Whether to prevent default browser behavior
    pub prevent_defaults: bool,
}

/// Detected browser types for applying specific workarounds
#[derive(Clone, PartialEq)]
pub enum BrowserType {
    Chrome,
    Firefox,
    Safari,
    Edge,
    Unknown,
}

/// ContentEditable component properties
#[derive(Props, Clone, PartialEq)]
pub struct ContentEditableProps {
    /// Initial content for the editor
    #[props(default = String::new())]
    pub initial_content: String,
    
    /// Whether the editor is editable
    #[props(default = true)]
    pub editable: bool,
    
    /// Whether the editor is in readonly mode
    #[props(default = false)]
    pub readonly: bool,
    
    /// Placeholder text when empty
    #[props(default = String::new())]
    pub placeholder: String,
    
    /// CSS classes to apply
    #[props(default = String::new())]
    pub class: String,
    
    /// Callback for content changes
    #[props(default)]
    pub on_change: Option<EventHandler<Delta>>,
    
    /// Callback for focus events
    #[props(default)]
    pub on_focus: Option<EventHandler<()>>,
    
    /// Callback for blur events
    #[props(default)]
    pub on_blur: Option<EventHandler<()>>,
    
    /// Callback for key events
    #[props(default)]
    pub on_key: Option<EventHandler<KeyboardEvent>>,
}

impl Default for ContentEditableManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentEditableManager {
    /// Create a new ContentEditable manager with default settings
    pub fn new() -> Self {
        let browser_config = BrowserConfig::detect_browser();
        
        Self {
            is_editable: true,
            is_readonly: false,
            has_focus: false,
            ime_state: ImeState::new(),
            browser_config,
            event_config: EventConfig::default(),
        }
    }

    /// Set the editable state
    pub fn set_editable(&mut self, editable: bool) {
        self.is_editable = editable;
    }

    /// Set the readonly state
    pub fn set_readonly(&mut self, readonly: bool) {
        self.is_readonly = readonly;
        if readonly {
            self.is_editable = false;
        }
    }

    /// Set focus state
    pub fn set_focus(&mut self, has_focus: bool) {
        self.has_focus = has_focus;
    }

    /// Apply contenteditable attributes to an element
    pub fn apply_to_element(&self, element: &HtmlElement) -> Result<(), JsValue> {
        // Set contenteditable attribute
        if self.is_readonly {
            element.set_attribute("contenteditable", "false")?;
        } else if self.is_editable {
            element.set_attribute("contenteditable", "true")?;
        } else {
            element.set_attribute("contenteditable", "false")?;
        }

        // Set other attributes
        element.set_attribute("role", "textbox")?;
        element.set_attribute("aria-multiline", "true")?;
        
        if self.is_readonly {
            element.set_attribute("aria-readonly", "true")?;
        } else {
            element.remove_attribute("aria-readonly")?;
        }

        // Apply browser-specific fixes
        self.apply_browser_fixes(element)?;

        Ok(())
    }

    /// Apply browser-specific workarounds
    fn apply_browser_fixes(&self, element: &HtmlElement) -> Result<(), JsValue> {
        match self.browser_config.browser_type {
            BrowserType::Safari => {
                if self.browser_config.apply_safari_fixes {
                    self.apply_safari_fixes(element)?;
                }
            }
            BrowserType::Firefox => {
                if self.browser_config.apply_firefox_fixes {
                    self.apply_firefox_fixes(element)?;
                }
            }
            BrowserType::Chrome => {
                if self.browser_config.apply_chrome_fixes {
                    self.apply_chrome_fixes(element)?;
                }
            }
            BrowserType::Edge => {
                // Edge generally follows Chrome behavior
                if self.browser_config.apply_chrome_fixes {
                    self.apply_chrome_fixes(element)?;
                }
            }
            BrowserType::Unknown => {
                // Apply conservative fixes
                self.apply_generic_fixes(element)?;
            }
        }

        Ok(())
    }

    /// Apply Safari-specific fixes
    fn apply_safari_fixes(&self, element: &HtmlElement) -> Result<(), JsValue> {
        // Safari has issues with certain contenteditable behaviors
        element.style().set_property("word-wrap", "break-word")?;
        element.style().set_property("-webkit-user-select", "text")?;
        element.style().set_property("-webkit-user-modify", "read-write-plaintext-only")?;
        
        Ok(())
    }

    /// Apply Firefox-specific fixes
    fn apply_firefox_fixes(&self, element: &HtmlElement) -> Result<(), JsValue> {
        // Firefox has different selection behavior
        element.style().set_property("-moz-user-select", "text")?;
        element.style().set_property("white-space", "pre-wrap")?;
        
        Ok(())
    }

    /// Apply Chrome-specific fixes
    fn apply_chrome_fixes(&self, element: &HtmlElement) -> Result<(), JsValue> {
        // Chrome generally works well, but some edge cases
        element.style().set_property("user-select", "text")?;
        element.style().set_property("white-space", "pre-wrap")?;
        
        Ok(())
    }

    /// Apply generic fixes for unknown browsers
    fn apply_generic_fixes(&self, element: &HtmlElement) -> Result<(), JsValue> {
        element.style().set_property("user-select", "text")?;
        element.style().set_property("white-space", "pre-wrap")?;
        element.style().set_property("word-wrap", "break-word")?;
        
        Ok(())
    }

    /// Handle focus event
    pub fn handle_focus(&mut self, event: &web_sys::FocusEvent) {
        self.has_focus = true;
        
        // Apply focus-specific styling or behavior
        if let Some(target) = event.target() {
            if let Ok(element) = target.dyn_into::<HtmlElement>() {
                let _ = element.style().set_property("outline", "none");
            }
        }
    }

    /// Handle blur event
    pub fn handle_blur(&mut self, event: &web_sys::FocusEvent) {
        self.has_focus = false;
        
        // Clean up any composition state
        self.ime_state.reset();
        
        // Remove focus-specific styling
        if let Some(target) = event.target() {
            if let Ok(element) = target.dyn_into::<HtmlElement>() {
                let _ = element.style().remove_property("outline");
            }
        }
    }

    /// Handle composition start (IME input beginning)
    pub fn handle_composition_start(&mut self, event: &web_sys::CompositionEvent) {
        self.ime_state.is_composing = true;
        self.ime_state.composition_text = event.data().unwrap_or_default();
        
        // Store composition range if available
        if let Some(target) = event.target() {
            if let Ok(element) = target.dyn_into::<HtmlElement>() {
                if let Ok(Some(selection)) = web_sys::window()
                    .and_then(|w| w.get_selection())
                {
                    if selection.range_count() > 0 {
                        if let Ok(range) = selection.get_range_at(0) {
                            self.ime_state.composition_start = range.start_offset() as usize;
                            self.ime_state.composition_end = range.end_offset() as usize;
                        }
                    }
                }
            }
        }
    }

    /// Handle composition update (IME input changing)
    pub fn handle_composition_update(&mut self, event: &web_sys::CompositionEvent) {
        if self.ime_state.is_composing {
            self.ime_state.composition_text = event.data().unwrap_or_default();
        }
    }

    /// Handle composition end (IME input completed)
    pub fn handle_composition_end(&mut self, event: &web_sys::CompositionEvent) {
        self.ime_state.composition_text = event.data().unwrap_or_default();
        self.ime_state.is_composing = false;
        
        // The final composition text will be handled by the input event
    }

    /// Check if an input event should be processed
    pub fn should_process_input(&self, event: &Event) -> bool {
        // Don't process input during IME composition unless it's the final result
        if self.ime_state.is_composing {
            // Only process if this is a composition end event
            return false; // Simplified for now - proper IME handling would check event type
        }
        
        // Don't process if readonly
        if self.is_readonly {
            return false;
        }
        
        // Don't process if not editable
        if !self.is_editable {
            return false;
        }
        
        true
    }

    /// Get the current text content from an element
    pub fn get_text_content(&self, element: &HtmlElement) -> String {
        element.text_content().unwrap_or_default()
    }

    /// Set text content in an element
    pub fn set_text_content(&self, element: &HtmlElement, content: &str) -> Result<(), JsValue> {
        element.set_text_content(Some(content));
        Ok(())
    }

    /// Get current selection range
    pub fn get_selection_range(&self) -> Option<(usize, usize)> {
        if let Ok(Some(selection)) = web_sys::window()
            .and_then(|w| w.get_selection())
        {
            if selection.range_count() > 0 {
                if let Ok(range) = selection.get_range_at(0) {
                    return Some((range.start_offset() as usize, range.end_offset() as usize));
                }
            }
        }
        None
    }

    /// Set selection range
    pub fn set_selection_range(&self, element: &HtmlElement, start: usize, end: usize) -> Result<(), JsValue> {
        if let Ok(Some(selection)) = web_sys::window()
            .and_then(|w| w.get_selection())
        {
            if let Ok(Some(document)) = web_sys::window()
                .and_then(|w| w.document())
            {
                let range = document.create_range()?;
                
                // Find text nodes and set range
                if let Some(text_node) = self.find_text_node_at_position(element, start) {
                    range.set_start(&text_node, start as u32)?;
                    
                    if let Some(end_text_node) = self.find_text_node_at_position(element, end) {
                        range.set_end(&end_text_node, end as u32)?;
                    } else {
                        range.set_end(&text_node, end as u32)?;
                    }
                    
                    selection.remove_all_ranges()?;
                    selection.add_range(&range)?;
                }
            }
        }
        
        Ok(())
    }

    /// Find text node at a specific position
    fn find_text_node_at_position(&self, element: &HtmlElement, position: usize) -> Option<web_sys::Text> {
        let mut current_position = 0;
        self.find_text_node_recursive(&element.clone().into(), position, &mut current_position)
    }

    /// Recursively find text node at position
    fn find_text_node_recursive(&self, node: &web_sys::Node, target_position: usize, current_position: &mut usize) -> Option<web_sys::Text> {
        if node.node_type() == web_sys::Node::TEXT_NODE {
            if let Ok(text_node) = node.clone().dyn_into::<web_sys::Text>() {
                let text_length = text_node.text_content().unwrap_or_default().len();
                if *current_position <= target_position && target_position < *current_position + text_length {
                    return Some(text_node);
                }
                *current_position += text_length;
            }
        } else {
            let children = node.child_nodes();
            for i in 0..children.length() {
                if let Some(child) = children.get(i) {
                    if let Some(found) = self.find_text_node_recursive(&child, target_position, current_position) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }
}

impl ImeState {
    /// Create a new IME state
    pub fn new() -> Self {
        Self {
            is_composing: false,
            composition_text: String::new(),
            composition_start: 0,
            composition_end: 0,
        }
    }

    /// Reset IME state
    pub fn reset(&mut self) {
        self.is_composing = false;
        self.composition_text.clear();
        self.composition_start = 0;
        self.composition_end = 0;
    }
}

impl BrowserConfig {
    /// Detect the current browser and create appropriate configuration
    pub fn detect_browser() -> Self {
        let browser_type = Self::detect_browser_type();
        
        Self {
            browser_type: browser_type.clone(),
            use_input_events: Self::supports_input_events(),
            use_beforeinput_events: Self::supports_beforeinput_events(),
            apply_safari_fixes: matches!(browser_type, BrowserType::Safari),
            apply_firefox_fixes: matches!(browser_type, BrowserType::Firefox),
            apply_chrome_fixes: matches!(browser_type, BrowserType::Chrome | BrowserType::Edge),
        }
    }

    /// Detect browser type from user agent
    fn detect_browser_type() -> BrowserType {
        if let Some(navigator) = web_sys::window().and_then(|w| w.navigator().user_agent().ok()) {
            let user_agent = navigator.to_lowercase();
            
            if user_agent.contains("chrome") && !user_agent.contains("edg") {
                BrowserType::Chrome
            } else if user_agent.contains("firefox") {
                BrowserType::Firefox
            } else if user_agent.contains("safari") && !user_agent.contains("chrome") {
                BrowserType::Safari
            } else if user_agent.contains("edg") {
                BrowserType::Edge
            } else {
                BrowserType::Unknown
            }
        } else {
            BrowserType::Unknown
        }
    }

    /// Check if browser supports modern input events
    fn supports_input_events() -> bool {
        // Most modern browsers support input events
        true
    }

    /// Check if browser supports beforeinput events
    fn supports_beforeinput_events() -> bool {
        // Check if beforeinput is supported
        if let Some(window) = web_sys::window() {
            if let Ok(Some(document)) = window.document() {
                if let Ok(element) = document.create_element("div") {
                    return js_sys::Reflect::has(&element, &JsValue::from_str("onbeforeinput")).unwrap_or(false);
                }
            }
        }
        false
    }
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            capture_input: true,
            capture_keyboard: true,
            capture_composition: true,
            capture_focus: true,
            prevent_defaults: false,
        }
    }
}

/// ContentEditable Dioxus component
#[component]
pub fn ContentEditable(props: ContentEditableProps) -> Element {
    let mut manager = use_signal(|| ContentEditableManager::new());
    let mut content = use_signal(|| props.initial_content.clone());
    let dom_capture = use_signal(|| None::<DomEventCapture>);
    let delta_converter = use_signal(|| DeltaOperationConverter::new());

    // Update manager state based on props
    use_effect(move || {
        let mut mgr = manager.write();
        mgr.set_editable(props.editable && !props.readonly);
        mgr.set_readonly(props.readonly);
    });

    // Handle input events - simplified for now
    let handle_input = move |event: FormEvent| {
        let new_content = event.value().clone();
        let old_content = content.read().clone();
        
        if new_content != old_content {
            // Simple Delta creation for text changes
            let delta = Delta::new().delete(old_content.len()).insert(&new_content, None);
            content.set(new_content);
            
            if let Some(on_change) = &props.on_change {
                on_change.call(delta);
            }
        }
    };

    // Handle focus events - simplified
    let handle_focus = move |_event: FocusEvent| {
        if let Some(on_focus) = &props.on_focus {
            on_focus.call(());
        }
    };

    // Handle blur events - simplified
    let handle_blur = move |_event: FocusEvent| {
        if let Some(on_blur) = &props.on_blur {
            on_blur.call(());
        }
    };

    // Handle keyboard events - simplified  
    let handle_keydown = move |event: KeyboardEvent| {
        if let Some(on_key) = &props.on_key {
            on_key.call(event);
        }
    };

    // Setup DOM event capture when element is mounted
    let setup_dom_capture = move |element: web_sys::Element| {
        if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
            let mgr = manager.read();
            let _ = mgr.apply_to_element(&html_element);
            
            // Setup DOM event capture
            if let Ok(capture) = DomEventCapture::new(html_element.clone()) {
                dom_capture.set(Some(capture));
            }
        }
    };

    rsx! {
        div {
            class: format!("contenteditable-wrapper {}", props.class),
            contenteditable: if props.readonly { "false" } else if props.editable { "true" } else { "false" },
            role: "textbox",
            "aria-multiline": "true",
            "aria-readonly": if props.readonly { "true" } else { "false" },
            // placeholder: props.placeholder, // placeholder doesn't exist on div elements
            
            oninput: handle_input,
            onfocus: handle_focus,
            onblur: handle_blur,
            // Composition events temporarily removed for Phase 1.2
            // Will be added back in Phase 2 with proper IME handling
            onkeydown: handle_keydown,
            
            onmounted: move |event| {
                if let Some(element) = event.as_element() {
                    setup_dom_capture(element.clone());
                }
            },
            
            "{content}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contenteditable_manager_creation() {
        let manager = ContentEditableManager::new();
        assert!(manager.is_editable);
        assert!(!manager.is_readonly);
        assert!(!manager.has_focus);
        assert!(!manager.ime_state.is_composing);
    }

    #[test]
    fn test_editable_state_management() {
        let mut manager = ContentEditableManager::new();
        
        manager.set_editable(false);
        assert!(!manager.is_editable);
        
        manager.set_readonly(true);
        assert!(manager.is_readonly);
        assert!(!manager.is_editable); // Should be false when readonly
    }

    #[test]
    fn test_ime_state() {
        let mut ime_state = ImeState::new();
        assert!(!ime_state.is_composing);
        assert!(ime_state.composition_text.is_empty());
        
        ime_state.is_composing = true;
        ime_state.composition_text = "test".to_string();
        
        ime_state.reset();
        assert!(!ime_state.is_composing);
        assert!(ime_state.composition_text.is_empty());
    }

    #[test]
    fn test_browser_detection() {
        let config = BrowserConfig::detect_browser();
        // Browser detection should work without errors
        assert!(matches!(config.browser_type, 
            BrowserType::Chrome | BrowserType::Firefox | BrowserType::Safari | BrowserType::Edge | BrowserType::Unknown
        ));
    }

    #[test]
    fn test_event_config_default() {
        let config = EventConfig::default();
        assert!(config.capture_input);
        assert!(config.capture_keyboard);
        assert!(config.capture_composition);
        assert!(config.capture_focus);
        assert!(!config.prevent_defaults);
    }
}