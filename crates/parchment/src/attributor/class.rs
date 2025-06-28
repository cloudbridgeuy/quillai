//! CSS class-based attributor for class-driven formatting
//!
//! ClassAttributor provides a CSS class-based approach to formatting by managing
//! CSS classes with a prefix pattern. This allows for clean separation of styling
//! from content and enables CSS-based theming and responsive design.
//!
//! ## Class Naming Pattern
//!
//! ClassAttributor uses a prefix-value pattern for CSS class names:
//! ```text
//! {key_name}-{value}
//! ```
//!
//! For example:
//! - Text alignment: `text-align-center`, `text-align-left`, `text-align-right`
//! - Font size: `font-size-small`, `font-size-medium`, `font-size-large`
//! - Color themes: `color-primary`, `color-secondary`, `color-accent`
//!
//! ## Advantages
//!
//! - **CSS Separation**: Keeps styling logic in CSS files
//! - **Theming Support**: Easy to implement themes and design systems
//! - **Performance**: CSS classes are more performant than inline styles
//! - **Responsive Design**: Supports media queries and responsive breakpoints
//! - **Maintainability**: Centralized styling in CSS files
//!
//! ## Common Use Cases
//!
//! - Text alignment (`text-align-left`, `text-align-center`)
//! - Typography scales (`font-size-sm`, `font-size-lg`)
//! - Color schemes (`text-primary`, `bg-secondary`)
//! - Spacing utilities (`margin-sm`, `padding-lg`)
//! - Layout classes (`display-flex`, `position-relative`)
//!
//! ## Examples
//!
//! ```rust
//! use quillai_parchment::{ClassAttributor, AttributorOptions, Scope};
//! 
//! // Create text alignment attributor
//! let align_attributor = ClassAttributor::new(
//!     "align",           // Parchment attribute name
//!     "text-align",      // CSS class prefix
//!     AttributorOptions {
//!         scope: Some(Scope::Block),
//!         whitelist: Some(vec![
//!             "left".to_string(),
//!             "center".to_string(), 
//!             "right".to_string(),
//!             "justify".to_string()
//!         ]),
//!     }
//! );
//! 
//! // Apply center alignment
//! align_attributor.add(&element, &JsValue::from_str("center"));
//! // Results in: class="text-align-center"
//! 
//! // Remove alignment
//! align_attributor.remove(&element);
//! // Removes all "text-align-*" classes
//! ```

use crate::attributor::base::AttributorOptions;
use crate::registry::AttributorTrait;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::Element;

/// CSS class-based attributor for prefix-pattern class management
///
/// ClassAttributor manages CSS classes using a prefix-value naming pattern,
/// enabling clean CSS-based styling and theming. It automatically handles
/// class addition, removal, and validation based on configured options.
///
/// # Class Pattern
///
/// Classes are generated using the pattern: `{key_name}-{value}`
///
/// # Characteristics
///
/// - **Prefix-Based**: Uses consistent naming patterns for CSS classes
/// - **Whitelist Support**: Optional validation of allowed values
/// - **Clean Removal**: Removes all classes matching the prefix pattern
/// - **CSS Integration**: Designed to work with CSS frameworks and design systems
///
/// # Examples
///
/// ```rust
/// use quillai_parchment::{ClassAttributor, AttributorOptions, Scope};
/// 
/// // Create a font size attributor
/// let size_attributor = ClassAttributor::new(
///     "size",
///     "font-size",
///     AttributorOptions {
///         scope: Some(Scope::Inline),
///         whitelist: Some(vec!["sm".to_string(), "md".to_string(), "lg".to_string()]),
///     }
/// );
/// 
/// // Apply large font size
/// size_attributor.add(&span_element, &JsValue::from_str("lg"));
/// // Results in: class="font-size-lg"
/// ```
pub struct ClassAttributor {
    /// The logical attribute name used by Parchment
    pub attr_name: String,
    /// The CSS class prefix used for generating class names
    pub key_name: String,
    /// The scope classification for this attributor
    pub scope: Scope,
    /// Optional whitelist of allowed values
    pub whitelist: Option<Vec<String>>,
}

