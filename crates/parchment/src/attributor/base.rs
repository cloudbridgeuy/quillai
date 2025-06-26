use crate::registry::AttributorTrait;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::Element;

#[derive(Default, Debug, Clone)]
pub struct AttributorOptions {
    pub scope: Option<Scope>,
    pub whitelist: Option<Vec<String>>,
}

pub struct Attributor {
    pub attr_name: String,
    pub key_name: String,
    pub scope: Scope,
    pub whitelist: Option<Vec<String>>,
}

impl Attributor {
    pub fn new(attr_name: &str, key_name: &str, options: AttributorOptions) -> Self {
        let scope = if let Some(opt_scope) = options.scope {
            // Force attribute bit while keeping level bits
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

    pub fn can_add(&self, _node: &Element, value: &JsValue) -> bool {
        if let Some(ref whitelist) = self.whitelist {
            if let Some(value_str) = value.as_string() {
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
