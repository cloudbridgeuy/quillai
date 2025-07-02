//! Attribute system for rich text formatting
//!
//! This module provides the attribute system used to represent formatting
//! and metadata in Delta documents. Attributes are key-value pairs that
//! can be attached to any operation to specify formatting like bold, italic,
//! color, font, or custom application-specific properties.
//!
//! # Key Concepts
//!
//! - **Attributes**: Key-value pairs representing formatting or metadata
//! - **Composition**: Combining attributes from sequential operations
//! - **Transformation**: Adjusting attributes for concurrent edits
//! - **Null values**: Special values that indicate attribute removal
//!
//! # Common Attributes
//!
//! While the Delta format doesn't prescribe specific attributes, common ones include:
//! - `bold`: Boolean indicating bold text
//! - `italic`: Boolean indicating italic text
//! - `underline`: Boolean indicating underlined text
//! - `strike`: Boolean indicating strikethrough text
//! - `color`: String with color value (e.g., "#ff0000" or "red")
//! - `background`: String with background color
//! - `font`: String with font family name
//! - `size`: Number with font size
//! - `link`: String with URL for hyperlinks
//! - `header`: Number indicating header level (1-6)
//! - `list`: String indicating list type ("bullet" or "ordered")
//! - `indent`: Number indicating indentation level
//! - `align`: String indicating text alignment ("left", "center", "right", "justify")

use std::collections::BTreeMap;

/// Represents a value that can be assigned to an attribute
///
/// The Delta format supports four types of attribute values:
/// - Strings for text values (colors, fonts, URLs, etc.)
/// - Numbers for numeric values (sizes, levels, etc.)
/// - Booleans for on/off formatting (bold, italic, etc.)
/// - Null to indicate attribute removal
///
/// # Examples
///
/// ```rust
/// use quillai_delta::AttributeValue;
///
/// // Different attribute value types
/// let color = AttributeValue::String("#ff0000".to_string());
/// let size = AttributeValue::Number(14);
/// let bold = AttributeValue::Boolean(true);
/// let remove = AttributeValue::Null;
///
/// // Using the From trait for convenience
/// let italic: AttributeValue = true.into();
/// let font: AttributeValue = "Arial".into();
/// let indent: AttributeValue = 2i32.into();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributeValue {
    /// A string value (colors, fonts, URLs, etc.)
    String(String),
    /// A numeric value (sizes, levels, counts, etc.)
    Number(i64),
    /// A boolean value (on/off formatting)
    Boolean(bool),
    /// Null indicates the attribute should be removed
    Null,
}

impl AttributeValue {
    /// Checks if this value represents a null/removal
    ///
    /// Null values are used in retain operations to indicate that
    /// an attribute should be removed from the content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::AttributeValue;
    ///
    /// let null_val = AttributeValue::Null;
    /// assert!(null_val.is_null());
    ///
    /// let string_val = AttributeValue::String("test".to_string());
    /// assert!(!string_val.is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        matches!(self, AttributeValue::Null)
    }
}

impl From<String> for AttributeValue {
    fn from(s: String) -> Self {
        AttributeValue::String(s)
    }
}

impl From<&str> for AttributeValue {
    fn from(s: &str) -> Self {
        AttributeValue::String(s.to_string())
    }
}

impl From<i64> for AttributeValue {
    fn from(n: i64) -> Self {
        AttributeValue::Number(n)
    }
}

impl From<i32> for AttributeValue {
    fn from(n: i32) -> Self {
        AttributeValue::Number(n as i64)
    }
}

impl From<bool> for AttributeValue {
    fn from(b: bool) -> Self {
        AttributeValue::Boolean(b)
    }
}

/// A map of attribute names to their values
///
/// Uses BTreeMap to ensure consistent ordering of attributes, which is
/// important for equality comparisons and serialization.
///
/// # Examples
///
/// ```rust
/// use quillai_delta::{AttributeMap, AttributeValue};
///
/// let mut attrs = AttributeMap::new();
/// attrs.insert("bold".to_string(), AttributeValue::Boolean(true));
/// attrs.insert("color".to_string(), AttributeValue::String("#ff0000".to_string()));
/// attrs.insert("size".to_string(), AttributeValue::Number(14));
/// ```
pub type AttributeMap = BTreeMap<String, AttributeValue>;

/// Utility operations for working with attribute maps
///
/// This struct provides static methods for composing, transforming,
/// and manipulating attribute maps in ways that support the Delta
/// format's operational transformation requirements.
pub struct AttributeMapOps;