impl ClassAttributor {
    /// Create a new CSS class-based attributor
    ///
    /// Creates an attributor that manages CSS classes using a prefix-value pattern.
    /// The key_name serves as the prefix for all generated CSS class names.
    ///
    /// # Parameters
    /// * `attr_name` - Logical attribute name used by Parchment
    /// * `key_name` - CSS class prefix for generating class names
    /// * `options` - Configuration options including scope and whitelist
    ///
    /// # Returns
    /// New ClassAttributor instance configured with the specified options
    ///
    /// # Class Generation
    /// CSS classes are generated as: `{key_name}-{value}`
    ///
    /// # Examples
    /// ```rust
    /// // Create text alignment attributor
    /// let align_attr = ClassAttributor::new(
    ///     "align",
    ///     "text-align",
    ///     AttributorOptions {
    ///         scope: Some(Scope::Block),
    ///         whitelist: Some(vec!["left".to_string(), "center".to_string(), "right".to_string()]),
    ///     }
    /// );
    /// 
    /// // Will generate classes like: "text-align-left", "text-align-center"
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

    /// Generate CSS class name from value using prefix pattern
    ///
    /// Creates a CSS class name by combining the key_name prefix with the
    /// provided value using a hyphen separator.
    ///
    /// # Parameters
    /// * `value` - The value to append to the prefix
    ///
    /// # Returns
    /// Generated CSS class name in the format `{key_name}-{value}`
    ///
    /// # Examples
    /// ```rust
    /// let attributor = ClassAttributor::new("align", "text-align", options);
    /// assert_eq!(attributor.get_class_name("center"), "text-align-center");
    /// ```
    fn get_class_name(&self, value: &str) -> String {
        format!("{}-{}", self.key_name, value)
    }

    /// Check if a value can be added to this class attributor
    ///
    /// Validates whether a given value is allowed for this attributor based
    /// on the configured whitelist. If no whitelist is configured, all values
    /// are allowed.
    ///
    /// # Parameters
    /// * `_node` - DOM element (unused in class implementation)
    /// * `value` - Value to validate
    ///
    /// # Returns
    /// `true` if the value is allowed, `false` otherwise
    ///
    /// # Examples
    /// ```rust
    /// let restricted_attr = ClassAttributor::new(
    ///     "size", "font-size",
    ///     AttributorOptions {
    ///         whitelist: Some(vec!["sm".to_string(), "lg".to_string()]),
    ///         ..Default::default()
    ///     }
    /// );
    /// 
    /// assert!(restricted_attr.can_add(&element, &JsValue::from_str("sm")));
    /// assert!(!restricted_attr.can_add(&element, &JsValue::from_str("invalid")));
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

impl AttributorTrait for ClassAttributor {
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
            let class_name = self.get_class_name(&value_str);
            let class_list = node.class_list();
            class_list.add_1(&class_name).is_ok()
        } else {
            false
        }
    }

    fn remove(&self, node: &Element) {
        let class_list = node.class_list();
        // Remove all classes that start with our key_name prefix
        let classes_to_remove: Vec<String> = (0..class_list.length())
            .filter_map(|i| class_list.item(i))
            .filter(|class: &String| class.starts_with(&format!("{}-", self.key_name)))
            .collect();

        for class in classes_to_remove {
            let _ = class_list.remove_1(&class);
        }
    }

    fn value(&self, node: &Element) -> JsValue {
        let class_list = node.class_list();
        let prefix = format!("{}-", self.key_name);
        for i in 0..class_list.length() {
            if let Some(class) = class_list.item(i) {
                if let Some(value) = class.strip_prefix(&prefix) {
                    if self.can_add(node, &JsValue::from_str(value)) {
                        return JsValue::from_str(value);
                    }
                }
            }
        }
        JsValue::from_str("")
    }
}
