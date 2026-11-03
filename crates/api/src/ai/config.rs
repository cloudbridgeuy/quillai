use super::provider::OpenAiCompatibleConfig;
use crate::error::Error;
use serde::Serialize;

#[derive(Default)]
pub struct SavedSettings {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
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
    pub model: Source,
}

pub struct Resolved {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
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
    let (model, model_source) = choose(saved.model, env("QUILLAI_AI_MODEL"));
    Resolved {
        base_url,
        api_key,
        model,
        sources: Sources {
            base_url: base_source,
            api_key: key_source,
            model: model_source,
        },
    }
}

impl Resolved {
    pub fn provider_config(&self, model: Option<&str>) -> Result<OpenAiCompatibleConfig, Error> {
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
            model: required(model.or(self.model.as_deref()), "QUILLAI_AI_MODEL")?,
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
                model: saved.map(|value| format!("{value}-model")),
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
                (resolved.model, resolved.sources.model, "model"),
            ] {
                assert_eq!(value, expected.map(|value| format!("{value}-{field}")));
                assert_eq!(actual_source, source);
            }
        }
    }

    #[test]
    fn resolves_saved_values_before_environment_and_requires_credentials() {
        let saved = SavedSettings {
            model: Some("saved".into()),
            ..Default::default()
        };
        let resolved = resolve(saved, |key| match key {
            "QUILLAI_AI_MODEL" => Some("env".into()),
            "QUILLAI_AI_API_KEY" => Some("key".into()),
            _ => None,
        });
        assert_eq!(resolved.model.as_deref(), Some("saved"));
        assert_eq!(resolved.sources.model, Source::Saved);
        assert_eq!(resolved.sources.api_key, Source::Env);
        assert!(resolved.provider_config(None).is_ok());
        assert!(matches!(
            resolve(SavedSettings::default(), |_| None).provider_config(None),
            Err(crate::error::Error::AiNotConfigured)
        ));
    }
}
