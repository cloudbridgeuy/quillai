use super::models::AiAction;
use super::provider::OpenAiCompatibleConfig;
use crate::error::Error;
use serde::Serialize;

#[derive(Default)]
pub struct SavedSettings {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub generate_model: Option<String>,
    pub edit_model: Option<String>,
    pub image_model: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Saved,
    Env,
    None,
}

#[derive(Serialize)]
pub struct Sources {
    pub base_url: Source,
    pub api_key: Source,
    pub generate_model: Source,
    pub edit_model: Source,
    pub image_model: Source,
}

pub struct Resolved {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub generate_model: Option<String>,
    pub edit_model: Option<String>,
    pub image_model: Option<String>,
    pub sources: Sources,
}

pub fn resolve(saved: SavedSettings, mut env: impl FnMut(&str) -> Option<String>) -> Resolved {
    fn choose(saved: Option<String>, env: Option<String>) -> (Option<String>, Source) {
        match (saved, env) {
            (Some(value), _) => (Some(value), Source::Saved),
            (None, Some(value)) => (Some(value), Source::Env),
            (None, None) => (None, Source::None),
        }
    }
    let (base_url, base_source) = choose(saved.base_url, env("QUILLAI_AI_BASE_URL"));
    let (api_key, key_source) = choose(saved.api_key, env("QUILLAI_AI_API_KEY"));
    let env_model = env("QUILLAI_AI_MODEL");
    let (generate_model, generate_source) = choose(saved.generate_model, env_model.clone());
    let (edit_model, edit_source) = choose(saved.edit_model, env_model);
    let (image_model, image_source) = choose(saved.image_model, env("QUILLAI_AI_IMAGE_MODEL"));
    Resolved {
        base_url,
        api_key,
        generate_model,
        edit_model,
        image_model,
        sources: Sources {
            base_url: base_source,
            api_key: key_source,
            generate_model: generate_source,
            edit_model: edit_source,
            image_model: image_source,
        },
    }
}

impl Resolved {
    pub fn model(&self, action: AiAction) -> Option<&str> {
        match action {
            AiAction::Generate => self.generate_model.as_deref(),
            AiAction::Edit => self.edit_model.as_deref(),
            AiAction::Image => self.image_model.as_deref(),
        }
    }

    pub fn model_source(&self, action: AiAction) -> Source {
        match action {
            AiAction::Generate => self.sources.generate_model,
            AiAction::Edit => self.sources.edit_model,
            AiAction::Image => self.sources.image_model,
        }
    }

    pub fn provider_config(
        &self,
        action: AiAction,
        model: Option<&str>,
    ) -> Result<OpenAiCompatibleConfig, Error> {
        fn required(value: Option<&str>, name: &str) -> Result<String, Error> {
            let value = value.ok_or(Error::AiNotConfigured)?.trim();
            if value.is_empty() {
                return Err(Error::AiProviderConfig(format!(
                    "environment variable {name} must not be blank"
                )));
            }
            Ok(value.into())
        }
        Ok(OpenAiCompatibleConfig {
            model: required(
                model.or(self.model(action)),
                match action {
                    AiAction::Image => "QUILLAI_AI_IMAGE_MODEL",
                    AiAction::Generate | AiAction::Edit => "QUILLAI_AI_MODEL",
                },
            )?,
            api_key: required(self.api_key.as_deref(), "QUILLAI_AI_API_KEY")?,
            base_url: self
                .base_url
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("https://api.openai.com/v1")
                .trim()
                .into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_each_field_and_source_for_all_saved_environment_combinations() {
        for (saved, env, expected, source) in [
            (Some("saved"), Some("env"), Some("saved"), Source::Saved),
            (Some("saved"), None, Some("saved"), Source::Saved),
            (None, Some("env"), Some("env"), Source::Env),
            (None, None, None, Source::None),
        ] {
            let saved = SavedSettings {
                base_url: saved.map(|value| format!("{value}-base_url")),
                api_key: saved.map(|value| format!("{value}-api_key")),
                generate_model: saved.map(|value| format!("{value}-model")),
                edit_model: saved.map(|value| format!("{value}-model")),
                image_model: saved.map(|value| format!("{value}-image_model")),
            };
            let resolved = resolve(saved, |key| {
                env.map(|value| {
                    format!(
                        "{value}-{}",
                        key.trim_start_matches("QUILLAI_AI_").to_lowercase()
                    )
                })
            });
            for (value, actual_source, field) in [
                (resolved.base_url, resolved.sources.base_url, "base_url"),
                (resolved.api_key, resolved.sources.api_key, "api_key"),
                (
                    resolved.generate_model,
                    resolved.sources.generate_model,
                    "model",
                ),
                (resolved.edit_model, resolved.sources.edit_model, "model"),
                (
                    resolved.image_model,
                    resolved.sources.image_model,
                    "image_model",
                ),
            ] {
                assert_eq!(value, expected.map(|value| format!("{value}-{field}")));
                assert_eq!(actual_source, source);
            }
        }
    }

    #[test]
    fn resolves_saved_values_before_environment_and_requires_credentials() {
        let saved = SavedSettings {
            generate_model: Some("saved".into()),
            ..Default::default()
        };
        let resolved = resolve(saved, |key| match key {
            "QUILLAI_AI_MODEL" => Some("env".into()),
            "QUILLAI_AI_API_KEY" => Some("key".into()),
            _ => None,
        });
        assert_eq!(resolved.model(AiAction::Generate), Some("saved"));
        assert_eq!(resolved.sources.generate_model, Source::Saved);
        assert_eq!(resolved.model(AiAction::Edit), Some("env"));
        assert_eq!(resolved.sources.edit_model, Source::Env);
        assert_eq!(resolved.sources.api_key, Source::Env);
        assert!(resolved.provider_config(AiAction::Generate, None).is_ok());
        assert!(matches!(
            resolve(SavedSettings::default(), |_| None).provider_config(AiAction::Generate, None),
            Err(crate::error::Error::AiNotConfigured)
        ));
    }

    #[test]
    fn image_model_reads_its_own_env_key_without_text_model_fallback() {
        let text_only = resolve(SavedSettings::default(), |key| {
            (key == "QUILLAI_AI_MODEL").then(|| "text".into())
        });
        assert_eq!(text_only.model(AiAction::Image), None);
        assert_eq!(text_only.sources.image_model, Source::None);
        let env = resolve(SavedSettings::default(), |key| {
            (key == "QUILLAI_AI_IMAGE_MODEL").then(|| "env-image".into())
        });
        assert_eq!(env.model(AiAction::Image), Some("env-image"));
        assert_eq!(env.sources.image_model, Source::Env);
        let saved = SavedSettings {
            image_model: Some("saved-image".into()),
            ..Default::default()
        };
        let saved = resolve(saved, |_| Some("env".into()));
        assert_eq!(saved.model(AiAction::Image), Some("saved-image"));
        assert_eq!(saved.model_source(AiAction::Image), Source::Saved);
    }

    #[test]
    fn blank_image_model_error_names_the_image_env_key() {
        let resolved = resolve(SavedSettings::default(), |key| {
            (key == "QUILLAI_AI_API_KEY").then(|| "key".into())
        });
        let error = resolved
            .provider_config(AiAction::Image, Some("  "))
            .unwrap_err()
            .to_string();
        assert!(error.contains("QUILLAI_AI_IMAGE_MODEL"), "{error}");
    }
}
