//! Inline style attributor for direct CSS property management
//!
//! StyleAttributor provides direct manipulation of CSS style properties through
//! the element's inline style attribute. This approach offers immediate styling
//! control and is ideal for dynamic, user-controlled formatting options.
//!
//! ## Style Property Management
//!
//! StyleAttributor directly sets CSS properties on the element's style attribute:
//! ```html
//! <span style="color: #ff0000; font-size: 14px;">Styled text</span>
//! ```
//!
//! ## Advantages
//!
//! - **Immediate Effect**: Styles are applied directly without CSS dependencies
//! - **High Specificity**: Inline styles override most CSS rules
//! - **Dynamic Values**: Perfect for user-controlled formatting (color pickers, etc.)
//! - **Precise Control**: Exact values can be specified (e.g., specific colors, sizes)
//! - **No CSS Dependencies**: Works without external stylesheets
//!
//! ## Common Use Cases
//!
//! - **Text Color**: User-selected colors from color pickers
//! - **Font Size**: Precise font size values
//! - **Background Color**: Highlighting and background colors
//! - **Margins/Padding**: Exact spacing values
//! - **Custom Properties**: CSS custom properties (CSS variables)
//!
//! ## Performance Considerations
//!
//! - Inline styles have higher specificity than CSS classes
//! - Large numbers of inline styles can impact performance
//! - Consider using ClassAttributor for predefined style sets
//! - Best for dynamic, user-controlled values
//!
//! ## Examples
//!
//! ```rust,no_run
//! # fn main() -> Result<(), wasm_bindgen::JsValue> {
//! use quillai_parchment::attributor::{StyleAttributor, AttributorOptions};
//! use quillai_parchment::scope::Scope;
//! use quillai_parchment::registry::AttributorTrait;
//! use quillai_parchment::dom::Dom;
//! use wasm_bindgen::JsValue;
//!
//! let element = Dom::create_element("p")?;
//!
//! let options = AttributorOptions {
//!    scope: Some(Scope::Block),
//!    whitelist: None,
//! };
//!
//! // Create text color attributor
//! let color_attributor = StyleAttributor::new_with_options(
//!     "color",           // Parchment attribute name
//!     "color",           // CSS property name
//!     AttributorOptions {
//!         scope: Some(Scope::Inline),
//!         whitelist: None, // Allow any color value
//!     }
//! );
//!
//! // Apply red color
//! color_attributor.add(&element, &JsValue::from_str("#ff0000"));
//! // Results in: style="color: #ff0000"
//!
//! // Apply font size
//! let size_attributor = StyleAttributor::new_with_options("fontSize", "font-size", options);
//! size_attributor.add(&element, &JsValue::from_str("16px"));
//! // Results in: style="color: #ff0000; font-size: 16px"
//! # Ok(())
//! # }
//! ```

use crate::attributor::base::AttributorOptions;
use crate::registry::AttributorTrait;
use crate::scope::Scope;
use js_sys::Array;
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement};

/// Inline style attributor for direct CSS property manipulation
///
/// StyleAttributor manages CSS properties through the element's inline style
/// attribute, providing direct control over styling with immediate effect.
/// It's ideal for dynamic, user-controlled formatting options.
///
/// # Characteristics
///
/// - **Direct CSS Control**: Sets CSS properties directly on elements
/// - **High Specificity**: Inline styles override most CSS rules
/// - **Dynamic Values**: Perfect for runtime-determined styles
/// - **Immediate Effect**: No dependency on external CSS files
///
/// # CSS Property Mapping
///
/// The key_name corresponds directly to CSS property names:
/// - `color` → `color` CSS property
/// - `font-size` → `font-size` CSS property
/// - `background-color` → `background-color` CSS property
///
/// # Examples
///
/// ```rust,no_run
/// # fn main() -> Result<(), wasm_bindgen::JsValue> {
/// use quillai_parchment::attributor::{StyleAttributor, AttributorOptions};
/// use quillai_parchment::scope::Scope;
/// use quillai_parchment::registry::AttributorTrait;
/// use quillai_parchment::dom::Dom;
/// use wasm_bindgen::JsValue;
///
/// let span_element = Dom::create_element("span")?;
///
/// // Create background color attributor
/// let bg_attributor = StyleAttributor::new_with_options(
///     "background",
///     "background-color",
///     AttributorOptions {
///         scope: Some(Scope::Inline),
///         whitelist: None,
///     }
/// );
///
/// // Apply yellow background
/// bg_attributor.add(&span_element, &JsValue::from_str("#ffff00"));
/// // Results in: style="background-color: #ffff00"
/// # Ok(())
/// # }
/// ```
#[wasm_bindgen]
pub struct StyleAttributor {
    /// The logical attribute name used by Parchment
    #[wasm_bindgen(skip)]
    pub attr_name: String,
    /// The CSS property name to manipulate
    #[wasm_bindgen(skip)]
    pub key_name: String,
    /// The scope classification for this attributor
    #[wasm_bindgen(skip)]
    pub scope: Scope,
    /// Optional whitelist of allowed values
    #[wasm_bindgen(skip)]
    pub whitelist: Option<Vec<String>>,
}

