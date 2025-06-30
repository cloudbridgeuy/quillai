//! Base attributor implementation for direct DOM attribute manipulation
//!
//! The base attributor provides the fundamental implementation for managing
//! DOM attributes directly. It handles setting, getting, and removing attributes
//! on DOM elements with support for whitelisting and scope management.
//!
//! ## Key Features
//!
//! - **Direct Attribute Management**: Sets/gets DOM attributes directly
//! - **Whitelist Support**: Optional value validation against allowed values
//! - **Scope Awareness**: Proper scope classification for attribute types
//! - **Type Safety**: Handles string values with proper validation
//!
//! ## Common Use Cases
//!
//! - Link `href` attributes
//! - Image `src` and `alt` attributes
//! - Form input attributes (`type`, `name`, `value`)
//! - Custom data attributes (`data-*`)
//! - Accessibility attributes (`aria-*`, `role`)
//!
//! ## Examples
//!
//! ```rust,no_run
//! use quillai_parchment::attributor::{Attributor, AttributorOptions};
//! use quillai_parchment::scope::Scope;
//!
//!
//! // Create a link href attributor
//! let href_attributor = Attributor::new_with_scope(
//!     "href".to_string(),
//!     "href".to_string(),
//!     Scope::Inline
//! );
//!
//! // Apply to an element
//! //let success = href_attributor.add(&link_element, &JsValue::from_str("https://example.com"));
//!
//! // Retrieve the value
//! //let value = href_attributor.value(&link_element);
//! ```

use crate::registry::AttributorTrait;
use crate::scope::Scope;
use js_sys::Array;
use wasm_bindgen::prelude::*;
use web_sys::Element;

/// Configuration options for creating attributors
///
/// AttributorOptions allows customization of attributor behavior including
/// scope classification and value validation through whitelisting.
///
/// # Examples
///
/// ```rust,no_run
/// use quillai_parchment::attributor::{Attributor, AttributorOptions};
/// use quillai_parchment::registry::AttributorTrait;
/// use quillai_parchment::Scope;
/// use wasm_bindgen::JsValue;
///
/// // Create a link href attributor
/// let href_attributor = Attributor::new_with_scope(
///     "href".to_string(),
///     "href".to_string(),
///     Scope::Inline
/// );
///
/// // Example usage (requires WASM environment):
/// // let success = href_attributor.add(&link_element, &JsValue::from_str("https://example.com"));
/// // let value = href_attributor.value(&link_element);
/// ```
#[derive(Default, Debug, Clone)]
pub struct AttributorOptions {
    /// Optional scope classification for this attributor
    pub scope: Option<Scope>,
    /// Optional whitelist of allowed values for validation
    pub whitelist: Option<Vec<String>>,
}

/// Base attributor for direct DOM attribute manipulation
///
/// The Attributor struct provides the core implementation for managing DOM
/// attributes directly. It supports value validation, scope management, and
/// provides a clean interface for attribute operations.
///
/// # Characteristics
///
/// - **Direct DOM Access**: Manipulates DOM attributes directly
/// - **Value Validation**: Optional whitelist-based validation
/// - **Scope Aware**: Automatically determines appropriate attribute scope
/// - **String-Based**: Handles string values with proper conversion
///
/// # Attribute vs Key Names
///
/// - `attr_name`: The logical name used by Parchment (e.g., "link")
/// - `key_name`: The actual DOM attribute name (e.g., "href")
///
/// # Examples
///
/// ## Rust Usage
/// ```rust,no_run
/// use quillai_parchment::attributor::{Attributor, AttributorOptions};
/// use quillai_parchment::registry::AttributorTrait;
/// use quillai_parchment::Scope;
/// use wasm_bindgen::JsValue;
///
/// // Create image source attributor
/// let src_attributor = Attributor::new_with_scope(
///     "image".to_string(), // Parchment attribute name
///     "src".to_string(),   // DOM attribute name
///     Scope::Inline        // Inline Scope
/// );
///
/// // Example usage (requires WASM environment):
/// // src_attributor.add(&img_element, &JsValue::from_str("image.jpg"));
/// ```
///
/// ## JavaScript Usage
/// ```javascript
/// import { Attributor, Scope } from './pkg/quillai_parchment.js';
///
/// // Basic attributor
/// const linkAttr = new Attributor("link", "href");
///
/// // With scope
/// const alignAttr = Attributor.newWithScope("align", "text-align", Scope.Block);
///
/// // With whitelist
/// const colorAttr = Attributor.newWithWhitelist("color", "color", ["red", "blue", "green"]);
///
/// // Full configuration
/// const sizeAttr = Attributor.newFull("size", "font-size", Scope.Inline, ["small", "medium", "large"]);
/// ```
#[wasm_bindgen]
pub struct Attributor {
    /// The logical attribute name used by Parchment
    #[wasm_bindgen(skip)]
    pub attr_name: String,
    /// The actual DOM attribute name
    #[wasm_bindgen(skip)]
    pub key_name: String,
    /// The scope classification for this attributor
    #[wasm_bindgen(skip)]
    pub scope: Scope,
    /// Optional whitelist of allowed values
    #[wasm_bindgen(skip)]
    pub whitelist: Option<Vec<String>>,
}

