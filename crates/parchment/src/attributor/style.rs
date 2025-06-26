use crate::attributor::base::AttributorOptions;
use crate::registry::AttributorTrait;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement};

pub struct StyleAttributor {
    pub attr_name: String,
    pub key_name: String,
    pub scope: Scope,
    pub whitelist: Option<Vec<String>>,
}

impl StyleAttributor {
    pub fn new(attr_name: &str, key_name: &str, options: AttributorOptions) -> Self {
        let scope = if let Some(opt_scope) = options.scope {
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
