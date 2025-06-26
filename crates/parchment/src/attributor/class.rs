use crate::attributor::base::AttributorOptions;
use crate::registry::AttributorTrait;
use crate::scope::Scope;
use wasm_bindgen::prelude::*;
use web_sys::Element;

pub struct ClassAttributor {
    pub attr_name: String,
    pub key_name: String,
    pub scope: Scope,
    pub whitelist: Option<Vec<String>>,
}

impl ClassAttributor {
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

    fn get_class_name(&self, value: &str) -> String {
        format!("{}-{}", self.key_name, value)
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
