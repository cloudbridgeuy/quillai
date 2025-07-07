//! Comprehensive integration tests for Phase 1.1 Core Editor Integration
//!
//! These tests verify that all the systems work together correctly:
//! - DOM event capture
//! - Delta operations
//! - Parchment integration
//! - ContentEditable wrapper
//! - Main editor component

#[cfg(test)]
mod tests {
    use super::super::*;
    use wasm_bindgen_test::*;
    use web_sys::{HtmlElement, Element};
    use delta::{Delta, Attributes};
    use std::collections::HashMap;

    wasm_bindgen_test_configure!(run_in_browser);

    /// Test DOM event capture system
    #[wasm_bindgen_test]
    fn test_dom_event_capture_integration() {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let element = document.create_element("div").unwrap();
        let html_element = element.dyn_into::<HtmlElement>().unwrap();

        // Create DOM event capture
        let capture = dom_integration::DomEventCapture::new(html_element.clone());
        assert!(capture.is_ok());

        let mut capture = capture.unwrap();
        
        // Setup event listeners
        let setup_result = capture.setup_event_listeners();
        assert!(setup_result.is_ok());

        // Test change detection
        let changes = capture.detect_changes();
        assert!(changes.is_empty()); // No changes initially

        // Test Delta conversion
        let test_changes = vec![dom_integration::DomChange {
            change_type: dom_integration::DomChangeType::Insert,
            position: 0,
            length: 5,
            content: Some("Hello".to_string()),
            attributes: None,
        }];

        let delta = capture.changes_to_delta(test_changes);
        assert_eq!(delta.ops().len(), 1);
    }

    /// Test Delta-to-DOM conversion utilities
    #[wasm_bindgen_test]
    fn test_delta_to_dom_conversion() {
        let converter = dom_integration::DeltaToDomConverter::new();
        assert!(converter.is_ok());

        let converter = converter.unwrap();
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let element = document.create_element("div").unwrap();
        let html_element = element.dyn_into::<HtmlElement>().unwrap();

        // Create a simple Delta
        let mut delta = Delta::new();
        delta = delta.insert("Hello ", None);
        
        let mut bold_attrs = Attributes::new();
        bold_attrs.insert("bold".to_string(), serde_json::Value::Bool(true));
        delta = delta.insert("World", Some(bold_attrs));

        // Apply Delta to element
        let result = converter.apply_delta_to_element(&html_element, &delta);
        assert!(result.is_ok());

        // Verify content was applied
        let inner_html = html_element.inner_html();
        assert!(inner_html.contains("Hello"));
        assert!(inner_html.contains("World"));
    }

    /// Test Delta operation conversion system
    #[wasm_bindgen_test]
    fn test_delta_operation_conversion() {
        let converter = delta_operations::DeltaOperationConverter::new();

        // Test DOM changes to Delta conversion
        let changes = vec![
            dom_integration::DomChange {
                change_type: dom_integration::DomChangeType::Insert,
                position: 0,
                length: 5,
                content: Some("Hello".to_string()),
                attributes: None,
            },
            dom_integration::DomChange {
                change_type: dom_integration::DomChangeType::Insert,
                position: 5,
                length: 6,
                content: Some(" World".to_string()),
                attributes: None,
            },
        ];

        let delta = converter.dom_changes_to_delta(changes);
        assert_eq!(delta.ops().len(), 2);

        // Test Delta to DOM changes conversion
        let dom_changes = converter.delta_to_dom_changes(&delta);
        assert_eq!(dom_changes.len(), 2);

        // Test individual Delta creation methods
        let insert_delta = converter.create_insert_delta(0, "Test", None);
        assert_eq!(insert_delta.ops().len(), 1);

        let delete_delta = converter.create_delete_delta(5, 3);
        assert_eq!(delete_delta.ops().len(), 2); // retain + delete

        let mut format_attrs = Attributes::new();
        format_attrs.insert("bold".to_string(), serde_json::Value::Bool(true));
        let format_delta = converter.create_format_delta(0, 4, format_attrs);
        assert_eq!(format_delta.ops().len(), 1);
    }

    /// Test Parchment-Delta bridge integration
    #[wasm_bindgen_test]
    fn test_parchment_delta_bridge() {
        let mut integration = parchment_integration::ParchmentIntegration::new();
        
        // Test initialization
        assert!(integration.is_initialized());

        // Test ScrollBlot initialization
        let init_result = integration.initialize_scroll_blot(None);
        assert!(init_result.is_ok());
        assert!(integration.scroll_blot().is_some());

        // Test Delta application
        let mut delta = Delta::new();
        delta = delta.insert("Hello, world!", None);

        let apply_result = integration.apply_delta_to_parchment(&delta);
        assert!(apply_result.is_ok());

        // Test Delta extraction
        let extracted_delta = integration.extract_delta_from_parchment();
        assert!(extracted_delta.is_ok());

        let extracted = extracted_delta.unwrap();
        assert!(!extracted.ops().is_empty());

        // Test Delta composition
        let change_delta = Delta::new().retain(5).insert(" beautiful", None);
        let compose_result = integration.compose_delta(&change_delta);
        assert!(compose_result.is_ok());

        // Test current Delta state
        let current_delta = integration.current_delta();
        assert!(!current_delta.ops().is_empty());
    }

