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
//! ```rust
//! use quillai_parchment::{StyleAttributor, AttributorOptions, Scope};
//! 
//! // Create text color attributor
//! let color_attributor = StyleAttributor::new(
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
//! let size_attributor = StyleAttributor::new("fontSize", "font-size", options);
//! size_attributor.add(&element, &JsValue::from_str("16px"));
//! // Results in: style="color: #ff0000; font-size: 16px"
//! ```

use crate::attributor::base::AttributorOptions;
use crate::registry::AttributorTrait;
use crate::scope::Scope;
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
/// ```rust
/// use quillai_parchment::{StyleAttributor, AttributorOptions, Scope};
/// 
/// // Create background color attributor
/// let bg_attributor = StyleAttributor::new(
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
/// ```
pub struct StyleAttributor {
    /// The logical attribute name used by Parchment
    pub attr_name: String,
    /// The CSS property name to manipulate
    pub key_name: String,
    /// The scope classification for this attributor
    pub scope: Scope,
    /// Optional whitelist of allowed values
    pub whitelist: Option<Vec<String>>,
}

impl StyleAttributor {
    /// Create a new inline style attributor
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
    /// ```rust
    /// // Create text color attributor
    /// let color_attr = StyleAttributor::new(
    ///     "textColor",
    ///     "color",
    ///     AttributorOptions {
    ///         scope: Some(Scope::Inline),
    ///         whitelist: None,
    ///     }
    /// );
    /// 
    /// // Create font size with restricted values
    /// let size_attr = StyleAttributor::new(
    ///     "fontSize",
    ///     "font-size",
    ///     AttributorOptions {
    ///         scope: Some(Scope::Inline),
    ///         whitelist: Some(vec!["12px".to_string(), "14px".to_string(), "16px".to_string()]),
    ///     }
    /// );
    /// ```
    pub fn new(attr_name: &str, key_name: &str, options: AttributorOptions) -> Self {
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
    /// ```rust
    /// let restricted_color = StyleAttributor::new(
    ///     "color", "color",
    ///     AttributorOptions {
    ///         whitelist: Some(vec!["#ff0000".to_string(), "#00ff00".to_string(), "#0000ff".to_string()]),
    ///         ..Default::default()
    ///     }
    /// );
    /// 
    /// assert!(restricted_color.can_add(&element, &JsValue::from_str("#ff0000")));
    /// assert!(!restricted_color.can_add(&element, &JsValue::from_str("#purple")));
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
                if !value.is_empty() && self.can_add(node, &JsValue::from_str(&value)) {
                    return JsValue::from_str(&value);
                }
            }
        }
        JsValue::from_str("")
    }
}
