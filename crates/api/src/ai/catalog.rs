use std::{collections::HashSet, path::Path};

use serde::Deserialize;

use super::models::AiAction;
use crate::error::Error;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CatalogModel {
    pub id: String,
    pub label: String,
    pub input_price_per_million: f64,
    pub output_price_per_million: f64,
    #[serde(default = "default_include_usage")]
    pub include_usage: bool,
}

fn default_include_usage() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Defaults {
    generate: String,
    edit: String,
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
    generate: usize,
    edit: usize,
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
            if [
                model.input_price_per_million,
                model.output_price_per_million,
            ]
            .iter()
            .any(|price| !price.is_finite() || *price < 0.0)
            {
                return Err(config_error(format!(
                    "model {} prices must be finite and non-negative",
                    model.id
                )));
            }
        }
        let default_index = |action, id: &str| {
            file.models
                .iter()
                .position(|model| model.id == id)
                .ok_or_else(|| {
                    config_error(format!(
                        "default {action} model {id:?} is not in the catalog"
                    ))
                })
        };
        let generate = default_index("generate", &file.defaults.generate)?;
        let edit = default_index("edit", &file.defaults.edit)?;
        Ok(Self {
            models: file.models,
            generate,
            edit,
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
                input_price_per_million: 0.0,
                output_price_per_million: 0.0,
                include_usage: true,
            }],
            generate: 0,
            edit: 0,
        })
    }

    pub fn models(&self) -> &[CatalogModel] {
        &self.models
    }

    pub fn resolve(&self, action: AiAction) -> &CatalogModel {
        &self.models[match action {
            AiAction::Generate => self.generate,
            AiAction::Edit => self.edit,
        }]
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
        let generate = catalog.resolve(AiAction::Generate);
        assert_eq!(generate.id, "generate-model");
        assert_eq!(generate.label, "Generate");
        assert_eq!(generate.input_price_per_million, 0.1);
        assert_eq!(generate.output_price_per_million, 0.2);
        assert!(generate.include_usage);
        let edit = catalog.resolve(AiAction::Edit);
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
            assert_eq!(catalog.resolve(action).id, "legacy-model");
            assert!(catalog.resolve(action).include_usage);
        }
        assert!(load(None, None).unwrap().is_none());
        assert!(load(None, Some(" ")).is_err());
    }
}