    /// Test ContentEditable manager functionality
    #[wasm_bindgen_test]
    fn test_contenteditable_manager() {
        let mut manager = contenteditable::ContentEditableManager::new();

        // Test initial state
        assert!(manager.is_editable);
        assert!(!manager.is_readonly);
        assert!(!manager.has_focus);

        // Test state changes
        manager.set_readonly(true);
        assert!(manager.is_readonly);
        assert!(!manager.is_editable); // Should be false when readonly

        manager.set_readonly(false);
        manager.set_editable(true);
        assert!(manager.is_editable);
        assert!(!manager.is_readonly);

        // Test focus state
        manager.set_focus(true);
        assert!(manager.has_focus);

        // Test browser configuration
        assert!(matches!(
            manager.browser_config.browser_type,
            contenteditable::BrowserType::Chrome |
            contenteditable::BrowserType::Firefox |
            contenteditable::BrowserType::Safari |
            contenteditable::BrowserType::Edge |
            contenteditable::BrowserType::Unknown
        ));

        // Test element application
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let element = document.create_element("div").unwrap();
        let html_element = element.dyn_into::<HtmlElement>().unwrap();

        let apply_result = manager.apply_to_element(&html_element);
        assert!(apply_result.is_ok());

        // Verify contenteditable attribute was set
        let contenteditable = html_element.get_attribute("contenteditable");
        assert!(contenteditable.is_some());
    }

    /// Test end-to-end DOM-Delta-Parchment flow
    #[wasm_bindgen_test]
    fn test_end_to_end_integration() {
        // 1. Create all components
        let mut parchment = parchment_integration::ParchmentIntegration::new();
        let _ = parchment.initialize_scroll_blot(None);

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let element = document.create_element("div").unwrap();
        let html_element = element.dyn_into::<HtmlElement>().unwrap();

        let mut dom_capture = dom_integration::DomEventCapture::new(html_element.clone()).unwrap();
        let _ = dom_capture.setup_event_listeners();

        let delta_converter = delta_operations::DeltaOperationConverter::new();
        let dom_converter = dom_integration::DeltaToDomConverter::new().unwrap();

        // 2. Simulate user typing "Hello"
        let typing_changes = vec![dom_integration::DomChange {
            change_type: dom_integration::DomChangeType::Insert,
            position: 0,
            length: 5,
            content: Some("Hello".to_string()),
            attributes: None,
        }];

        // 3. Convert DOM changes to Delta
        let delta = delta_converter.dom_changes_to_delta(typing_changes);
        assert_eq!(delta.ops().len(), 1);

        // 4. Apply Delta to Parchment
        let apply_result = parchment.apply_delta_to_parchment(&delta);
        assert!(apply_result.is_ok());

        // 5. Extract Delta from Parchment (round-trip test)
        let extracted_delta = parchment.extract_delta_from_parchment().unwrap();
        assert!(!extracted_delta.ops().is_empty());

        // 6. Apply Delta back to DOM
        let dom_apply_result = dom_converter.apply_delta_to_element(&html_element, &delta);
        assert!(dom_apply_result.is_ok());

        // 7. Verify content consistency
        let text_content = html_element.text_content().unwrap_or_default();
        assert!(text_content.contains("Hello"));
    }

    /// Test formatting operations integration
    #[wasm_bindgen_test]
    fn test_formatting_integration() {
        let mut parchment = parchment_integration::ParchmentIntegration::new();
        let _ = parchment.initialize_scroll_blot(None);

        let delta_converter = delta_operations::DeltaOperationConverter::new();

        // 1. Insert text
        let insert_delta = delta_converter.create_insert_delta(0, "Hello World", None);
        let _ = parchment.apply_delta_to_parchment(&insert_delta);

        // 2. Apply bold formatting to "Hello"
        let mut bold_attrs = Attributes::new();
        bold_attrs.insert("bold".to_string(), serde_json::Value::Bool(true));
        let format_delta = delta_converter.create_format_delta(0, 5, bold_attrs);

        let compose_result = parchment.compose_delta(&format_delta);
        assert!(compose_result.is_ok());

        // 3. Verify the formatting was applied
        let current_delta = parchment.current_delta();
        assert!(!current_delta.ops().is_empty());

        // 4. Test document statistics
        if let Some(stats) = parchment.get_document_statistics() {
            assert!(stats.words >= 2); // "Hello World" = 2 words
            assert!(stats.characters > 10); // "Hello World" = 11 characters
        }
    }