impl Attributor {
    /// Create a new base attributor with the specified configuration
    ///
    /// Creates an attributor that manages DOM attributes directly, with optional
    /// scope classification and value validation through whitelisting.
    ///
    /// This is the Rust-native constructor that takes `AttributorOptions`.
    /// For JavaScript usage, use the WASM constructor methods instead.
    ///
    /// # Parameters
    /// * `attr_name` - Logical attribute name used by Parchment
    /// * `key_name` - Actual DOM attribute name
    /// * `options` - Configuration options including scope and whitelist
    ///
    /// # Returns
    /// New Attributor instance configured with the specified options
    ///
    /// # Scope Handling
    /// The scope is automatically converted to the appropriate attribute scope:
    /// - `Scope::Block` → `Scope::BlockAttribute`
    /// - `Scope::Inline` → `Scope::InlineAttribute`
    /// - Other scopes → `Scope::Attribute`
    ///
    /// # Examples
    /// ```rust
    /// use quillai_parchment::attributor::{Attributor, AttributorOptions};
    /// use quillai_parchment::Scope;
    ///
    /// // Create a link attributor
    /// let link_attr = Attributor::new_with_options(
    ///     "link",
    ///     "href",
    ///     AttributorOptions {
    ///         scope: Some(Scope::Inline),
    ///         whitelist: None,
    ///     }
    /// );
    ///
    /// // Create a restricted alignment attributor
    /// let align_attr = Attributor::new_with_options(
    ///     "align",
    ///     "align",
    ///     AttributorOptions {
    ///         scope: Some(Scope::Block),
    ///         whitelist: Some(vec!["left".to_string(), "center".to_string(), "right".to_string()]),
    ///     }
    /// );
    /// ```
    pub fn new_with_options(attr_name: &str, key_name: &str, options: AttributorOptions) -> Self {
        let scope = if let Some(opt_scope) = options.scope {
            // Convert to appropriate attribute scope while preserving level information
            match opt_scope {
                Scope::Block => Scope::BlockAttribute,
                Scope::Inline => Scope::InlineAttribute,
                _ => Scope::Attribute,
            }
        } else {
            Scope::Attribute
        };

        Self {
            attr_name: attr_name.to_string(),
            key_name: key_name.to_string(),
            scope,
            whitelist: options.whitelist,
        }
    }

    /// Get all attribute names present on a DOM element
    ///
    /// Utility method that extracts all attribute names from a DOM element,
    /// useful for introspection and debugging.
    ///
    /// # Parameters
    /// * `node` - DOM element to inspect
    ///
    /// # Returns
    /// Vector of attribute names present on the element
    ///
    /// # Examples
    /// ```rust,no_run
    /// use quillai_parchment::attributor::Attributor;
    ///
    /// // Example usage (requires WASM environment):
    /// // let keys = Attributor::keys(&element);
    /// // println!("Element has attributes: {:?}", keys);
    /// ```
    pub fn keys(node: &Element) -> Vec<String> {
        let mut keys = Vec::new();
        let attributes = node.attributes();
        for i in 0..attributes.length() {
            if let Some(attr) = attributes.item(i) {
                keys.push(attr.name());
            }
        }
        keys
    }

