use crate::blot::traits_simple::{BlotTrait, LeafBlotTrait};
use crate::dom::Dom;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::{Element, Node};

/// EmbedBlot represents an embedded element (img, video, iframe, etc.)
/// These are typically self-closing or void elements that don't contain text content
#[wasm_bindgen]
pub struct EmbedBlot {
    /// The underlying DOM element (img, video, iframe, etc.)
    dom_node: Element,
    /// The value/content of this embed (could be URL, data, etc.)
    value: String,
}

#[wasm_bindgen]
impl EmbedBlot {
    /// Create a new EmbedBlot with optional DOM element
    #[wasm_bindgen(constructor)]
    pub fn new(element: Option<Element>) -> Result<EmbedBlot, JsValue> {
        let dom_node = match element {
            Some(el) => el,
            None => Dom::create_element("span")?, // Default to span for generic embeds
        };

        Ok(EmbedBlot {
            dom_node,
            value: String::new(),
        })
    }

    /// Create an EmbedBlot from an existing Element
    pub fn from_element(element: Element) -> EmbedBlot {
        // Extract value from common attributes
        let value = element
            .get_attribute("src")
            .or_else(|| element.get_attribute("href"))
            .or_else(|| element.get_attribute("data"))
            .unwrap_or_default();

        EmbedBlot {
            dom_node: element,
            value,
        }
    }

    /// Create an EmbedBlot with a specific tag and value
    pub fn with_tag_and_value(tag: &str, value: &str) -> Result<EmbedBlot, JsValue> {
        let element = Dom::create_element(tag)?;

        // Set appropriate attribute based on tag
        match tag.to_lowercase().as_str() {
            "img" | "video" | "audio" | "iframe" => {
                element.set_attribute("src", value)?;
            }
            "a" => {
                element.set_attribute("href", value)?;
            }
            _ => {
                element.set_attribute("data", value)?;
            }
        }

        Ok(EmbedBlot {
            dom_node: element,
            value: value.to_string(),
        })
    }

    /// Get the underlying DOM element
    pub fn dom_element(&self) -> Element {
        self.dom_node.clone()
    }

    /// Convert to generic Node for DOM operations
    pub fn as_node(&self) -> Node {
        self.dom_node.clone().into()
    }

    /// Static methods for blot identification (WASM-compatible)
    pub fn blot_name() -> String {
        "embed".to_string()
    }

    pub fn tag_name() -> String {
        "SPAN".to_string() // Default, but typically overridden by specific embed types
    }

    pub fn scope() -> Scope {
        Scope::EmbedBlot
    }

    /// Get the value (src, href, data, etc.) - WASM-compatible
    pub fn get_value(&self) -> String {
        self.value.clone()
    }

    /// Set the value and update the appropriate DOM attribute
    pub fn set_value_and_attribute(&mut self, value: &str) {
        self.value = value.to_string();

        // Update the appropriate DOM attribute based on tag
        let tag_name = self.dom_node.tag_name().to_lowercase();
        let attr_name = match tag_name.as_str() {
            "img" | "video" | "audio" | "iframe" => "src",
            "a" => "href",
            _ => "data",
        };

        let _ = self.dom_node.set_attribute(attr_name, value);
    }
}

impl BlotTrait for EmbedBlot {
    fn get_blot_name(&self) -> &'static str {
        "embed"
    }

    fn get_tag_name(&self) -> &'static str {
        // Return generic tag name for trait compatibility
        "EMBED"
    }

    fn get_scope(&self) -> Scope {
        Scope::EmbedBlot
    }

    fn dom_node(&self) -> &Node {
        self.dom_node.as_ref()
    }

    fn length(&self) -> usize {
        // Embed blots typically have a length of 1 (they represent a single embedded object)
        1
    }

    fn attach(&mut self) {
        // Embed blot attachment logic - register with registry
        // This is called when the blot is added to the document
    }

    fn detach(&mut self) {
        // Embed blot detachment logic - unregister from registry
        // This is called when the blot is removed from the document
    }

    fn remove(&mut self) {
        self.detach();
        if let Some(parent) = self.dom_node.parent_node() {
            let _ = parent.remove_child(&self.dom_node);
        }
    }

    fn delete_at(&mut self, index: usize, length: usize) {
        // For embed blots, any deletion that affects them should remove the entire blot
        if index == 0 && length >= 1 {
            self.remove();
        }
        // Otherwise, ignore the deletion (embed blots are atomic)
    }

    fn insert_at(&mut self, _index: usize, _value: &str) {
        // Embed blots don't support text insertion - they are atomic elements
        // This is a no-op for embed blots
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn build_children(&mut self) -> Result<(), JsValue> {
        // Embed blots don't have children - they are leaf nodes
        // This is a no-op for embed blots
        Ok(())
    }
}

