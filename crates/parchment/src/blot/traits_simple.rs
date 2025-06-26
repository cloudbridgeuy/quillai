use crate::collection::linked_list::LinkedList;
use crate::scope::Scope;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, Node};

/// Simplified base trait for all Blots - focus on dyn compatibility
pub trait BlotTrait {
    /// Instance method alternatives to static methods for dyn compatibility
    fn get_blot_name(&self) -> &'static str;
    fn get_tag_name(&self) -> &'static str;
    fn get_scope(&self) -> Scope;
    fn get_class_name(&self) -> Option<&'static str> {
        None
    }

    /// Core interface
    fn dom_node(&self) -> &Node;
    fn length(&self) -> usize;

    /// Basic lifecycle
    fn attach(&mut self);
    fn detach(&mut self);
    fn remove(&mut self);

    /// Basic editing
    fn delete_at(&mut self, index: usize, length: usize);
    fn insert_at(&mut self, index: usize, value: &str);

    /// Support for downcasting - needed for tree navigation
    fn as_any(&self) -> &dyn std::any::Any;

    /// Build children from DOM (for parent blots) - optional default implementation
    fn build_children(&mut self) -> Result<(), JsValue> {
        // Default implementation does nothing - only parent blots override this
        Ok(())
    }
}

/// Parent trait for blots that can contain children - with LinkedList support
pub trait ParentBlotTrait: BlotTrait {
    /// Children management
    fn children(&self) -> &LinkedList<Box<dyn BlotTrait>>;
    fn children_mut(&mut self) -> &mut LinkedList<Box<dyn BlotTrait>>;
    fn children_count(&self) -> usize {
        self.children().length as usize
    }
    fn is_empty(&self) -> bool {
        self.children().length == 0
    }

    /// DOM element access
    fn dom_element(&self) -> &HtmlElement;

    /// Child operations - mirrors TypeScript ParentBlot API
    fn append_child(&mut self, child: Box<dyn BlotTrait>) -> Result<(), JsValue>;
    fn insert_before(
        &mut self,
        child: Box<dyn BlotTrait>,
        ref_blot: Option<&dyn BlotTrait>,
    ) -> Result<(), JsValue>;
    fn remove_child(&mut self, child: &dyn BlotTrait) -> Result<Box<dyn BlotTrait>, JsValue>;

    /// Tree navigation methods
    fn descendant(
        &self,
        matcher: fn(&dyn BlotTrait) -> bool,
        index: Option<usize>,
    ) -> Option<&dyn BlotTrait>;
    fn descendants(
        &self,
        matcher: fn(&dyn BlotTrait) -> bool,
        index: Option<usize>,
        length: Option<usize>,
    ) -> Vec<&dyn BlotTrait>;
    fn path(&self, index: usize) -> Vec<(&dyn BlotTrait, usize)>;

    /// Basic child operations (convenience methods)
    fn append_text(&mut self, text: &str) -> Result<(), JsValue>;
    fn clear(&mut self);
    fn text_content(&self) -> String;
}

/// Leaf trait for terminal nodes - simplified
pub trait LeafBlotTrait: BlotTrait {
    /// Value operations
    fn value(&self) -> String;
    fn set_value(&mut self, value: &str);
}

/// Context objects for update cycles
#[derive(Default)]
pub struct OptimizeContext {
    pub data: HashMap<String, JsValue>,
}

#[derive(Default)]
pub struct UpdateContext {
    pub data: HashMap<String, JsValue>,
}

/// Registry definition - simplified version
#[derive(Clone)]
pub struct RegistryDefinition {
    pub blot_name: String,
    pub tag_name: String,
    pub scope: Scope,
    pub class_name: Option<String>,
}

/// Blot constructor trait for static methods
pub trait BlotConstructor {
    /// Create DOM node
    fn create(value: Option<&JsValue>) -> Result<Node, JsValue>;

    /// Static metadata
    fn blot_name() -> &'static str;
    fn tag_name() -> &'static str;
    fn scope() -> Scope;
    fn class_name() -> Option<&'static str> {
        None
    }

    /// Value extraction from DOM
    fn value_from_dom(dom_node: &Node) -> JsValue;
}

/// Helper function to create DOM elements with proper class names
pub fn create_element_with_class(
    tag_name: &str,
    class_name: Option<&str>,
) -> Result<HtmlElement, JsValue> {
    let window = web_sys::window().ok_or("No global window")?;
    let document = window.document().ok_or("No document")?;
    let element = document
        .create_element(tag_name)?
        .dyn_into::<HtmlElement>()
        .map_err(|_| "Failed to cast to HtmlElement")?;

    if let Some(class) = class_name {
        element.set_class_name(class);
    }

    Ok(element)
}