    /// Check if a value can be added to this attributor
    ///
    /// Validates whether a given value is allowed for this attributor based
    /// on the configured whitelist. If no whitelist is configured, all values
    /// are allowed.
    ///
    /// # Parameters
    /// * `_node` - DOM element (unused in base implementation)
    /// * `value` - Value to validate
    ///
    /// # Returns
    /// `true` if the value is allowed, `false` otherwise
    ///
    /// # Validation Rules
    /// - If no whitelist is configured: all values are allowed
    /// - If whitelist is configured: value must be in the whitelist
    /// - Quotes are stripped from values before validation
    ///
    /// # Examples
    /// ```rust,no_run
    /// use quillai_parchment::attributor::{Attributor, AttributorOptions};
    /// use quillai_parchment::scope::Scope;
    /// use wasm_bindgen::JsValue;
    ///
    /// let restricted_attr = Attributor::new_with_options(
    ///     "align", "align", AttributorOptions {
    ///         scope: Some(Scope::Inline),
    ///         whitelist: Some(vec!["left".to_string(), "center".to_string()]),
    ///     }
    /// );
    ///
    /// // Example usage (requires WASM environment):
    /// // assert!(restricted_attr.can_add(&element, &JsValue::from_str("center")));
    /// // assert!(!restricted_attr.can_add(&element, &JsValue::from_str("invalid")));
    /// ```
    pub fn can_add(&self, _node: &Element, value: &JsValue) -> bool {
        if let Some(ref whitelist) = self.whitelist {
            if let Some(value_str) = value.as_string() {
                // Clean quotes from the value before checking whitelist
                let cleaned = value_str.replace(&['\"', '\''][..], "");
                return whitelist.contains(&cleaned);
            }
            false
        } else {
            true
        }
    }
}

impl AttributorTrait for Attributor {
    fn attr_name(&self) -> &str {
        &self.attr_name
    }

    fn key_name(&self) -> &str {
        &self.key_name
    }

    fn scope(&self) -> Scope {
        self.scope
    }

    fn add(&self, node: &Element, value: &JsValue) -> bool {
        if !self.can_add(node, value) {
            return false;
        }
        if let Some(value_str) = value.as_string() {
            node.set_attribute(&self.key_name, &value_str).is_ok()
        } else {
            false
        }
    }

    fn remove(&self, node: &Element) {
        let _ = node.remove_attribute(&self.key_name);
    }

    fn value(&self, node: &Element) -> JsValue {
        if let Some(value) = node.get_attribute(&self.key_name) {
            if self.can_add(node, &JsValue::from_str(&value)) {
                return JsValue::from_str(&value);
            }
        }
        JsValue::from_str("")
    }
}

