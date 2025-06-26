use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Window};

/// Get the browser's window object
pub fn window() -> Result<Window, JsValue> {
    web_sys::window().ok_or_else(|| JsValue::from_str("no global `window` exists"))
}

/// Get the browser's document object  
pub fn document() -> Result<Document, JsValue> {
    window()?
        .document()
        .ok_or_else(|| JsValue::from_str("should have a document on window"))
}

/// Create a new DOM element with the given tag name
pub fn create_element(tag_name: &str) -> Result<Element, JsValue> {
    document()?.create_element(tag_name)
}

/// Create a text node with the given content  
pub fn create_text_node(content: &str) -> Result<web_sys::Text, JsValue> {
    Ok(document()?.create_text_node(content))
}

/// Get an element by its ID
pub fn get_element_by_id(id: &str) -> Result<Option<Element>, JsValue> {
    Ok(document()?.get_element_by_id(id))
}

/// Set panic hook for better error messages in development
pub fn set_panic_hook() {
    // Note: console_error_panic_hook feature not currently enabled
    // Uncomment the following lines if you add the console_error_panic_hook dependency
    // #[cfg(feature = "console_error_panic_hook")]
    // console_error_panic_hook::set_once();
}
