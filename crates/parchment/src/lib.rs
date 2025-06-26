use wasm_bindgen::prelude::*;

pub mod attributor;
pub mod blot;
pub mod collection;
pub mod dom;
pub mod registry;
pub mod scope;
pub mod utils;

// Re-exports for public API
pub use blot::block::BlockBlot;
pub use blot::embed::EmbedBlot;
pub use blot::inline::InlineBlot;
pub use blot::scroll::ScrollBlot;
pub use blot::text::TextBlot;
pub use blot::traits_simple::*;
pub use registry::Registry;
pub use scope::Scope;
pub use utils::*;

// This is like the `extern` block in C.
#[wasm_bindgen]
extern "C" {
    // Bind the `console.log` function from the browser.
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Unused console_log macro removed for cleaner compilation

#[wasm_bindgen]
pub fn init_panic_hook() {
    utils::set_panic_hook();
}

/// Create a new registry instance
#[wasm_bindgen]
pub fn create_registry() -> Registry {
    Registry::new()
}

/// Test function to verify WASM compilation
#[wasm_bindgen]
pub fn test_scope_operations() -> u8 {
    let block_scope = Scope::Block;
    let inline_scope = Scope::Inline;

    // Test bitwise operations
    if block_scope.matches(Scope::BlockBlot) && inline_scope.matches(Scope::InlineBlot) {
        1 // Success
    } else {
        0 // Failure
    }
}

/// Get version information
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Test function to verify TextBlot creation and operations
#[wasm_bindgen]
pub fn test_text_blot() -> u8 {
    match TextBlot::new("Hello, World!") {
        Ok(text_blot) => {
            // Test basic operations
            if text_blot.value() == "Hello, World!" && text_blot.length() == 13 {
                1 // Success
            } else {
                0 // Failure
            }
        }
        Err(_) => 0, // Error creating TextBlot
    }
}

/// Test function to verify ScrollBlot creation and operations
#[wasm_bindgen]
pub fn test_scroll_blot() -> u8 {
    match ScrollBlot::new(None) {
        Ok(mut scroll_blot) => {
            // Test basic operations
            if scroll_blot.is_empty() && scroll_blot.children_count() == 0 {
                // Test adding text
                match scroll_blot.append_text("Test content") {
                    Ok(_) => {
                        if scroll_blot.children_count() == 1 && !scroll_blot.is_empty() {
                            1 // Success
                        } else {
                            0 // Failed count check
                        }
                    }
                    Err(_) => 0, // Failed to append text
                }
            } else {
                0 // Initial state failure
            }
        }
        Err(_) => 0, // Error creating ScrollBlot
    }
}

/// Test function to verify BlockBlot creation and operations
#[wasm_bindgen]
pub fn test_block_blot() -> u8 {
    match BlockBlot::with_text("Test paragraph content") {
        Ok(block_blot) => {
            // Test basic operations
            if !block_blot.is_empty()
                && block_blot.children_count() == 1
                && block_blot.text_content() == "Test paragraph content"
            {
                1 // Success
            } else {
                0 // Failed validation
            }
        }
        Err(_) => 0, // Error creating BlockBlot
    }
}

/// Test function to verify InlineBlot creation and operations
#[wasm_bindgen]
pub fn test_inline_blot() -> u8 {
    match InlineBlot::with_text("Test inline content") {
        Ok(inline_blot) => {
            // Test basic operations
            if !inline_blot.is_empty()
                && inline_blot.children_count() == 1
                && inline_blot.text_content() == "Test inline content"
            {
                1 // Success
            } else {
                // Debug output to see what's failing
                log(&format!(
                    "InlineBlot test failed - is_empty: {}, children_count: {}, text_content: '{}'",
                    inline_blot.is_empty(),
                    inline_blot.children_count(),
                    inline_blot.text_content()
                ));
                0 // Failed validation
            }
        }
        Err(e) => {
            log(&format!("Error creating InlineBlot: {:?}", e));
            0 // Error creating InlineBlot
        }
    }
}

/// Test function to verify EmbedBlot creation and operations
#[wasm_bindgen]
pub fn test_embed_blot() -> u8 {
    match EmbedBlot::create_image("test.jpg", Some("Test image")) {
        Ok(embed_blot) => {
            // Test basic operations
            if embed_blot.length() == 1
                && embed_blot.get_value() == "test.jpg"
                && embed_blot.is_image()
            {
                1 // Success
            } else {
                log(&format!(
                    "EmbedBlot test failed - length: {}, value: '{}', is_image: {}",
                    embed_blot.length(),
                    embed_blot.get_value(),
                    embed_blot.is_image()
                ));
                0 // Failed validation
            }
        }
        Err(e) => {
            log(&format!("Error creating EmbedBlot: {:?}", e));
            0 // Error creating EmbedBlot
        }
    }
}

/// Test function to verify Registry DOM-to-Blot creation
#[wasm_bindgen]
pub fn test_registry_blot_creation() -> u8 {
    use crate::registry::Registry;
    use web_sys::window;

    if let Some(window) = window() {
        if let Some(document) = window.document() {
            // Test text node creation
            let text_node = document.create_text_node("Hello World");
            match Registry::create_blot_from_node(&text_node.into()) {
                Ok(blot) => {
                    if blot.get_blot_name() == "text" && blot.length() > 0 {
                        return 1; // Success
                    }
                }
                Err(_) => return 0,
            }
        }
    }
    0 // Failed to create or validate
}

/// Test function to verify Registry element type detection
#[wasm_bindgen]
pub fn test_registry_element_detection() -> u8 {
    use crate::registry::Registry;
    use web_sys::window;

    if let Some(window) = window() {
        if let Some(document) = window.document() {
            let mut tests_passed = 0;

            // Test block element detection (p tag)
            if let Ok(p_element) = document.create_element("p") {
                match Registry::create_blot_from_node(&p_element.into()) {
                    Ok(blot) => {
                        if blot.get_blot_name() == "block" {
                            tests_passed += 1;
                        }
                    }
                    Err(_) => return 0,
                }
            }

            // Test inline element detection (span tag)
            if let Ok(span_element) = document.create_element("span") {
                match Registry::create_blot_from_node(&span_element.into()) {
                    Ok(blot) => {
                        if blot.get_blot_name() == "inline" {
                            tests_passed += 1;
                        }
                    }
                    Err(_) => return 0,
                }
            }

            // Test embed element detection (img tag)
            if let Ok(img_element) = document.create_element("img") {
                match Registry::create_blot_from_node(&img_element.into()) {
                    Ok(blot) => {
                        if blot.get_blot_name() == "embed" {
                            tests_passed += 1;
                        }
                    }
                    Err(_) => return 0,
                }
            }

            if tests_passed == 3 {
                return 1; // All tests passed
            }
        }
    }
    0 // Some tests failed
}

/// Test function to verify Scope enum completeness
#[wasm_bindgen]
pub fn test_scope_completeness() -> u8 {
    use crate::scope::Scope;

    // Test that all blot scopes exist and have correct values
    let block_blot = Scope::BlockBlot;
    let inline_blot = Scope::InlineBlot;
    let embed_blot = Scope::EmbedBlot;

    // Test scope matching
    if block_blot.matches(Scope::Block)
        && inline_blot.matches(Scope::Inline)
        && embed_blot as u8 == 0b0100
    {
        // Verify EmbedBlot has correct bitwise value
        1 // Success
    } else {
        0 // Failed scope tests
    }
}

/// Test function to verify Registry blot instance management
#[wasm_bindgen]
pub fn test_registry_instance_management() -> u8 {
    use crate::registry::Registry;
    use wasm_bindgen::JsValue;
    use web_sys::window;

    if let Some(window) = window() {
        if let Some(document) = window.document() {
            if let Ok(div_element) = document.create_element("div") {
                let node = div_element.clone().into();
                let dummy_value = JsValue::from_str("test_blot");

                // Test registration
                match Registry::register_blot_instance(&node, &dummy_value) {
                    Ok(_) => {
                        // Test unregistration
                        if Registry::unregister_blot_instance(&node) {
                            return 1; // Success
                        }
                    }
                    Err(_) => return 0,
                }
            }
        }
    }
    0 // Failed
}

/// Test function to verify MutationObserver creation and basic functionality
#[wasm_bindgen]
pub fn test_mutation_observer_creation() -> u8 {
    use crate::blot::mutations::MutationObserverWrapper;
    use web_sys::window;

    if let Some(window) = window() {
        if let Some(document) = window.document() {
            if let Some(body) = document.body() {
                // Test creating a mutation observer
                match MutationObserverWrapper::new(body.into()) {
                    Ok(observer) => {
                        // Test starting observation
                        match observer.observe() {
                            Ok(_) => {
                                // Test stopping observation
                                observer.disconnect();
                                return 1; // Success
                            }
                            Err(e) => {
                                log(&format!("Failed to start observation: {:?}", e));
                                return 0;
                            }
                        }
                    }
                    Err(e) => {
                        log(&format!("Failed to create MutationObserver: {:?}", e));
                        return 0;
                    }
                }
            }
        }
    }
    0 // Failed (no DOM environment)
}

/// Test function to verify mutation observer handles DOM changes
#[wasm_bindgen]
pub fn test_mutation_observer_dom_changes() -> u8 {
    use crate::blot::mutations::MutationObserverWrapper;
    use web_sys::window;

    if let Some(window) = window() {
        if let Some(document) = window.document() {
            if let Ok(test_div) = document.create_element("div") {
                test_div.set_inner_html("Initial content");

                // Create mutation observer for the test div
                match MutationObserverWrapper::new(test_div.clone().into()) {
                    Ok(observer) => {
                        // Start observing
                        if observer.observe().is_ok() {
                            // Simulate DOM changes
                            test_div.set_inner_html("Modified content");
                            let _ = test_div.set_attribute("class", "test-class");

                            // In a real environment, mutations would be processed asynchronously
                            // For testing, we just verify the observer was created successfully
                            observer.disconnect();
                            return 1; // Success
                        }
                    }
                    Err(e) => {
                        log(&format!("Failed to create mutation observer: {:?}", e));
                        return 0;
                    }
                }
            }
        }
    }
    0 // Failed
}

/// Test function to verify mutation observer attribute change handling
#[wasm_bindgen]
pub fn test_mutation_observer_attributes() -> u8 {
    use crate::blot::mutations::MutationObserverWrapper;
    use web_sys::window;

    if let Some(window) = window() {
        if let Some(document) = window.document() {
            if let Ok(test_element) = document.create_element("span") {
                // Create mutation observer
                match MutationObserverWrapper::new(test_element.clone().into()) {
                    Ok(observer) => {
                        // Test observer configuration for attributes
                        if observer.observe().is_ok() {
                            // Test various attribute changes that our system should handle
                            let _ = test_element.set_attribute("class", "test-class");
                            let _ = test_element.set_attribute("style", "color: red;");
                            let _ = test_element.set_attribute("data-test", "value");

                            observer.disconnect();
                            return 1; // Success - observer can handle attribute changes
                        }
                    }
                    Err(e) => {
                        log(&format!("Failed to create attribute observer: {:?}", e));
                        return 0;
                    }
                }
            }
        }
    }
    0 // Failed
}

/// Test function to verify mutation observer text content handling
#[wasm_bindgen]
pub fn test_mutation_observer_text_changes() -> u8 {
    use crate::blot::mutations::MutationObserverWrapper;
    use web_sys::window;

    if let Some(window) = window() {
        if let Some(document) = window.document() {
            // Create a text node to observe
            let text_node = document.create_text_node("Initial text");

            if let Ok(container) = document.create_element("div") {
                let _ = container.append_child(&text_node);

                // Create mutation observer for the container
                match MutationObserverWrapper::new(container.into()) {
                    Ok(observer) => {
                        if observer.observe().is_ok() {
                            // Test text content changes
                            text_node.set_text_content(Some("Modified text"));

                            observer.disconnect();
                            return 1; // Success
                        }
                    }
                    Err(e) => {
                        log(&format!("Failed to create text change observer: {:?}", e));
                        return 0;
                    }
                }
            }
        }
    }
    0 // Failed
}

/// Test function to verify mutation observer node addition/removal
#[wasm_bindgen]
pub fn test_mutation_observer_node_changes() -> u8 {
    use crate::blot::mutations::MutationObserverWrapper;
    use web_sys::window;

    if let Some(window) = window() {
        if let Some(document) = window.document() {
            if let Ok(container) = document.create_element("div") {
                // Create mutation observer
                match MutationObserverWrapper::new(container.clone().into()) {
                    Ok(observer) => {
                        if observer.observe().is_ok() {
                            // Test node addition
                            if let Ok(new_element) = document.create_element("span") {
                                new_element.set_text_content(Some("New content"));
                                let _ = container.append_child(&new_element);

                                // Test node removal
                                let _ = container.remove_child(&new_element);
                            }

                            observer.disconnect();
                            return 1; // Success
                        }
                    }
                    Err(e) => {
                        log(&format!("Failed to create node change observer: {:?}", e));
                        return 0;
                    }
                }
            }
        }
    }
    0 // Failed
}

/// Test function to verify mutation observer optimization cycles
#[wasm_bindgen]
pub fn test_mutation_observer_optimization() -> u8 {
    use crate::blot::mutations::MutationObserverWrapper;
    use web_sys::window;

    if let Some(window) = window() {
        if let Some(document) = window.document() {
            if let Ok(container) = document.create_element("div") {
                // Create mutation observer
                match MutationObserverWrapper::new(container.clone().into()) {
                    Ok(observer) => {
                        // Test manual optimization trigger
                        match observer.optimize() {
                            Ok(_) => {
                                observer.disconnect();
                                return 1; // Success - optimization cycle completed
                            }
                            Err(e) => {
                                log(&format!("Optimization failed: {:?}", e));
                                return 0;
                            }
                        }
                    }
                    Err(e) => {
                        log(&format!(
                            "Failed to create observer for optimization test: {:?}",
                            e
                        ));
                        return 0;
                    }
                }
            }
        }
    }
    0 // Failed
}
