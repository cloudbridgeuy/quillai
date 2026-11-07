use std::{collections::HashSet, path::Path};

use serde::{Deserialize, Serialize};

use super::models::AiAction;
use crate::error::Error;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelKind {
    #[default]
    Text,
    Image,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CatalogModel {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub kind: ModelKind,
    pub input_price_per_million: Option<f64>,
    pub output_price_per_million: Option<f64>,
    pub price_per_image: Option<f64>,
    pub size: Option<String>,
    pub quality: Option<String>,
    #[serde(default = "default_include_usage")]
    pub include_usage: bool,
    pub reasoning_effort: Option<ReasoningEffort>,
}

impl CatalogModel {
    fn validate(&self) -> Result<(), Error> {
        let (prices, shape_ok) = match self.kind {
            ModelKind::Text => (
                vec![self.input_price_per_million, self.output_price_per_million],
                self.price_per_image.is_none() && self.size.is_none() && self.quality.is_none(),
            ),
            ModelKind::Image => (
                vec![self.price_per_image],
                [&self.size, &self.quality].iter().all(|value| {
                    value
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty())
                }),
            ),
        };
        if !shape_ok {
            return Err(config_error(format!(
                "model {}: image models need size and quality; text models must not set price_per_image, size and quality",
                self.id
            )));
        }
        if prices
            .iter()
            .any(|price| price.is_none_or(|price| !price.is_finite() || price < 0.0))
        {
            return Err(config_error(format!(
                "model {} prices must be present, finite, and non-negative",
                self.id
            )));
        }
        Ok(())
    }
}