impl AttributeMapOps {
    /// Composes two attribute maps, with the second taking precedence
    ///
    /// Composition is used when applying sequential operations. Attributes
    /// from `b` override those in `a`. The `keep_null` parameter determines
    /// whether null values in `b` are preserved (for explicit removal) or
    /// ignored.
    ///
    /// # Arguments
    ///
    /// * `a` - The first attribute map (base attributes)
    /// * `b` - The second attribute map (overriding attributes)
    /// * `keep_null` - If true, null values in `b` are kept; if false, they're ignored
    ///
    /// # Returns
    ///
    /// Returns `None` if the result would be empty, otherwise returns the composed attributes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{AttributeMap, AttributeValue, attributes::AttributeMapOps};
    ///
    /// let mut a = AttributeMap::new();
    /// a.insert("bold".to_string(), AttributeValue::Boolean(true));
    /// a.insert("color".to_string(), AttributeValue::String("red".to_string()));
    ///
    /// let mut b = AttributeMap::new();
    /// b.insert("color".to_string(), AttributeValue::String("blue".to_string()));
    /// b.insert("italic".to_string(), AttributeValue::Boolean(true));
    ///
    /// let result = AttributeMapOps::compose(Some(&a), Some(&b), false).unwrap();
    /// // Result: bold=true, color="blue", italic=true
    /// ```
    pub fn compose(
        a: Option<&AttributeMap>,
        b: Option<&AttributeMap>,
        keep_null: bool,
    ) -> Option<AttributeMap> {
        let empty_a = BTreeMap::new();
        let empty_b = BTreeMap::new();
        let a = a.unwrap_or(&empty_a);
        let b = b.unwrap_or(&empty_b);

        let mut result = BTreeMap::new();

        // Start with attributes from `a`
        for (key, value) in a {
            if !b.contains_key(key) {
                result.insert(key.clone(), value.clone());
            }
        }

        // Add attributes from `b`, potentially overriding `a`
        for (key, value) in b {
            if keep_null || !value.is_null() {
                result.insert(key.clone(), value.clone());
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Creates a diff between two attribute maps
    ///
    /// The diff contains the minimal set of changes needed to transform
    /// attributes from `a` to `b`. Attributes that need to be removed
    /// are represented with null values.
    ///
    /// # Arguments
    ///
    /// * `a` - The source attribute map
    /// * `b` - The target attribute map
    ///
    /// # Returns
    ///
    /// Returns `None` if the attributes are identical, otherwise returns
    /// the changes needed to transform `a` into `b`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{AttributeMap, AttributeValue, attributes::AttributeMapOps};
    ///
    /// let mut a = AttributeMap::new();
    /// a.insert("bold".to_string(), AttributeValue::Boolean(true));
    /// a.insert("color".to_string(), AttributeValue::String("red".to_string()));
    ///
    /// let mut b = AttributeMap::new();
    /// b.insert("bold".to_string(), AttributeValue::Boolean(false));
    /// b.insert("italic".to_string(), AttributeValue::Boolean(true));
    ///
    /// let diff = AttributeMapOps::diff(Some(&a), Some(&b)).unwrap();
    /// // diff contains: bold=false, color=null, italic=true
    /// ```
    pub fn diff(
        a: Option<&AttributeMap>,
        b: Option<&AttributeMap>,
    ) -> Option<AttributeMap> {
        let empty_a = BTreeMap::new();
        let empty_b = BTreeMap::new();
        let a = a.unwrap_or(&empty_a);
        let b = b.unwrap_or(&empty_b);

        let mut result = BTreeMap::new();

        // Find all keys that exist in either map
        let mut all_keys: std::collections::BTreeSet<&String> = std::collections::BTreeSet::new();
        all_keys.extend(a.keys());
        all_keys.extend(b.keys());

        for key in all_keys {
            let a_val = a.get(key);
            let b_val = b.get(key);

            if a_val != b_val {
                match b_val {
                    Some(val) => {
                        result.insert(key.clone(), val.clone());
                    }
                    None => {
                        result.insert(key.clone(), AttributeValue::Null);
                    }
                }
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Creates an inverted attribute map for undo operations
    ///
    /// Given a set of attribute changes and the base attributes they were
    /// applied to, this creates the inverse changes that would restore
    /// the original attributes.
    ///
    /// # Arguments
    ///
    /// * `attr` - The attribute changes that were applied
    /// * `base` - The original attributes before the changes
    ///
    /// # Returns
    ///
    /// Returns the attribute changes needed to undo `attr` and restore `base`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{AttributeMap, AttributeValue, attributes::AttributeMapOps};
    ///
    /// let mut changes = AttributeMap::new();
    /// changes.insert("bold".to_string(), AttributeValue::Boolean(true));
    /// changes.insert("color".to_string(), AttributeValue::Null); // Remove color
    ///
    /// let mut base = AttributeMap::new();
    /// base.insert("color".to_string(), AttributeValue::String("red".to_string()));
    /// base.insert("size".to_string(), AttributeValue::Number(12));
    ///
    /// let inverted = AttributeMapOps::invert(Some(&changes), Some(&base)).unwrap();
    /// // inverted contains: bold=null, color="red", size=12
    /// ```
    pub fn invert(
        attr: Option<&AttributeMap>,
        base: Option<&AttributeMap>,
    ) -> Option<AttributeMap> {
        let empty_attr = BTreeMap::new();
        let empty_base = BTreeMap::new();
        let attr = attr.unwrap_or(&empty_attr);
        let base = base.unwrap_or(&empty_base);

        let mut result = BTreeMap::new();

        // For each attribute in the change
        for (key, value) in attr {
            let base_value = base.get(key);
            
            if base_value != Some(value) {
                match base_value {
                    Some(base_val) => {
                        result.insert(key.clone(), base_val.clone());
                    }
                    None => {
                        result.insert(key.clone(), AttributeValue::Null);
                    }
                }
            }
        }

        // For attributes that were in base but removed in attr
        for (key, value) in base {
            if !attr.contains_key(key) {
                result.insert(key.clone(), value.clone());
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Transforms attributes for operational transformation
    ///
    /// This is used when two concurrent operations need to be reconciled.
    /// The `priority` parameter determines which operation takes precedence
    /// when both modify the same attribute.
    ///
    /// # Arguments
    ///
    /// * `a` - Attributes from the first operation
    /// * `b` - Attributes from the second operation
    /// * `priority` - If true, `a` takes precedence in conflicts; if false, `b` takes precedence
    ///
    /// # Returns
    ///
    /// Returns the transformed attributes from `b` that don't conflict with `a`
    /// (when priority is true), or all of `b` (when priority is false).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{AttributeMap, AttributeValue, attributes::AttributeMapOps};
    ///
    /// let mut a = AttributeMap::new();
    /// a.insert("bold".to_string(), AttributeValue::Boolean(true));
    ///
    /// let mut b = AttributeMap::new();
    /// b.insert("bold".to_string(), AttributeValue::Boolean(false));
    /// b.insert("italic".to_string(), AttributeValue::Boolean(true));
    ///
    /// // With priority, a's bold=true takes precedence
    /// let result = AttributeMapOps::transform(Some(&a), Some(&b), true).unwrap();
    /// // result contains only: italic=true
    ///
    /// // Without priority, b's attributes take precedence
    /// let result = AttributeMapOps::transform(Some(&a), Some(&b), false).unwrap();
    /// // result contains: bold=false, italic=true
    /// ```
    pub fn transform(
        a: Option<&AttributeMap>,
        b: Option<&AttributeMap>,
        priority: bool,
    ) -> Option<AttributeMap> {
        let empty_a = BTreeMap::new();
        let empty_b = BTreeMap::new();
        let a = a.unwrap_or(&empty_a);
        let b = b.unwrap_or(&empty_b);

        if !priority {
            // b simply overwrites a without priority
            return if b.is_empty() { None } else { Some(b.clone()) };
        }

        let mut result = BTreeMap::new();

        // With priority, only add attributes from b that don't conflict with a
        for (key, value) in b {
            if !a.contains_key(key) {
                result.insert(key.clone(), value.clone());
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Checks if an attribute map is empty or contains only null values
    ///
    /// This is useful for determining whether an operation actually has
    /// meaningful attributes or if they can be omitted.
    ///
    /// # Arguments
    ///
    /// * `attrs` - The attribute map to check
    ///
    /// # Returns
    ///
    /// Returns true if the map is None, empty, or contains only null values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{AttributeMap, AttributeValue, attributes::AttributeMapOps};
    ///
    /// assert!(AttributeMapOps::is_empty_or_null(None));
    ///
    /// let empty = AttributeMap::new();
    /// assert!(AttributeMapOps::is_empty_or_null(Some(&empty)));
    ///
    /// let mut nulls = AttributeMap::new();
    /// nulls.insert("bold".to_string(), AttributeValue::Null);
    /// assert!(AttributeMapOps::is_empty_or_null(Some(&nulls)));
    ///
    /// let mut attrs = AttributeMap::new();
    /// attrs.insert("bold".to_string(), AttributeValue::Boolean(true));
    /// assert!(!AttributeMapOps::is_empty_or_null(Some(&attrs)));
    /// ```
    pub fn is_empty_or_null(attrs: Option<&AttributeMap>) -> bool {
        match attrs {
            None => true,
            Some(map) => map.is_empty() || map.values().all(|v| v.is_null()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_attrs(items: &[(&str, AttributeValue)]) -> AttributeMap {
        items.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn test_attribute_compose() {
        let a = create_attrs(&[
            ("bold", AttributeValue::Boolean(true)),
            ("color", AttributeValue::String("red".to_string())),
        ]);

        let b = create_attrs(&[
            ("italic", AttributeValue::Boolean(true)),
            ("color", AttributeValue::String("blue".to_string())),
        ]);

        let result = AttributeMapOps::compose(Some(&a), Some(&b), false).unwrap();

        assert_eq!(result.get("bold"), Some(&AttributeValue::Boolean(true)));
        assert_eq!(result.get("italic"), Some(&AttributeValue::Boolean(true)));
        assert_eq!(result.get("color"), Some(&AttributeValue::String("blue".to_string())));
    }

    #[test]
    fn test_attribute_compose_with_null() {
        let a = create_attrs(&[
            ("bold", AttributeValue::Boolean(true)),
            ("color", AttributeValue::String("red".to_string())),
        ]);

        let b = create_attrs(&[
            ("bold", AttributeValue::Null),
            ("italic", AttributeValue::Boolean(true)),
        ]);

        let result = AttributeMapOps::compose(Some(&a), Some(&b), true).unwrap();

        assert_eq!(result.get("bold"), Some(&AttributeValue::Null));
        assert_eq!(result.get("color"), Some(&AttributeValue::String("red".to_string())));
        assert_eq!(result.get("italic"), Some(&AttributeValue::Boolean(true)));
    }

    #[test]
    fn test_attribute_diff() {
        let a = create_attrs(&[
            ("bold", AttributeValue::Boolean(true)),
            ("color", AttributeValue::String("red".to_string())),
        ]);

        let b = create_attrs(&[
            ("bold", AttributeValue::Boolean(false)),
            ("italic", AttributeValue::Boolean(true)),
        ]);

        let result = AttributeMapOps::diff(Some(&a), Some(&b)).unwrap();

        assert_eq!(result.get("bold"), Some(&AttributeValue::Boolean(false)));
        assert_eq!(result.get("color"), Some(&AttributeValue::Null));
        assert_eq!(result.get("italic"), Some(&AttributeValue::Boolean(true)));
    }

    #[test]
    fn test_attribute_invert() {
        let attr = create_attrs(&[
            ("bold", AttributeValue::Boolean(true)),
            ("color", AttributeValue::Null),
        ]);

        let base = create_attrs(&[
            ("color", AttributeValue::String("red".to_string())),
            ("size", AttributeValue::Number(12)),
        ]);

        let result = AttributeMapOps::invert(Some(&attr), Some(&base)).unwrap();

        assert_eq!(result.get("bold"), Some(&AttributeValue::Null));
        assert_eq!(result.get("color"), Some(&AttributeValue::String("red".to_string())));
        assert_eq!(result.get("size"), Some(&AttributeValue::Number(12)));
    }

    #[test]
    fn test_attribute_transform() {
        let a = create_attrs(&[
            ("bold", AttributeValue::Boolean(true)),
        ]);

        let b = create_attrs(&[
            ("bold", AttributeValue::Boolean(false)),
            ("italic", AttributeValue::Boolean(true)),
        ]);

        // With priority, a takes precedence
        let result = AttributeMapOps::transform(Some(&a), Some(&b), true).unwrap();
        assert_eq!(result.get("italic"), Some(&AttributeValue::Boolean(true)));
        assert!(!result.contains_key("bold"));

        // Without priority, b takes precedence
        let result = AttributeMapOps::transform(Some(&a), Some(&b), false).unwrap();
        assert_eq!(result.get("bold"), Some(&AttributeValue::Boolean(false)));
        assert_eq!(result.get("italic"), Some(&AttributeValue::Boolean(true)));
    }
}