/// WebAssembly bindings for Attributor with builder pattern constructors
///
/// This implementation provides JavaScript-friendly constructors and methods
/// while maintaining the full functionality of the Rust implementation.
#[wasm_bindgen]
impl Attributor {
    /// Creates a new Attributor with default options
    ///
    /// This is the basic constructor that creates an attributor with default scope
    /// (Attribute) and no value restrictions.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The actual DOM attribute name
    ///
    /// # Returns
    /// New Attributor instance with default configuration
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor } from './pkg/quillai_parchment.js';
    ///
    /// // Create a basic link attributor
    /// const linkAttr = new Attributor("link", "href");
    ///
    /// // Use with DOM elements
    /// const success = linkAttr.add(linkElement, "https://example.com");
    /// const currentHref = linkAttr.value(linkElement);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(attr_name: String, key_name: String) -> Self {
        Self::new_with_options(&attr_name, &key_name, AttributorOptions::default())
    }

    /// Creates a new Attributor with a specific scope
    ///
    /// This constructor allows you to specify the scope classification for the
    /// attributor, which affects how it interacts with the document model.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The actual DOM attribute name
    /// * `scope` - The scope classification for this attributor
    ///
    /// # Returns
    /// New Attributor instance with the specified scope
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor, Scope } from './pkg/quillai_parchment.js';
    ///
    /// // Create an inline-scoped attributor
    /// const boldAttr = Attributor.newWithScope("bold", "data-bold", Scope.Inline);
    ///
    /// // Create a block-scoped attributor
    /// const alignAttr = Attributor.newWithScope("align", "text-align", Scope.Block);
    /// ```
    #[wasm_bindgen(js_name = "newWithScope")]
    pub fn new_with_scope(attr_name: String, key_name: String, scope: Scope) -> Self {
        Self::new_with_options(
            &attr_name,
            &key_name,
            AttributorOptions {
                scope: Some(scope),
                whitelist: None,
            },
        )
    }

    /// Creates a new Attributor with a whitelist of allowed values
    ///
    /// This constructor creates an attributor that only accepts values from
    /// the provided whitelist. Any attempt to set a value not in the whitelist
    /// will be rejected.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The actual DOM attribute name
    /// * `whitelist` - JavaScript array of allowed string values
    ///
    /// # Returns
    /// New Attributor instance with value validation
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor } from './pkg/quillai_parchment.js';
    ///
    /// // Create a restricted alignment attributor
    /// const alignAttr = Attributor.newWithWhitelist(
    ///     "align",
    ///     "text-align",
    ///     ["left", "center", "right", "justify"]
    /// );
    ///
    /// // This will succeed
    /// alignAttr.add(element, "center");
    ///
    /// // This will fail (returns false)
    /// alignAttr.add(element, "invalid-value");
    /// ```
    #[wasm_bindgen(js_name = "newWithWhitelist")]
    pub fn new_with_whitelist(attr_name: String, key_name: String, whitelist: Array) -> Self {
        let whitelist_vec: Vec<String> = (0..whitelist.length())
            .filter_map(|i| whitelist.get(i).as_string())
            .collect();

        Self::new_with_options(
            &attr_name,
            &key_name,
            AttributorOptions {
                scope: None,
                whitelist: Some(whitelist_vec),
            },
        )
    }

    /// Creates a new Attributor with both scope and whitelist
    ///
    /// This is the most comprehensive constructor that allows you to specify
    /// both the scope classification and a whitelist of allowed values.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The actual DOM attribute name
    /// * `scope` - The scope classification for this attributor
    /// * `whitelist` - JavaScript array of allowed string values
    ///
    /// # Returns
    /// New Attributor instance with full configuration
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor, Scope } from './pkg/quillai_parchment.js';
    ///
    /// // Create a fully configured attributor
    /// const sizeAttr = Attributor.newFull(
    ///     "size",
    ///     "font-size",
    ///     Scope.Inline,
    ///     ["small", "medium", "large", "x-large"]
    /// );
    ///
    /// // Use with validation and proper scope
    /// const success = sizeAttr.add(textElement, "large");
    /// ```
    #[wasm_bindgen(js_name = "newFull")]
    pub fn new_full(attr_name: String, key_name: String, scope: Scope, whitelist: Array) -> Self {
        let whitelist_vec: Vec<String> = (0..whitelist.length())
            .filter_map(|i| whitelist.get(i).as_string())
            .collect();

        Self::new_with_options(
            &attr_name,
            &key_name,
            AttributorOptions {
                scope: Some(scope),
                whitelist: Some(whitelist_vec),
            },
        )
    }

    /// Adds an attribute value to the specified DOM element
    ///
    /// Attempts to set the attribute value on the given DOM element. The operation
    /// will fail if the value is not allowed by the whitelist (if configured) or
    /// if the DOM operation fails.
    ///
    /// # Arguments
    /// * `element` - The DOM element to modify
    /// * `value` - The attribute value to set (will be converted to string)
    ///
    /// # Returns
    /// `true` if the attribute was successfully added, `false` otherwise
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor } from './pkg/quillai_parchment.js';
    ///
    /// const linkAttr = new Attributor("link", "href");
    /// const linkElement = document.createElement('a');
    ///
    /// // Set the href attribute
    /// const success = linkAttr.add(linkElement, "https://example.com");
    /// if (success) {
    ///     console.log("Link href set successfully");
    /// }
    /// ```
    pub fn add(&self, element: &Element, value: &JsValue) -> bool {
        AttributorTrait::add(self, element, value)
    }

    /// Removes the attribute from the specified DOM element
    ///
    /// Removes the attribute managed by this attributor from the given DOM element.
    /// This operation always succeeds, even if the attribute was not present.
    ///
    /// # Arguments
    /// * `element` - The DOM element to modify
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor } from './pkg/quillai_parchment.js';
    ///
    /// const linkAttr = new Attributor("link", "href");
    /// const linkElement = document.querySelector('a');
    ///
    /// // Remove the href attribute
    /// linkAttr.remove(linkElement);
    /// ```
    pub fn remove(&self, element: &Element) {
        AttributorTrait::remove(self, element)
    }

    /// Gets the current attribute value from the specified DOM element
    ///
    /// Retrieves the current value of the attribute managed by this attributor
    /// from the given DOM element. Returns an empty string if the attribute
    /// is not present or if the current value is not allowed by the whitelist.
    ///
    /// # Arguments
    /// * `element` - The DOM element to inspect
    ///
    /// # Returns
    /// The attribute value as a JsValue, or empty string if not present/invalid
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor } from './pkg/quillai_parchment.js';
    ///
    /// const linkAttr = new Attributor("link", "href");
    /// const linkElement = document.querySelector('a');
    ///
    /// // Get the current href value
    /// const currentHref = linkAttr.value(linkElement);
    /// console.log("Current href:", currentHref);
    /// ```
    pub fn value(&self, element: &Element) -> JsValue {
        AttributorTrait::value(self, element)
    }

    /// Gets the logical attribute name used by Parchment
    ///
    /// Returns the logical name that Parchment uses to identify this attributor.
    /// This may be different from the actual DOM attribute name.
    ///
    /// # Returns
    /// The attribute name as a string
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor } from './pkg/quillai_parchment.js';
    ///
    /// const linkAttr = new Attributor("link", "href");
    /// console.log("Attribute name:", linkAttr.attrName()); // "link"
    /// ```
    #[wasm_bindgen(js_name = "attrName")]
    pub fn attr_name(&self) -> String {
        AttributorTrait::attr_name(self).to_string()
    }

    /// Gets the actual DOM attribute name
    ///
    /// Returns the actual DOM attribute name that this attributor manipulates.
    /// This is the name used in DOM operations like `setAttribute` and `getAttribute`.
    ///
    /// # Returns
    /// The key name as a string
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor } from './pkg/quillai_parchment.js';
    ///
    /// const linkAttr = new Attributor("link", "href");
    /// console.log("DOM attribute:", linkAttr.keyName()); // "href"
    /// ```
    #[wasm_bindgen(js_name = "keyName")]
    pub fn key_name(&self) -> String {
        AttributorTrait::key_name(self).to_string()
    }

    /// Gets the scope classification for this attributor
    ///
    /// Returns the scope that determines how this attributor interacts with
    /// the document model and other blots.
    ///
    /// # Returns
    /// The Scope enum value
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor, Scope } from './pkg/quillai_parchment.js';
    ///
    /// const linkAttr = Attributor.newWithScope("link", "href", Scope.Inline);
    /// const scope = linkAttr.scope();
    ///
    /// if (scope === Scope.InlineAttribute) {
    ///     console.log("This is an inline attribute");
    /// }
    /// ```
    pub fn scope(&self) -> Scope {
        AttributorTrait::scope(self)
    }

    /// Gets all attribute names present on a DOM element
    ///
    /// Utility method that extracts all attribute names from a DOM element.
    /// This is useful for introspection and debugging purposes.
    ///
    /// # Arguments
    /// * `element` - The DOM element to inspect
    ///
    /// # Returns
    /// JavaScript array of attribute names present on the element
    ///
    /// # Examples
    /// ```javascript
    /// import { Attributor } from './pkg/quillai_parchment.js';
    ///
    /// const element = document.querySelector('a');
    /// const attributes = Attributor.keys(element);
    ///
    /// console.log("Element attributes:");
    /// attributes.forEach(attr => console.log(`  ${attr}`));
    /// ```
    #[wasm_bindgen(js_name = "keys")]
    pub fn keys_js(element: &Element) -> Array {
        let keys = Self::keys(element);
        let array = Array::new();
        for key in keys {
            array.push(&JsValue::from_str(&key));
        }
        array
    }
}