fn default_include_usage() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Defaults {
    generate: String,
    edit: String,
    image: Option<String>,
    lore_complete: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogFile {
    models: Vec<CatalogModel>,
    defaults: Defaults,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelCatalog {
    models: Vec<CatalogModel>,
    file: bool,
    generate: usize,
    edit: usize,
    image: Option<usize>,
    lore_complete: usize,
}

impl ModelCatalog {
    pub fn parse(json: &str) -> Result<Self, Error> {
        let file: CatalogFile = serde_json::from_str(json)
            .map_err(|error| config_error(format!("invalid model catalog JSON: {error}")))?;
        let mut ids = HashSet::new();
        for model in &file.models {
            if model.id.trim().is_empty() || model.label.trim().is_empty() {
                return Err(config_error("model id and label must not be blank"));
            }
            if !ids.insert(&model.id) {
                return Err(config_error(format!("duplicate model id: {}", model.id)));
            }
            model.validate()?;
        }
        let default_index = |action, id: &str, kind| {
            let index = file
                .models
                .iter()
                .position(|model| model.id == id)
                .ok_or_else(|| {
                    config_error(format!(
                        "default {action} model {id:?} is not in the catalog"
                    ))
                })?;
            if file.models[index].kind != kind {
                return Err(config_error(format!(
                    "default {action} model {id:?} must be a {kind:?} model"
                )));
            }
            Ok(index)
        };
        let generate = default_index("generate", &file.defaults.generate, ModelKind::Text)?;
        let edit = default_index("edit", &file.defaults.edit, ModelKind::Text)?;
        let image = file
            .defaults
            .image
            .as_deref()
            .map(|id| default_index("image", id, ModelKind::Image))
            .transpose()?;
        let lore_complete = file
            .defaults
            .lore_complete
            .as_deref()
            .map(|id| default_index("lore_complete", id, ModelKind::Text))
            .transpose()?
            .unwrap_or(generate);
        Ok(Self {
            models: file.models,
            file: true,
            generate,
            edit,
            image,
            lore_complete,
        })
    }

    pub fn from_model(model: &str) -> Result<Self, Error> {
        let id = model.trim();
        if id.is_empty() {
            return Err(config_error("QUILLAI_AI_MODEL must not be blank"));
        }
        Ok(Self {
            models: vec![CatalogModel {
                id: id.into(),
                label: id.into(),
                kind: ModelKind::Text,
                input_price_per_million: Some(0.0),
                output_price_per_million: Some(0.0),
                price_per_image: None,
                size: None,
                quality: None,
                include_usage: true,
                reasoning_effort: None,
            }],
            file: false,
            generate: 0,
            edit: 0,
            image: None,
            lore_complete: 0,
        })
    }

    pub fn is_file(&self) -> bool {
        self.file
    }

    pub fn models(&self) -> &[CatalogModel] {
        &self.models
    }

    pub fn find(&self, id: &str) -> Option<&CatalogModel> {
        self.models.iter().find(|model| model.id == id)
    }

    pub fn resolve(&self, action: AiAction) -> Option<&CatalogModel> {
        match action {
            AiAction::Generate => Some(self.generate),
            AiAction::Edit => Some(self.edit),
            AiAction::Image => self.image,
            AiAction::LoreComplete => Some(self.lore_complete),
        }
        .map(|index| &self.models[index])
    }
}

pub fn load(path: Option<&Path>, fallback: Option<&str>) -> Result<Option<ModelCatalog>, Error> {
    match path {
        Some(path) => {
            let json = std::fs::read_to_string(path).map_err(|error| {
                config_error(format!(
                    "cannot read model catalog {}: {error}",
                    path.display()
                ))
            })?;
            ModelCatalog::parse(&json)
                .map(Some)
                .map_err(|error| config_error(format!("model catalog {}: {error}", path.display())))
        }
        None => fallback.map(ModelCatalog::from_model).transpose(),
    }
}

fn config_error(message: impl Into<String>) -> Error {
    Error::AiProviderConfig(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn file() -> serde_json::Value {
        json!({
            "models": [
                {"id":"generate-model","label":"Generate","input_price_per_million":0.1,"output_price_per_million":0.2},
                {"id":"edit-model","label":"Edit","input_price_per_million":1,"output_price_per_million":2,"include_usage":false}
            ],
            "defaults":{"generate":"generate-model","edit":"edit-model"}
        })
    }

    #[test]
    fn parse_and_resolve_per_action_defaults() {
        let catalog = ModelCatalog::parse(&file().to_string()).unwrap();
        let generate = catalog.resolve(AiAction::Generate).unwrap();
        assert_eq!(generate.id, "generate-model");
        assert_eq!(generate.label, "Generate");
        assert_eq!(generate.input_price_per_million, Some(0.1));
        assert_eq!(generate.output_price_per_million, Some(0.2));
        assert!(generate.include_usage);
        let edit = catalog.resolve(AiAction::Edit).unwrap();
        assert_eq!(edit.id, "edit-model");
        assert!(!edit.include_usage);
    }

    #[test]
    fn parse_rejects_invalid_catalogs() {
        for (pointer, value, message) in [
            ("/models", json!([]), "not in the catalog"),
            (
                "/models/1/id",
                json!("generate-model"),
                "duplicate model id",
            ),
            ("/models/0/id", json!(" "), "must not be blank"),
            ("/models/0/label", json!(""), "must not be blank"),
            (
                "/models/0/input_price_per_million",
                json!(-1),
                "non-negative",
            ),
            (
                "/models/0/output_price_per_million",
                json!(-1),
                "non-negative",
            ),
            ("/models/1/include_usage", json!("false"), "boolean"),
            ("/defaults/edit", json!("missing"), "default edit"),
            ("/defaults/generate", json!("missing"), "default generate"),
        ] {
            let mut file = file();
            *file.pointer_mut(pointer).unwrap() = value;
            let error = ModelCatalog::parse(&file.to_string())
                .unwrap_err()
                .to_string();
            assert!(error.contains(message), "{error}");
        }
        assert!(ModelCatalog::parse("not json")
            .unwrap_err()
            .to_string()
            .contains("invalid model catalog JSON"));
        let mut file = file();
        file["models"][0]
            .as_object_mut()
            .unwrap()
            .remove("input_price_per_million");
        assert!(ModelCatalog::parse(&file.to_string()).is_err());
    }

    #[test]
    fn no_file_uses_legacy_model_for_both_actions() {
        let catalog = load(None, Some(" legacy-model ")).unwrap().unwrap();
        assert_eq!(catalog.models.len(), 1);
        for action in [AiAction::Generate, AiAction::Edit] {
            let model = catalog.resolve(action).unwrap();
            assert_eq!(model.id, "legacy-model");
            assert!(model.include_usage);
            assert_eq!(model.reasoning_effort, None);
        }
        assert_eq!(catalog.resolve(AiAction::Image), None);
        assert!(load(None, None).unwrap().is_none());
        assert!(load(None, Some(" ")).is_err());
    }

    #[test]
    fn image_entries_parse_with_kind_and_per_image_price() {
        let catalog = ModelCatalog::parse(&file().to_string()).unwrap();
        assert_eq!(catalog.models()[0].kind, ModelKind::Text);
        assert_eq!(catalog.resolve(AiAction::Image), None);
        let mut file = file();
        file["models"].as_array_mut().unwrap().push(json!({"id":"gpt-image-1-mini","label":"Image mini","kind":"image","price_per_image":0.011,"size":"1024x1024","quality":"medium"}));
        file["defaults"]["image"] = json!("gpt-image-1-mini");
        let catalog = ModelCatalog::parse(&file.to_string()).unwrap();
        let image = catalog.resolve(AiAction::Image).unwrap();
        assert_eq!(image.kind, ModelKind::Image);
        assert_eq!(image.price_per_image, Some(0.011));
        assert_eq!(image.size.as_deref(), Some("1024x1024"));
        assert_eq!(image.quality.as_deref(), Some("medium"));
        for (pointer, value, message) in [
            (
                "/defaults/image",
                json!("generate-model"),
                "default image model",
            ),
            (
                "/defaults/edit",
                json!("gpt-image-1-mini"),
                "default edit model",
            ),
            ("/models/2/price_per_image", json!(-1), "non-negative"),
            ("/models/2/size", json!(" "), "size and quality"),
            ("/models/2/kind", json!("video"), "unknown variant"),
        ] {
            let mut file = file.clone();
            *file.pointer_mut(pointer).unwrap() = value;
            let error = ModelCatalog::parse(&file.to_string())
                .unwrap_err()
                .to_string();
            assert!(error.contains(message), "{pointer}: {error}");
        }
        let mut missing = file.clone();
        missing["models"][2]
            .as_object_mut()
            .unwrap()
            .remove("quality");
        assert!(ModelCatalog::parse(&missing.to_string()).is_err());
        let mut text_with_image_price = file.clone();
        text_with_image_price["models"][0]["price_per_image"] = json!(1);
        assert!(ModelCatalog::parse(&text_with_image_price.to_string()).is_err());
    }

    #[test]
    fn reasoning_effort_accepts_known_values_and_rejects_unknown() {
        let mut file = file();
        assert_eq!(
            ModelCatalog::parse(&file.to_string()).unwrap().models()[0].reasoning_effort,
            None
        );
        for (value, effort) in [
            ("none", ReasoningEffort::None),
            ("minimal", ReasoningEffort::Minimal),
            ("low", ReasoningEffort::Low),
            ("medium", ReasoningEffort::Medium),
            ("high", ReasoningEffort::High),
        ] {
            file["models"][0]["reasoning_effort"] = json!(value);
            assert_eq!(
                ModelCatalog::parse(&file.to_string()).unwrap().models()[0].reasoning_effort,
                Some(effort)
            );
            assert_eq!(serde_json::to_value(effort).unwrap(), json!(value));
        }
        file["models"][0]["reasoning_effort"] = json!("unknown");
        assert!(ModelCatalog::parse(&file.to_string()).is_err());
    }
}