impl StyleAttributor {
    /// Create a new inline style attributor with options
    ///
    /// Creates an attributor that manages CSS properties through the element's
    /// inline style attribute. The key_name should correspond to a valid CSS
    /// property name.
    ///
    /// # Parameters
    /// * `attr_name` - Logical attribute name used by Parchment
    /// * `key_name` - CSS property name to manipulate
    /// * `options` - Configuration options including scope and whitelist
    ///
    /// # Returns
    /// New StyleAttributor instance configured with the specified options
    ///
    /// # CSS Property Names
    /// Use standard CSS property names for key_name:
    /// - `color`, `background-color`, `border-color`
    /// - `font-size`, `font-weight`, `font-family`
    /// - `margin`, `padding`, `border-width`
    /// - `text-align`, `text-decoration`
    ///
    /// # Examples
    /// ```rust,no_run
    /// use quillai_parchment::attributor::{StyleAttributor, AttributorOptions};
    /// use quillai_parchment::scope::Scope;
    ///
    /// // Create text color attributor
    /// let color_attr = StyleAttributor::new_with_options(
    ///     "textColor",
    ///     "color",
    ///     AttributorOptions {
    ///         scope: Some(Scope::Inline),
    ///         whitelist: None,
    ///     }
    /// );
    ///
    /// // Create font size with restricted values
    /// let size_attr = StyleAttributor::new_with_options(
    ///     "fontSize",
    ///     "font-size",
    ///     AttributorOptions {
    ///         scope: Some(Scope::Inline),
    ///         whitelist: Some(vec!["12px".to_string(), "14px".to_string(), "16px".to_string()]),
    ///     }
    /// );
    /// ```
    pub fn new_with_options(attr_name: &str, key_name: &str, options: AttributorOptions) -> Self {
        let scope = if let Some(opt_scope) = options.scope {
            // Convert to appropriate attribute scope
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

    /// Check if a value can be added to this style attributor
    ///
    /// Validates whether a given CSS value is allowed for this attributor based
    /// on the configured whitelist. If no whitelist is configured, all values
    /// are allowed.
    ///
    /// # Parameters
    /// * `_node` - DOM element (unused in style implementation)
    /// * `value` - CSS value to validate
    ///
    /// # Returns
    /// `true` if the value is allowed, `false` otherwise
    ///
    /// # Validation
    /// - If no whitelist: all values are allowed
    /// - If whitelist exists: value must be in the whitelist
    /// - Useful for restricting to specific color palettes, font sizes, etc.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # fn main() -> Result<(), wasm_bindgen::JsValue> {
    /// use quillai_parchment::attributor::{StyleAttributor, AttributorOptions};
    /// use quillai_parchment::dom::Dom;
    /// use wasm_bindgen::JsValue;
    ///
    /// let element = Dom::create_element("p")?;
    ///
    /// let restricted_color = StyleAttributor::new_with_options(
    ///     "color", "color",
    ///     AttributorOptions {
    ///         whitelist: Some(vec!["#ff0000".to_string(), "#00ff00".to_string(), "#0000ff".to_string()]),
    ///         ..Default::default()
    ///     }
    /// );
    ///
    /// assert!(restricted_color.can_add(&element, &JsValue::from_str("#ff0000")));
    /// assert!(!restricted_color.can_add(&element, &JsValue::from_str("#purple")));
    /// # Ok(())
    /// # }
    /// ```
    pub fn can_add(&self, _node: &Element, value: &JsValue) -> bool {
        if let Some(ref whitelist) = self.whitelist {
            if let Some(value_str) = value.as_string() {
                return whitelist.contains(&value_str);
            }
            false
        } else {
            true
        }
    }
}

impl AttributorTrait for StyleAttributor {
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

        if let Some(html_element) = node.dyn_ref::<HtmlElement>() {
            if let Some(value_str) = value.as_string() {
                let style = html_element.style();
                style.set_property(&self.key_name, &value_str).is_ok()
            } else {
                false
            }
        } else {
            false
        }
    }

    fn remove(&self, node: &Element) {
        if let Some(html_element) = node.dyn_ref::<HtmlElement>() {
            let style = html_element.style();
            let _ = style.remove_property(&self.key_name);
        }
    }

    fn value(&self, node: &Element) -> JsValue {
        if let Some(html_element) = node.dyn_ref::<HtmlElement>() {
            let style = html_element.style();
            if let Ok(value) = style.get_property_value(&self.key_name) {
                return JsValue::from_str(&value);
            }
        }
        JsValue::from_str("")
    }
}

/// WebAssembly bindings for StyleAttributor with builder pattern constructors
///
/// This implementation provides JavaScript-friendly constructors and methods
/// for CSS style property manipulation while maintaining the full functionality
/// of the Rust implementation.
#[wasm_bindgen]
impl StyleAttributor {
    /// Creates a new StyleAttributor with default options
    ///
    /// This is the basic constructor that creates a style attributor with default scope
    /// (Attribute) and no value restrictions. Perfect for basic CSS property manipulation.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The CSS property name to manipulate (e.g., "color", "font-size")
    ///
    /// # Returns
    /// New StyleAttributor instance with default configuration
    ///
    /// # Examples
    /// ```javascript
    /// import { StyleAttributor } from './pkg/quillai_parchment.js';
    ///
    /// // Create a basic color attributor
    /// const colorAttr = new StyleAttributor("color", "color");
    ///
    /// // Use with DOM elements
    /// const element = document.createElement('span');
    /// const success = colorAttr.add(element, "#ff0000");
    /// const currentColor = colorAttr.value(element);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(attr_name: String, key_name: String) -> Self {
        Self::new_with_options(&attr_name, &key_name, AttributorOptions::default())
    }