impl LeafBlotTrait for EmbedBlot {
    fn value(&self) -> String {
        self.value.clone()
    }

    fn set_value(&mut self, value: &str) {
        self.set_value_and_attribute(value);
    }
}

impl EmbedBlot {
    /// Create an image embed blot
    pub fn create_image(src: &str, alt: Option<&str>) -> Result<EmbedBlot, JsValue> {
        let element = Dom::create_element("img")?;
        element.set_attribute("src", src)?;

        if let Some(alt_text) = alt {
            element.set_attribute("alt", alt_text)?;
        }

        Ok(EmbedBlot {
            dom_node: element,
            value: src.to_string(),
        })
    }

    /// Create a video embed blot
    pub fn create_video(src: &str, controls: bool) -> Result<EmbedBlot, JsValue> {
        let element = Dom::create_element("video")?;
        element.set_attribute("src", src)?;

        if controls {
            element.set_attribute("controls", "")?;
        }

        Ok(EmbedBlot {
            dom_node: element,
            value: src.to_string(),
        })
    }

    /// Create a link embed blot
    pub fn create_link(href: &str, text: Option<&str>) -> Result<EmbedBlot, JsValue> {
        let element = Dom::create_element("a")?;
        element.set_attribute("href", href)?;

        if let Some(link_text) = text {
            element.set_text_content(Some(link_text));
        }

        Ok(EmbedBlot {
            dom_node: element,
            value: href.to_string(),
        })
    }

    /// Create an iframe embed blot
    pub fn create_iframe(
        src: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<EmbedBlot, JsValue> {
        let element = Dom::create_element("iframe")?;
        element.set_attribute("src", src)?;

        if let Some(w) = width {
            element.set_attribute("width", &w.to_string())?;
        }

        if let Some(h) = height {
            element.set_attribute("height", &h.to_string())?;
        }

        Ok(EmbedBlot {
            dom_node: element,
            value: src.to_string(),
        })
    }

    /// Create a line break embed blot
    pub fn create_break() -> Result<EmbedBlot, JsValue> {
        let element = Dom::create_element("br")?;

        Ok(EmbedBlot {
            dom_node: element,
            value: "\n".to_string(), // Line breaks represent newlines
        })
    }

    /// Create a horizontal rule embed blot
    pub fn create_horizontal_rule() -> Result<EmbedBlot, JsValue> {
        let element = Dom::create_element("hr")?;

        Ok(EmbedBlot {
            dom_node: element,
            value: "---".to_string(), // HR represents a divider
        })
    }

    /// Check if this embed blot represents a specific type
    pub fn is_image(&self) -> bool {
        self.dom_node.tag_name().to_lowercase() == "img"
    }

    pub fn is_video(&self) -> bool {
        self.dom_node.tag_name().to_lowercase() == "video"
    }

    pub fn is_link(&self) -> bool {
        self.dom_node.tag_name().to_lowercase() == "a"
    }

    pub fn is_iframe(&self) -> bool {
        self.dom_node.tag_name().to_lowercase() == "iframe"
    }

    pub fn is_break(&self) -> bool {
        self.dom_node.tag_name().to_lowercase() == "br"
    }

    pub fn is_horizontal_rule(&self) -> bool {
        self.dom_node.tag_name().to_lowercase() == "hr"
    }
}
