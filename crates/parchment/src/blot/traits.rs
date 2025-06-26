use wasm_bindgen::prelude::*;
use web_sys::{Node, Element, HtmlElement};
use crate::scope::Scope;
use crate::collection::linked_list::LinkedList;
use std::collections::HashMap;

/// Base trait for all Blots - mirrors TypeScript Blot interface
pub trait BlotTrait {
    /// Instance method alternatives to static methods for dyn compatibility
    fn get_blot_name(&self) -> &'static str;
    fn get_tag_name(&self) -> &'static str;
    fn get_scope(&self) -> Scope;
    fn get_class_name(&self) -> Option<&'static str> { None }
    
    /// Instance methods - core Blot interface
    fn dom_node(&self) -> &Node;
    fn dom_node_mut(&mut self) -> &mut Node;
    fn parent(&self) -> Option<&dyn ParentBlotTrait>;
    fn parent_mut(&mut self) -> Option<&mut dyn ParentBlotTrait>;
    fn prev(&self) -> Option<&dyn BlotTrait>;
    fn next(&self) -> Option<&dyn BlotTrait>;
    fn scroll(&self) -> &dyn RootBlotTrait;
    
    /// Core lifecycle methods
    fn attach(&mut self);
    fn detach(&mut self);
    fn clone_blot(&self) -> Box<dyn BlotTrait>;
    fn remove(&mut self);
    
    /// Content operations
    fn length(&self) -> usize;
    fn offset(&self, root: Option<&dyn BlotTrait>) -> usize;
    fn split(&mut self, index: usize, force: bool) -> Option<Box<dyn BlotTrait>>;
    fn isolate(&mut self, index: usize, length: usize) -> Box<dyn BlotTrait>;
    
    /// Editing operations
    fn delete_at(&mut self, index: usize, length: usize);
    fn format_at(&mut self, index: usize, length: usize, name: &str, value: &JsValue);
    fn insert_at(&mut self, index: usize, value: &str, def: Option<&JsValue>);
    
    /// Update cycle methods
    fn optimize(&mut self, context: &mut OptimizeContext);
    fn update(&mut self, mutations: &[web_sys::MutationRecord], context: &mut UpdateContext);
    
    /// Replacement operations
    fn replace_with_name(&mut self, name: &str, value: &JsValue) -> Box<dyn BlotTrait>;
    fn replace_with_blot(&mut self, replacement: Box<dyn BlotTrait>) -> Box<dyn BlotTrait>;
    fn wrap_with_name(&mut self, name: &str, value: Option<&JsValue>) -> Box<dyn ParentBlotTrait>;
    fn wrap_with_parent(&mut self, wrapper: Box<dyn ParentBlotTrait>) -> Box<dyn ParentBlotTrait>;
}

/// Parent trait for blots that can contain children - mirrors TypeScript Parent interface
pub trait ParentBlotTrait: BlotTrait {
    /// Children management
    fn children(&self) -> &LinkedList<Box<dyn BlotTrait>>;
    fn children_mut(&mut self) -> &mut LinkedList<Box<dyn BlotTrait>>;
    fn dom_element(&self) -> &HtmlElement;
    fn dom_element_mut(&mut self) -> &mut HtmlElement;
    
    /// Child operations
    fn append_child(&mut self, child: Box<dyn BlotTrait>);
    fn insert_before(&mut self, child: Box<dyn BlotTrait>, ref_node: Option<&dyn BlotTrait>);
    fn remove_child(&mut self, child: &dyn BlotTrait);
    fn move_children(&mut self, target_parent: &mut dyn ParentBlotTrait, ref_node: Option<&dyn BlotTrait>);
    
    /// Navigation and search (simplified for dyn compatibility)
    fn descendant_by_matcher(&self, matcher: fn(&dyn BlotTrait) -> bool, index: usize) -> (Option<&dyn BlotTrait>, usize);
    fn descendants_by_matcher(&self, matcher: fn(&dyn BlotTrait) -> bool, index: usize, length: usize) -> Vec<&dyn BlotTrait>;
    fn path(&self, index: usize, inclusive: bool) -> Vec<(&dyn BlotTrait, usize)>;
    
    /// Parent-specific operations
    fn unwrap(&mut self);
    fn split_after(&mut self, child: &dyn BlotTrait) -> Box<dyn ParentBlotTrait>;
    fn enforce_allowed_children(&mut self);
    
    /// UI node management (for Quill UI elements)
    fn attach_ui(&mut self, node: HtmlElement);
    fn ui_node(&self) -> Option<&HtmlElement>;
}

/// Root trait for the document root - mirrors TypeScript Root interface
pub trait RootBlotTrait: ParentBlotTrait {
    /// Creation and registry operations
    fn create_by_input(&mut self, input: &Node, value: Option<&JsValue>) -> Box<dyn BlotTrait>;
    fn create_by_name(&mut self, name: &str, value: Option<&JsValue>) -> Box<dyn BlotTrait>;
    fn create_by_scope(&mut self, scope: Scope, value: Option<&JsValue>) -> Box<dyn BlotTrait>;
    
    /// Finding blots
    fn find(&self, node: &Node, bubble: bool) -> Option<&dyn BlotTrait>;
    fn query(&self, query: &str, scope: Option<Scope>) -> Option<RegistryDefinition>;
}

/// Leaf trait for terminal nodes - mirrors TypeScript Leaf interface
pub trait LeafBlotTrait: BlotTrait {
    /// Position and indexing
    fn index(&self, node: &Node, offset: usize) -> i32;
    fn position(&self, index: usize, inclusive: bool) -> (Node, usize);
    
    /// Value operations
    fn value(&self) -> JsValue;
}

/// Formattable trait for blots that support formatting - mirrors TypeScript Formattable interface
pub trait FormattableBlotTrait: BlotTrait {
    /// Formatting operations
    fn format(&mut self, name: &str, value: &JsValue);
    fn formats(&self) -> HashMap<String, JsValue>;
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
    fn class_name() -> Option<&'static str> { None }
    
    /// Hierarchy information
    fn required_container() -> Option<&'static str> { None }
    fn allowed_children() -> Vec<&'static str> { Vec::new() }
    fn default_child() -> Option<&'static str> { None }
    
    /// Value extraction from DOM
    fn value_from_dom(dom_node: &Node) -> JsValue;
}

/// Helper function to create DOM elements with proper class names
pub fn create_element_with_class(tag_name: &str, class_name: Option<&str>) -> Result<HtmlElement, JsValue> {
    let window = web_sys::window().ok_or("No global window")?;
    let document = window.document().ok_or("No document")?;
    let element = document.create_element(tag_name)?
        .dyn_into::<HtmlElement>()
        .map_err(|_| "Failed to cast to HtmlElement")?;
    
    if let Some(class) = class_name {
        element.set_class_name(class);
    }
    
    Ok(element)
}

/// Helper function for type checking blots
pub fn is_blot_type<T: BlotTrait>(blot: &dyn BlotTrait) -> bool {
    // This would need proper type checking in a real implementation
    // For now, we'll use blot_name comparison
    std::any::type_name::<T>().contains(&format!("::{}", T::blot_name()))
}