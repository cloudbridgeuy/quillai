use std::ffi::OsString;
use std::path::Path;

const KEYS: [&str; 4] = [
    "QUILLAI_AI_API_KEY",
    "QUILLAI_AI_MODEL",
    "QUILLAI_AI_BASE_URL",
    "QUILLAI_AI_MODELS_FILE",
];

#[derive(Default)]
pub(crate) struct CliSettings<'a> {
    pub model: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub models: Option<&'a Path>,
}

pub(crate) fn parse(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut pairs = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let error = |reason| format!("line {}: {reason}", index + 1);
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| error("expected KEY=value"))?;
        let key = key.trim();
        if !KEYS.contains(&key) {
            return Err(error("unknown key"));
        }
        if pairs.iter().any(|(existing, _)| existing == key) {
            return Err(error("duplicate key"));
        }
        let value = value.trim();
        if value.contains('\0') {
            return Err(error("NUL is not allowed"));
        }
        let value = if value.starts_with(['\'', '"']) {
            let quote = &value[..1];
            value[1..]
                .strip_suffix(quote)
                .filter(|value| !value.contains(quote))
                .ok_or_else(|| error("expected matching quotes around value"))?
        } else {
            value
        };
        pairs.push((key.to_string(), value.to_string()));
    }
    Ok(pairs)
}

pub(crate) fn resolve(
    file: &[(String, String)],
    mut shell: impl FnMut(&str) -> Option<OsString>,
    cli: CliSettings<'_>,
) -> Vec<(&'static str, OsString)> {
    KEYS.into_iter()
        .filter_map(|key| {
            let flag = match key {
                "QUILLAI_AI_MODEL" => cli.model.map(OsString::from),
                "QUILLAI_AI_BASE_URL" => cli.base_url.map(OsString::from),
                "QUILLAI_AI_MODELS_FILE" => cli.models.map(|path| path.as_os_str().to_owned()),
                _ => None,
            };
            flag.or_else(|| shell(key))
                .or_else(|| {
                    file.iter()
                        .find(|(name, _)| name == key)
                        .map(|(_, value)| value.into())
                })
                .map(|value| (key, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_assignments_quotes_comments_and_whitespace() {
        let text = "  # settings\r\n\n export QUILLAI_AI_API_KEY = 'secret=one # two' \r\nQUILLAI_AI_MODEL=\"model name\"\nQUILLAI_AI_BASE_URL = https://example.test/v1\nQUILLAI_AI_MODELS_FILE = models.json\n";
        let pairs = parse(text).unwrap();
        assert_eq!(
            pairs,
            KEYS.into_iter()
                .zip([
                    "secret=one # two",
                    "model name",
                    "https://example.test/v1",
                    "models.json"
                ])
                .map(|(key, value)| (key.into(), value.into()))
                .collect::<Vec<_>>()
        );
        assert!(parse("\n# empty file\n").unwrap().is_empty());
        assert_eq!(parse("QUILLAI_AI_API_KEY=\"\"").unwrap()[0].1, "");
    }

    #[test]
    fn rejects_invalid_lines_without_disclosing_values() {
        for line in [
            "QUILLAI_AI_API_KEY=private\nQUILLAI_AI_API_KEY=private",
            "UNKNOWN=private",
            "QUILLAI_AI_API_KEY private",
            "QUILLAI_AI_API_KEY='private",
            "QUILLAI_AI_API_KEY=\"private'",
            "QUILLAI_AI_API_KEY='private' trailing",
            "QUILLAI_AI_API_KEY=private\0",
        ] {
            let error = parse(&format!("# settings\n{line}")).unwrap_err();
            let expected_line = if line.contains('\n') { 3 } else { 2 };
            assert!(error.contains(&format!("line {expected_line}:")), "{error}");
            assert!(!error.contains("private"), "{error}");
        }
    }

    #[test]
    fn precedence_is_cli_then_shell_then_file_for_each_key() {
        for key in KEYS {
            for (has_file, has_shell, has_cli, expected) in [
                (false, false, false, None),
                (true, false, false, Some("file")),
                (false, true, false, Some("shell")),
                (false, false, true, Some("cli")),
                (true, true, false, Some("shell")),
                (true, false, true, Some("cli")),
                (false, true, true, Some("cli")),
                (true, true, true, Some("cli")),
            ] {
                let file = if has_file {
                    vec![(key.into(), "file".into())]
                } else {
                    vec![]
                };
                let cli = if has_cli {
                    CliSettings {
                        model: (key == "QUILLAI_AI_MODEL").then_some("cli"),
                        base_url: (key == "QUILLAI_AI_BASE_URL").then_some("cli"),
                        models: (key == "QUILLAI_AI_MODELS_FILE").then_some(Path::new("cli")),
                    }
                } else {
                    CliSettings::default()
                };
                let result = resolve(
                    &file,
                    |name| (has_shell && name == key).then(|| "shell".into()),
                    cli,
                );
                let expected = if has_cli && key == "QUILLAI_AI_API_KEY" {
                    if has_shell {
                        Some("shell")
                    } else if has_file {
                        Some("file")
                    } else {
                        None
                    }
                } else {
                    expected
                };
                assert_eq!(
                    result,
                    expected
                        .map(|value| vec![(key, value.into())])
                        .unwrap_or_default(),
                    "{key}"
                );
            }
        }
    }

    #[test]
    fn empty_shell_value_overrides_file_without_mutating_environment() {
        let file = parse("QUILLAI_AI_API_KEY=file").unwrap();
        assert_eq!(
            resolve(
                &file,
                |key| (key == KEYS[0]).then(OsString::new),
                CliSettings::default()
            ),
            vec![(KEYS[0], OsString::new())]
        );
    }
}