    /// Creates a new StyleAttributor with a specific scope
    ///
    /// This constructor allows you to specify the scope classification for the
    /// style attributor, which affects how it interacts with the document model.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The CSS property name to manipulate
    /// * `scope` - The scope classification for this attributor
    ///
    /// # Returns
    /// New StyleAttributor instance with the specified scope
    ///
    /// # Examples
    /// ```javascript
    /// import { StyleAttributor, Scope } from './pkg/quillai_parchment.js';
    ///
    /// // Create an inline-scoped color attributor
    /// const colorAttr = StyleAttributor.newWithScope("color", "color", Scope.Inline);
    ///
    /// // Create a block-scoped text alignment attributor
    /// const alignAttr = StyleAttributor.newWithScope("align", "text-align", Scope.Block);
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

    /// Creates a new StyleAttributor with a whitelist of allowed CSS values
    ///
    /// This constructor creates a style attributor that only accepts CSS values from
    /// the provided whitelist. Any attempt to set a value not in the whitelist
    /// will be rejected. Perfect for restricting to specific color palettes, font sizes, etc.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The CSS property name to manipulate
    /// * `whitelist` - JavaScript array of allowed CSS values
    ///
    /// # Returns
    /// New StyleAttributor instance with value validation
    ///
    /// # Examples
    /// ```javascript
    /// import { StyleAttributor } from './pkg/quillai_parchment.js';
    ///
    /// // Create a restricted font size attributor
    /// const sizeAttr = StyleAttributor.newWithWhitelist(
    ///     "fontSize",
    ///     "font-size",
    ///     ["12px", "14px", "16px", "18px", "24px"]
    /// );
    ///
    /// // This will succeed
    /// sizeAttr.add(element, "16px");
    ///
    /// // This will fail (returns false)
    /// sizeAttr.add(element, "13px");
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

    /// Creates a new StyleAttributor with both scope and whitelist
    ///
    /// This is the most comprehensive constructor that allows you to specify
    /// both the scope classification and a whitelist of allowed CSS values.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The CSS property name to manipulate
    /// * `scope` - The scope classification for this attributor
    /// * `whitelist` - JavaScript array of allowed CSS values
    ///
    /// # Returns
    /// New StyleAttributor instance with full configuration
    ///
    /// # Examples
    /// ```javascript
    /// import { StyleAttributor, Scope } from './pkg/quillai_parchment.js';
    ///
    /// // Create a fully configured background color attributor
    /// const bgAttr = StyleAttributor.newFull(
    ///     "background",
    ///     "background-color",
    ///     Scope.Inline,
    ///     ["#ffffff", "#f0f0f0", "#e0e0e0", "#d0d0d0"]
    /// );
    ///
    /// // Use with validation and proper scope
    /// const success = bgAttr.add(textElement, "#ffffff");
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