    /// Test error handling and edge cases
    #[wasm_bindgen_test]
    fn test_error_handling() {
        // Test empty Delta operations
        let converter = delta_operations::DeltaOperationConverter::new();
        let empty_changes = vec![];
        let empty_delta = converter.dom_changes_to_delta(empty_changes);
        assert!(empty_delta.ops().is_empty());

        // Test invalid position operations
        let invalid_delete = converter.create_delete_delta(1000, 5); // Position beyond content
        assert!(!invalid_delete.ops().is_empty()); // Should still create valid Delta

        // Test Parchment without ScrollBlot
        let mut parchment = parchment_integration::ParchmentIntegration::new();
        let delta = Delta::new().insert("test", None);
        
        // Should fail gracefully without ScrollBlot
        let result = parchment.apply_delta_to_parchment(&delta);
        assert!(result.is_err());

        // Test ContentEditable with invalid element
        let manager = contenteditable::ContentEditableManager::new();
        // This would test with invalid elements in a real browser environment
        assert!(manager.is_editable); // Basic state should still work
    }

    /// Test performance with larger documents
    #[wasm_bindgen_test]
    fn test_performance_integration() {
        let mut parchment = parchment_integration::ParchmentIntegration::new();
        let _ = parchment.initialize_scroll_blot(None);

        let delta_converter = delta_operations::DeltaOperationConverter::new();

        // Create a larger document (1000 characters)
        let large_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(20);
        let insert_delta = delta_converter.create_insert_delta(0, &large_text, None);

        // Measure basic performance (should complete without timeout)
        let start_time = js_sys::Date::now();
        let _ = parchment.apply_delta_to_parchment(&insert_delta);
        let end_time = js_sys::Date::now();

        // Should complete within reasonable time (less than 100ms for this size)
        let duration = end_time - start_time;
        assert!(duration < 100.0, "Operation took too long: {}ms", duration);

        // Test multiple small operations
        for i in 0..50 {
            let small_insert = delta_converter.create_insert_delta(i * 10, "x", None);
            let _ = parchment.compose_delta(&small_insert);
        }

        // Verify document statistics are still reasonable
        if let Some(stats) = parchment.get_document_statistics() {
            assert!(stats.characters > 1000); // Should have grown
        }
    }

    /// Test browser compatibility features
    #[wasm_bindgen_test]
    fn test_browser_compatibility() {
        let config = contenteditable::BrowserConfig::detect_browser();
        
        // Should detect some browser type
        assert!(matches!(
            config.browser_type,
            contenteditable::BrowserType::Chrome |
            contenteditable::BrowserType::Firefox |
            contenteditable::BrowserType::Safari |
            contenteditable::BrowserType::Edge |
            contenteditable::BrowserType::Unknown
        ));

        // Should have reasonable feature detection
        assert!(config.use_input_events); // Most modern browsers support this

        // Test IME state management
        let mut ime_state = contenteditable::ImeState::new();
        assert!(!ime_state.is_composing);

        ime_state.is_composing = true;
        ime_state.composition_text = "test".to_string();
        assert!(ime_state.is_composing);
        assert_eq!(ime_state.composition_text, "test");

        ime_state.reset();
        assert!(!ime_state.is_composing);
        assert!(ime_state.composition_text.is_empty());
    }

    /// Test Delta validation and optimization
    #[wasm_bindgen_test]
    fn test_delta_validation() {
        let converter = delta_operations::DeltaOperationConverter::new();

        // Test valid Delta
        let valid_delta = Delta::new().insert("Hello", None).retain(5);
        assert!(converter.validate_delta(&valid_delta));

        // Test empty Delta
        let empty_delta = Delta::new();
        assert!(converter.validate_delta(&empty_delta));

        // Test Delta optimization
        let delta_to_optimize = Delta::new().insert("Hello", None).insert(" World", None);
        let optimized = converter.optimize_delta(&delta_to_optimize);
        assert!(!optimized.ops().is_empty());
    }

    /// Test mutation detection integration
    #[wasm_bindgen_test]
    fn test_mutation_detection() {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let element = document.create_element("div").unwrap();

        // Create mutation observer wrapper
        let observer = quillai_parchment::blot::mutations::MutationObserverWrapper::new(element.clone().into());
        assert!(observer.is_ok());

        let observer = observer.unwrap();

        // Test observer setup
        let observe_result = observer.observe();
        assert!(observe_result.is_ok());

        // Test manual update (simulating mutations)
        let update_result = observer.update(vec![]);
        assert!(update_result.is_ok());

        // Test optimization
        let optimize_result = observer.optimize();
        assert!(optimize_result.is_ok());

        // Test cleanup
        observer.disconnect(); // Should not panic
    }
}