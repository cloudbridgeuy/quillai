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
//! ```rust
//! use quillai_parchment::{Attributor, AttributorOptions, Scope};
//! 
//! // Create a link href attributor
//! let href_attributor = Attributor::new(
//!     "href",
//!     "href", 
//!     AttributorOptions {
//!         scope: Some(Scope::Inline),
//!         whitelist: None,
//!     }
//! );
//! 
//! // Apply to an element
//! let success = href_attributor.add(&link_element, &JsValue::from_str("https://example.com"));
//! assert!(success);
//! 
//! // Retrieve the value
//! let value = href_attributor.value(&link_element);
//! assert_eq!(value.as_string().unwrap(), "https://example.com");
//! ```

use crate::registry::AttributorTrait;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::Element;

/// Configuration options for creating attributors
///
/// AttributorOptions allows customization of attributor behavior including
/// scope classification and value validation through whitelisting.
///
/// # Examples
///
/// ```rust
/// use quillai_parchment::{AttributorOptions, Scope};
/// 
/// // Basic options with scope
/// let options = AttributorOptions {
///     scope: Some(Scope::Inline),
///     whitelist: None,
/// };
/// 
/// // Options with whitelist for validation
/// let restricted_options = AttributorOptions {
///     scope: Some(Scope::Block),
///     whitelist: Some(vec!["left".to_string(), "center".to_string(), "right".to_string()]),
/// };
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
/// ```rust
/// use quillai_parchment::{Attributor, AttributorOptions, Scope};
/// 
/// // Create image source attributor
/// let src_attributor = Attributor::new(
///     "image",           // Parchment attribute name
///     "src",             // DOM attribute name
///     AttributorOptions {
///         scope: Some(Scope::Inline),
///         whitelist: None,
///     }
/// );
/// 
/// // Use with DOM elements
/// src_attributor.add(&img_element, &JsValue::from_str("image.jpg"));
/// ```
pub struct Attributor {
    /// The logical attribute name used by Parchment
    pub attr_name: String,
    /// The actual DOM attribute name
    pub key_name: String,
    /// The scope classification for this attributor
    pub scope: Scope,
    /// Optional whitelist of allowed values
    pub whitelist: Option<Vec<String>>,
}

impl Attributor {
    /// Create a new base attributor with the specified configuration
    ///
    /// Creates an attributor that manages DOM attributes directly, with optional
    /// scope classification and value validation through whitelisting.
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
    /// // Create a link attributor
    /// let link_attr = Attributor::new(
    ///     "link",
    ///     "href",
    ///     AttributorOptions {
    ///         scope: Some(Scope::Inline),
    ///         whitelist: None,
    ///     }
    /// );
    /// 
    /// // Create a restricted alignment attributor
    /// let align_attr = Attributor::new(
    ///     "align",
    ///     "align",
    ///     AttributorOptions {
    ///         scope: Some(Scope::Block),
    ///         whitelist: Some(vec!["left".to_string(), "center".to_string(), "right".to_string()]),
    ///     }
    /// );
    /// ```
    pub fn new(attr_name: &str, key_name: &str, options: AttributorOptions) -> Self {
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
    /// ```rust
    /// let keys = Attributor::keys(&element);
    /// println!("Element has attributes: {:?}", keys);
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
    /// ```rust
    /// let restricted_attr = Attributor::new(
    ///     "align", "align",
    ///     AttributorOptions {
    ///         whitelist: Some(vec!["left".to_string(), "center".to_string()]),
    ///         ..Default::default()
    ///     }
    /// );
    /// 
    /// assert!(restricted_attr.can_add(&element, &JsValue::from_str("center")));
    /// assert!(!restricted_attr.can_add(&element, &JsValue::from_str("invalid")));
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
