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
//! ```rust,no_run
//! # fn main() -> Result<(), wasm_bindgen::JsValue> {
//! use quillai_parchment::attributor::{ClassAttributor, AttributorOptions};
//! use quillai_parchment::scope::Scope;
//! use quillai_parchment::dom::Dom;
//! use quillai_parchment::registry::AttributorTrait;
//! use wasm_bindgen::JsValue;
//!
//! // Create a new block element
//! let element = Dom::create_element("p")?;
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
//! # Ok(())
//! # }
//! ```

use crate::attributor::base::AttributorOptions;
use crate::registry::AttributorTrait;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::Element;
use js_sys::Array;

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
/// ```rust,no_run
/// # fn main() -> Result<(), wasm_bindgen::JsValue> {
/// use quillai_parchment::attributor::{ClassAttributor, AttributorOptions};
/// use quillai_parchment::registry::AttributorTrait;
/// use quillai_parchment::scope::Scope;
/// use quillai_parchment::dom::Dom;
/// use wasm_bindgen::JsValue;
///
/// let span_element = Dom::create_element("span")?;
///
/// // Create a font size attributor
/// let size_attributor = ClassAttributor::new_with_options(
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
/// # Ok(())
/// # }
/// ```
#[wasm_bindgen]
pub struct ClassAttributor {
    /// The logical attribute name used by Parchment
    #[wasm_bindgen(skip)]
    pub attr_name: String,
    /// The CSS class prefix used for generating class names
    #[wasm_bindgen(skip)]
    pub key_name: String,
    /// The scope classification for this attributor
    #[wasm_bindgen(skip)]
    pub scope: Scope,
    /// Optional whitelist of allowed values
    #[wasm_bindgen(skip)]
    pub whitelist: Option<Vec<String>>,
}

impl ClassAttributor {
    /// Create a new CSS class-based attributor with options
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
    /// ```rust,no_run
    /// use quillai_parchment::attributor::{ClassAttributor, AttributorOptions};
    /// use quillai_parchment::scope::Scope;
    ///
    /// // Create text alignment attributor
    /// let align_attr = ClassAttributor::new_with_options(
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
    /// ```rust,no_run
    /// use quillai_parchment::attributor::{ClassAttributor, AttributorOptions};
    /// use quillai_parchment::scope::Scope;
    /// use quillai_parchment::dom::Dom;
    ///
    /// // Create a new block element
    /// let element = Dom::create_element("p");
    /// let options = AttributorOptions { scope: Some(Scope::Block), whitelist: None };
    ///
    /// let attributor = ClassAttributor::new("align", "text-align", options);
    /// assert_eq!(attributor.get_class_name("center"), "text-align-center");
    /// ```
    pub fn get_class_name(&self, value: &str) -> String {
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
    /// ```rust,no_run
    /// # fn main() -> Result<(), wasm_bindgen::JsValue> {
    /// use quillai_parchment::attributor::{ClassAttributor, AttributorOptions};
    /// use quillai_parchment::dom::Dom;
    /// use wasm_bindgen::JsValue;
    ///
    /// // Create a new block element
    /// let element = Dom::create_element("p")?;
    ///
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

/// WebAssembly bindings for ClassAttributor with builder pattern constructors
///
/// This implementation provides JavaScript-friendly constructors and methods
/// for CSS class-based formatting using prefix-value patterns while maintaining
/// the full functionality of the Rust implementation.
#[wasm_bindgen]
impl ClassAttributor {
    /// Creates a new ClassAttributor with default options
    ///
    /// This is the basic constructor that creates a class attributor with default scope
    /// (Attribute) and no value restrictions. Perfect for basic CSS class management
    /// using the prefix-value pattern.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The CSS class prefix for generating class names
    ///
    /// # Returns
    /// New ClassAttributor instance with default configuration
    ///
    /// # Class Pattern
    /// Generated classes follow the pattern: `{key_name}-{value}`
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor } from './pkg/quillai_parchment.js';
    ///
    /// // Create a basic alignment attributor
    /// const alignAttr = new ClassAttributor("align", "text-align");
    ///
    /// // Use with DOM elements
    /// const element = document.createElement('div');
    /// const success = alignAttr.add(element, "center");
    /// // Results in: class="text-align-center"
    /// const currentAlign = alignAttr.value(element);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(attr_name: String, key_name: String) -> Self {
        Self::new_with_options(&attr_name, &key_name, AttributorOptions::default())
    }