    /// Adds a CSS style property value to the specified DOM element
    ///
    /// Attempts to set the CSS property value on the given DOM element's inline style.
    /// The operation will fail if the value is not allowed by the whitelist (if configured),
    /// if the element is not an HtmlElement, or if the CSS operation fails.
    ///
    /// # Arguments
    /// * `element` - The DOM element to modify (must be an HtmlElement)
    /// * `value` - The CSS value to set (will be converted to string)
    ///
    /// # Returns
    /// `true` if the style property was successfully added, `false` otherwise
    ///
    /// # Examples
    /// ```javascript
    /// import { StyleAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const colorAttr = new StyleAttributor("color", "color");
    /// const element = document.createElement('span');
    ///
    /// // Set the color style property
    /// const success = colorAttr.add(element, "#ff0000");
    /// if (success) {
    ///     console.log("Color set successfully");
    ///     // element.style.color is now "#ff0000"
    /// }
    /// ```
    pub fn add(&self, element: &Element, value: &JsValue) -> bool {
        // Check for null element to prevent JavaScript null pointer errors
        if element.is_null() {
            return false;
        }
        AttributorTrait::add(self, element, value)
    }

    /// Removes the CSS style property from the specified DOM element
    ///
    /// Removes the CSS property managed by this attributor from the given DOM element's
    /// inline style. This operation always succeeds, even if the property was not present.
    /// Only the specific CSS property is removed, not the entire style attribute.
    ///
    /// # Arguments
    /// * `element` - The DOM element to modify
    ///
    /// # Examples
    /// ```javascript
    /// import { StyleAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const colorAttr = new StyleAttributor("color", "color");
    /// const element = document.querySelector('span');
    ///
    /// // Remove the color style property
    /// colorAttr.remove(element);
    /// // element.style.color is now empty, but other styles remain
    /// ```
    pub fn remove(&self, element: &Element) {
        // Check for null element to prevent JavaScript null pointer errors
        if element.is_null() {
            return;
        }
        AttributorTrait::remove(self, element)
    }

    /// Gets the current CSS style property value from the specified DOM element
    ///
    /// Retrieves the current value of the CSS property managed by this attributor
    /// from the given DOM element's inline style. Returns an empty string if the property
    /// is not present or if the current value is not allowed by the whitelist.
    ///
    /// # Arguments
    /// * `element` - The DOM element to inspect
    ///
    /// # Returns
    /// The CSS property value as a JsValue, or empty string if not present/invalid
    ///
    /// # Examples
    /// ```javascript
    /// import { StyleAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const colorAttr = new StyleAttributor("color", "color");
    /// const element = document.querySelector('span');
    ///
    /// // Get the current color value
    /// const currentColor = colorAttr.value(element);
    /// console.log("Current color:", currentColor);
    /// ```
    pub fn value(&self, element: &Element) -> JsValue {
        // Check for null element to prevent JavaScript null pointer errors
        if element.is_null() {
            return JsValue::from_str("");
        }
        AttributorTrait::value(self, element)
    }

    /// Gets the logical attribute name used by Parchment
    ///
    /// Returns the logical name that Parchment uses to identify this style attributor.
    /// This may be different from the actual CSS property name.
    ///
    /// # Returns
    /// The attribute name as a string
    ///
    /// # Examples
    /// ```javascript
    /// import { StyleAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const colorAttr = new StyleAttributor("textColor", "color");
    /// console.log("Attribute name:", colorAttr.attrName()); // "textColor"
    /// ```
    #[wasm_bindgen(js_name = "attrName")]
    pub fn attr_name(&self) -> String {
        AttributorTrait::attr_name(self).to_string()
    }

    /// Gets the actual CSS property name
    ///
    /// Returns the actual CSS property name that this attributor manipulates.
    /// This is the name used in CSS operations like `style.setProperty` and `style.getPropertyValue`.
    ///
    /// # Returns
    /// The CSS property name as a string
    ///
    /// # Examples
    /// ```javascript
    /// import { StyleAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const colorAttr = new StyleAttributor("textColor", "color");
    /// console.log("CSS property:", colorAttr.keyName()); // "color"
    /// ```
    #[wasm_bindgen(js_name = "keyName")]
    pub fn key_name(&self) -> String {
        AttributorTrait::key_name(self).to_string()
    }

    /// Gets the scope classification for this style attributor
    ///
    /// Returns the scope that determines how this attributor interacts with
    /// the document model and other blots.
    ///
    /// # Returns
    /// The Scope enum value
    ///
    /// # Examples
    /// ```javascript
    /// import { StyleAttributor, Scope } from './pkg/quillai_parchment.js';
    ///
    /// const colorAttr = StyleAttributor.newWithScope("color", "color", Scope.Inline);
    /// const scope = colorAttr.scope();
    ///
    /// if (scope === Scope.InlineAttribute) {
    ///     console.log("This is an inline style attribute");
    /// }
    /// ```
    pub fn scope(&self) -> Scope {
        AttributorTrait::scope(self)
    }
}
