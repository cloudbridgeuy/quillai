use std::collections::BTreeMap;

/// Represents an attribute value in the Delta format
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributeValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Null,
}

impl AttributeValue {
    /// Check if this attribute value represents a null/removal
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

/// Represents a map of attributes for formatting
pub type AttributeMap = BTreeMap<String, AttributeValue>;

/// Utility functions for working with AttributeMaps
pub struct AttributeMapOps;

impl AttributeMapOps {
    /// Compose two attribute maps, with `b` taking precedence
    /// If `keep_null` is true, null values are preserved for removal
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

    /// Create a diff between two attribute maps
    /// Returns None if no differences, otherwise returns the changes needed
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

    /// Create an inverted attribute map for undo operations
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

    /// Transform attributes for operational transformation
    /// If `priority` is true, `a` takes precedence in conflicts
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

    /// Check if an attribute map is empty or contains only null values
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