    /// Creates a new ClassAttributor with a specific scope
    ///
    /// This constructor allows you to specify the scope classification for the
    /// class attributor, which affects how it interacts with the document model.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The CSS class prefix for generating class names
    /// * `scope` - The scope classification for this attributor
    ///
    /// # Returns
    /// New ClassAttributor instance with the specified scope
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor, Scope } from './pkg/quillai_parchment.js';
    ///
    /// // Create an inline-scoped size attributor
    /// const sizeAttr = ClassAttributor.newWithScope("size", "font-size", Scope.Inline);
    ///
    /// // Create a block-scoped alignment attributor
    /// const alignAttr = ClassAttributor.newWithScope("align", "text-align", Scope.Block);
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

    /// Creates a new ClassAttributor with a whitelist of allowed values
    ///
    /// This constructor creates a class attributor that only accepts values from
    /// the provided whitelist. Any attempt to set a value not in the whitelist
    /// will be rejected. Perfect for restricting to specific design system values.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The CSS class prefix for generating class names
    /// * `whitelist` - JavaScript array of allowed values
    ///
    /// # Returns
    /// New ClassAttributor instance with value validation
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor } from './pkg/quillai_parchment.js';
    ///
    /// // Create a restricted size attributor
    /// const sizeAttr = ClassAttributor.newWithWhitelist(
    ///     "size",
    ///     "font-size",
    ///     ["xs", "sm", "md", "lg", "xl"]
    /// );
    ///
    /// // This will succeed
    /// sizeAttr.add(element, "lg");
    ///
    /// // This will fail (returns false)
    /// sizeAttr.add(element, "invalid-size");
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

    /// Creates a new ClassAttributor with both scope and whitelist
    ///
    /// This is the most comprehensive constructor that allows you to specify
    /// both the scope classification and a whitelist of allowed values.
    ///
    /// # Arguments
    /// * `attr_name` - The logical attribute name used by Parchment
    /// * `key_name` - The CSS class prefix for generating class names
    /// * `scope` - The scope classification for this attributor
    /// * `whitelist` - JavaScript array of allowed values
    ///
    /// # Returns
    /// New ClassAttributor instance with full configuration
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor, Scope } from './pkg/quillai_parchment.js';
    ///
    /// // Create a fully configured color attributor
    /// const colorAttr = ClassAttributor.newFull(
    ///     "color",
    ///     "text-color",
    ///     Scope.Inline,
    ///     ["primary", "secondary", "accent", "muted"]
    /// );
    ///
    /// // Use with validation and proper scope
    /// const success = colorAttr.add(textElement, "primary");
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

    /// Adds a CSS class to the specified DOM element using the prefix pattern
    ///
    /// Attempts to add a CSS class to the given DOM element using the prefix-value
    /// pattern. The operation will fail if the value is not allowed by the whitelist
    /// (if configured) or if the DOM operation fails.
    ///
    /// # Arguments
    /// * `element` - The DOM element to modify
    /// * `value` - The value to append to the prefix (will be converted to string)
    ///
    /// # Returns
    /// `true` if the class was successfully added, `false` otherwise
    ///
    /// # Class Generation
    /// The generated class name follows the pattern: `{key_name}-{value}`
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const alignAttr = new ClassAttributor("align", "text-align");
    /// const element = document.createElement('div');
    ///
    /// // Add alignment class
    /// const success = alignAttr.add(element, "center");
    /// if (success) {
    ///     console.log("Class added successfully");
    ///     // element.className now includes "text-align-center"
    /// }
    /// ```
    pub fn add(&self, element: &Element, value: &JsValue) -> bool {
        // Check for null element to prevent JavaScript null pointer errors
        if element.is_null() {
            return false;
        }
        AttributorTrait::add(self, element, value)
    }

    /// Removes all CSS classes matching the prefix pattern from the specified DOM element
    ///
    /// Removes all CSS classes that match the prefix pattern from the given DOM element.
    /// This operation always succeeds, even if no matching classes were present.
    /// Only classes matching the prefix pattern are removed; other classes are preserved.
    ///
    /// # Arguments
    /// * `element` - The DOM element to modify
    ///
    /// # Class Removal
    /// Removes all classes that start with `{key_name}-`
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const alignAttr = new ClassAttributor("align", "text-align");
    /// const element = document.querySelector('div');
    ///
    /// // Remove all alignment classes
    /// alignAttr.remove(element);
    /// // Removes "text-align-center", "text-align-left", etc., but preserves other classes
    /// ```
    pub fn remove(&self, element: &Element) {
        // Check for null element to prevent JavaScript null pointer errors
        if element.is_null() {
            return;
        }
        AttributorTrait::remove(self, element)
    }

    /// Gets the current CSS class value from the specified DOM element
    ///
    /// Retrieves the current value from CSS classes that match the prefix pattern
    /// on the given DOM element. Returns an empty string if no matching class is found
    /// or if the current value is not allowed by the whitelist.
    ///
    /// # Arguments
    /// * `element` - The DOM element to inspect
    ///
    /// # Returns
    /// The value portion of the matching class, or empty string if not present/invalid
    ///
    /// # Value Extraction
    /// Extracts the value from classes matching `{key_name}-{value}` pattern
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const alignAttr = new ClassAttributor("align", "text-align");
    /// const element = document.querySelector('div');
    ///
    /// // Get the current alignment value
    /// const currentAlign = alignAttr.value(element);
    /// console.log("Current alignment:", currentAlign); // e.g., "center"
    /// ```
    pub fn value(&self, element: &Element) -> JsValue {
        // Check for null element to prevent JavaScript null pointer errors
        if element.is_null() {
            return JsValue::from_str("");
        }
        AttributorTrait::value(self, element)
    }

    /// Generates a CSS class name from a value using the prefix pattern
    ///
    /// Creates a CSS class name by combining the key_name prefix with the
    /// provided value using a hyphen separator. This is useful for understanding
    /// what class names will be generated or for manual class manipulation.
    ///
    /// # Arguments
    /// * `value` - The value to append to the prefix
    ///
    /// # Returns
    /// Generated CSS class name in the format `{key_name}-{value}`
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const alignAttr = new ClassAttributor("align", "text-align");
    /// 
    /// console.log(alignAttr.getClassName("center")); // "text-align-center"
    /// console.log(alignAttr.getClassName("left"));   // "text-align-left"
    /// console.log(alignAttr.getClassName("right"));  // "text-align-right"
    /// ```
    #[wasm_bindgen(js_name = "getClassName")]
    pub fn generate_class_name(&self, value: String) -> String {
        format!("{}-{}", self.key_name, value)
    }

    /// Gets the logical attribute name used by Parchment
    ///
    /// Returns the logical name that Parchment uses to identify this class attributor.
    /// This may be different from the actual CSS class prefix.
    ///
    /// # Returns
    /// The attribute name as a string
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const alignAttr = new ClassAttributor("textAlign", "text-align");
    /// console.log("Attribute name:", alignAttr.attrName()); // "textAlign"
    /// ```
    #[wasm_bindgen(js_name = "attrName")]
    pub fn attr_name(&self) -> String {
        AttributorTrait::attr_name(self).to_string()
    }

    /// Gets the CSS class prefix used for generating class names
    ///
    /// Returns the prefix that this attributor uses for generating CSS class names.
    /// This is combined with values to create the full class names.
    ///
    /// # Returns
    /// The class prefix as a string
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor } from './pkg/quillai_parchment.js';
    ///
    /// const alignAttr = new ClassAttributor("textAlign", "text-align");
    /// console.log("Class prefix:", alignAttr.keyName()); // "text-align"
    /// ```
    #[wasm_bindgen(js_name = "keyName")]
    pub fn key_name(&self) -> String {
        AttributorTrait::key_name(self).to_string()
    }

    /// Gets the scope classification for this class attributor
    ///
    /// Returns the scope that determines how this attributor interacts with
    /// the document model and other blots.
    ///
    /// # Returns
    /// The Scope enum value
    ///
    /// # Examples
    /// ```javascript
    /// import { ClassAttributor, Scope } from './pkg/quillai_parchment.js';
    ///
    /// const alignAttr = ClassAttributor.newWithScope("align", "text-align", Scope.Block);
    /// const scope = alignAttr.scope();
    ///
    /// if (scope === Scope.BlockAttribute) {
    ///     console.log("This is a block-level class attribute");
    /// }
    /// ```
    pub fn scope(&self) -> Scope {
        AttributorTrait::scope(self)
    }
}
