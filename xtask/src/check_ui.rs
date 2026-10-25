use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::Command;

const ALLOWLIST_PATH: &str = "app/src/ui/ALLOWLIST";
const UI_DIR: &str = "app/src/ui/";
const TOKENS_PATH: &str = "app/src/ui/tokens.css";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    RawButton,
    RawInput,
    RawTextarea,
    RawSelect,
    TailwindBracket,
    HexColour,
    FontSize,
}

impl Rule {
    const ALL: [Rule; 7] = [
        Rule::RawButton,
        Rule::RawInput,
        Rule::RawTextarea,
        Rule::RawSelect,
        Rule::TailwindBracket,
        Rule::HexColour,
        Rule::FontSize,
    ];

    fn name(self) -> &'static str {
        match self {
            Rule::RawButton => "raw-button",
            Rule::RawInput => "raw-input",
            Rule::RawTextarea => "raw-textarea",
            Rule::RawSelect => "raw-select",
            Rule::TailwindBracket => "tailwind-bracket",
            Rule::HexColour => "hex-colour",
            Rule::FontSize => "font-size",
        }
    }

    fn parse(name: &str) -> Option<Rule> {
        Rule::ALL.into_iter().find(|rule| rule.name() == name)
    }

    fn fix_hint(self) -> &'static str {
        match self {
            Rule::RawButton | Rule::RawInput | Rule::RawTextarea | Rule::RawSelect => {
                "use an app/src/ui/ primitive"
            }
            Rule::TailwindBracket => "use a token utility from app/src/ui/tokens.css",
            Rule::HexColour | Rule::FontSize => "use a token from app/src/ui/tokens.css",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Tsx,
    Css,
}

pub fn file_kind(path: &str) -> Option<FileKind> {
    if path.ends_with(".tsx") && !path.starts_with(UI_DIR) {
        Some(FileKind::Tsx)
    } else if path.ends_with(".css") && path != TOKENS_PATH {
        Some(FileKind::Css)
    } else {
        None
    }
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

fn count_tags(text: &str, tag: &str) -> usize {
    let pattern = format!("<{tag}");
    text.match_indices(&pattern)
        .filter(|(index, _)| {
            text.as_bytes()
                .get(index + pattern.len())
                .is_none_or(|next| !is_identifier_byte(*next) && *next != b'.')
        })
        .count()
}

fn count_tailwind_brackets(text: &str) -> usize {
    text.match_indices("-[")
        .filter(|(index, _)| {
            index.checked_sub(1).is_some_and(|before| {
                let byte = text.as_bytes()[before];
                byte.is_ascii_lowercase() || byte.is_ascii_digit()
            })
        })
        .count()
}

fn count_hex_colours(text: &str) -> usize {
    let bytes = text.as_bytes();
    text.match_indices('#')
        .filter(|(index, _)| {
            let digits = bytes[index + 1..]
                .iter()
                .take_while(|byte| byte.is_ascii_hexdigit())
                .count();
            let after = bytes.get(index + 1 + digits);
            matches!(digits, 3 | 4 | 6 | 8) && after.is_none_or(|byte| !is_identifier_byte(*byte))
        })
        .count()
}

fn count_font_sizes(text: &str) -> usize {
    let bytes = text.as_bytes();
    text.match_indices("font-size")
        .filter(|(index, matched)| {
            let standalone = index
                .checked_sub(1)
                .is_none_or(|before| !is_identifier_byte(bytes[before]));
            standalone && text[index + matched.len()..].trim_start().starts_with(':')
        })
        .count()
}

pub fn scan(kind: FileKind, text: &str) -> BTreeMap<Rule, usize> {
    let counts = match kind {
        FileKind::Tsx => vec![
            (Rule::RawButton, count_tags(text, "button")),
            (Rule::RawInput, count_tags(text, "input")),
            (Rule::RawTextarea, count_tags(text, "textarea")),
            (Rule::RawSelect, count_tags(text, "select")),
            (Rule::TailwindBracket, count_tailwind_brackets(text)),
        ],
        FileKind::Css => vec![
            (Rule::HexColour, count_hex_colours(text)),
            (Rule::FontSize, count_font_sizes(text)),
        ],
    };
    counts.into_iter().filter(|(_, count)| *count > 0).collect()
}

pub type Counts = BTreeMap<(String, Rule), usize>;

pub fn parse_allowlist(text: &str) -> Result<Counts, Vec<String>> {
    let mut allowed = Counts::new();
    let mut errors = Vec::new();
    for (number, line) in text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line))
    {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.is_empty() {
            continue;
        }
        let parsed = match fields.as_slice() {
            [path, count, rule] => match (count.parse::<usize>(), Rule::parse(rule)) {
                (Ok(0), _) => Err("count must be above 0; remove the line instead".to_string()),
                (Ok(count), Some(rule)) => Ok(((path.to_string(), rule), count)),
                (Err(_), _) => Err(format!("count `{count}` is not a whole number")),
                (_, None) => Err(format!("unknown rule `{rule}`")),
            },
            _ => Err("expected `path count rule`".to_string()),
        };
        match parsed {
            Ok((key, count)) => {
                if allowed.insert(key, count).is_some() {
                    errors.push(format!("{ALLOWLIST_PATH}:{number}: duplicate entry"));
                }
            }
            Err(error) => errors.push(format!("{ALLOWLIST_PATH}:{number}: {error}")),
        }
    }
    if errors.is_empty() {
        Ok(allowed)
    } else {
        Err(errors)
    }
}

pub fn compare(found: &Counts, allowed: &Counts) -> Vec<String> {
    let mut keys: Vec<&(String, Rule)> = found.keys().chain(allowed.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter_map(|key @ (path, rule)| {
            let found = found.get(key).copied().unwrap_or(0);
            let allowed = allowed.get(key).copied().unwrap_or(0);
            let rule_name = rule.name();
            if found > allowed {
                Some(format!(
                    "{path}: {found} {rule_name}, {ALLOWLIST_PATH} allows {allowed}; {}, see docs/design-system.md",
                    rule.fix_hint()
                ))
            } else if found < allowed && found == 0 {
                Some(format!(
                    "{path}: 0 {rule_name}, {ALLOWLIST_PATH} allows {allowed}; remove the line `{path} {allowed} {rule_name}`"
                ))
            } else if found < allowed {
                Some(format!(
                    "{path}: {found} {rule_name}, {ALLOWLIST_PATH} allows {allowed}; lower the count to `{path} {found} {rule_name}`"
                ))
            } else {
                None
            }
        })
        .collect()
}

pub fn run(root: &Path, log_path: &Path) -> std::io::Result<bool> {
    let mut log = std::fs::File::create(log_path)?;
    let output = Command::new("git")
        .args(["ls-files", "-z", "--", "app/src"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        log.write_all(&output.stderr)?;
        return Ok(false);
    }
    let mut found = Counts::new();
    let mut problems = Vec::new();
    for file in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|s| !s.is_empty())
    {
        let file = std::str::from_utf8(file)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        let Some(kind) = file_kind(file) else {
            continue;
        };
        match std::fs::read_to_string(root.join(file)) {
            Ok(text) => found.extend(
                scan(kind, &text)
                    .into_iter()
                    .map(|(rule, count)| ((file.to_string(), rule), count)),
            ),
            Err(error) => problems.push(format!("{file}: {error}")),
        }
    }
    match std::fs::read_to_string(root.join(ALLOWLIST_PATH)) {
        Ok(text) => match parse_allowlist(&text) {
            Ok(allowed) => problems.extend(compare(&found, &allowed)),
            Err(errors) => problems.extend(errors),
        },
        Err(error) => problems.push(format!("{ALLOWLIST_PATH}: {error}")),
    }
    for problem in &problems {
        writeln!(log, "{problem}")?;
    }
    Ok(problems.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(entries: &[(&str, Rule, usize)]) -> Counts {
        entries
            .iter()
            .map(|(path, rule, count)| ((path.to_string(), *rule), *count))
            .collect()
    }

    #[test]
    fn file_kind_skips_ui_directory_and_tokens() {
        assert_eq!(file_kind("app/src/App.tsx"), Some(FileKind::Tsx));
        assert_eq!(file_kind("app/src/ui/Button.tsx"), None);
        assert_eq!(file_kind("app/src/index.css"), Some(FileKind::Css));
        assert_eq!(file_kind("app/src/ui/tokens.css"), None);
        assert_eq!(file_kind("app/src/ui/other.css"), Some(FileKind::Css));
        assert_eq!(file_kind("app/src/api_config.ts"), None);
    }

    #[test]
    fn tsx_counts_raw_controls_and_brackets() {
        let text = r#"<button type="button"><input/><textarea
            rows={2}></textarea><select>{x}</select><div className="w-[262px] hover:bg-[#fff]"/>"#;
        assert_eq!(
            scan(FileKind::Tsx, text),
            BTreeMap::from([
                (Rule::RawButton, 1),
                (Rule::RawInput, 1),
                (Rule::RawTextarea, 1),
                (Rule::RawSelect, 1),
                (Rule::TailwindBracket, 2),
            ])
        );
    }

    #[test]
    fn tsx_ignores_components_and_plain_indexing() {
        let text =
            "<Button/><ButtonGroup/><inputs/><selection/><select.Item/> items[0] a - [1] </button>";
        assert!(scan(FileKind::Tsx, text).is_empty());
    }

    #[test]
    fn css_counts_hex_colours_and_font_sizes() {
        let text = ".a { color: #fff; background: #D2E2EC; border-color: #1f29330f; font-size: 12px; }\n.b { font-size : 1rem; outline: #abcd; }";
        assert_eq!(
            scan(FileKind::Css, text),
            BTreeMap::from([(Rule::HexColour, 4), (Rule::FontSize, 2)])
        );
    }

    #[test]
    fn css_ignores_ids_tokens_and_shorthand() {
        let text = "#root { --text-font-size: 1px; } #e2e-x { color: var(--ink); font: 500 11px/1.3 var(--sans); } #12345 {}";
        assert!(scan(FileKind::Css, text).is_empty());
    }

    #[test]
    fn allowlist_parses_entries_and_rejects_bad_lines() {
        assert_eq!(
            parse_allowlist("app/src/a.tsx 2 raw-button\n\napp/src/index.css 3 hex-colour\n"),
            Ok(counts(&[
                ("app/src/a.tsx", Rule::RawButton, 2),
                ("app/src/index.css", Rule::HexColour, 3),
            ]))
        );
        let errors = parse_allowlist(
            "app/src/a.tsx:4 raw-button\napp/src/a.tsx 0 raw-button\napp/src/a.tsx x raw-button\napp/src/a.tsx 1 raw-div\napp/src/b.tsx 1 raw-input\napp/src/b.tsx 2 raw-input\n",
        )
        .unwrap_err();
        assert_eq!(errors.len(), 5);
        assert!(errors[0].ends_with(":1: expected `path count rule`"));
        assert!(errors[1].contains(":2: count must be above 0"));
        assert!(errors[2].contains(":3: count `x`"));
        assert!(errors[3].contains(":4: unknown rule `raw-div`"));
        assert!(errors[4].ends_with(":6: duplicate entry"));
    }

    #[test]
    fn compare_passes_when_counts_match() {
        let found = counts(&[("app/src/a.tsx", Rule::RawButton, 2)]);
        assert!(compare(&found, &found.clone()).is_empty());
    }

    #[test]
    fn compare_fails_new_and_excess_violations() {
        let found = counts(&[
            ("app/src/a.tsx", Rule::RawButton, 3),
            ("app/src/new.tsx", Rule::RawInput, 1),
        ]);
        let allowed = counts(&[("app/src/a.tsx", Rule::RawButton, 2)]);
        assert_eq!(
            compare(&found, &allowed),
            vec![
                "app/src/a.tsx: 3 raw-button, app/src/ui/ALLOWLIST allows 2; use an app/src/ui/ primitive, see docs/design-system.md",
                "app/src/new.tsx: 1 raw-input, app/src/ui/ALLOWLIST allows 0; use an app/src/ui/ primitive, see docs/design-system.md",
            ]
        );
    }

    #[test]
    fn compare_ratchets_counts_down() {
        let found = counts(&[("app/src/a.tsx", Rule::RawButton, 1)]);
        let allowed = counts(&[
            ("app/src/a.tsx", Rule::RawButton, 2),
            ("app/src/gone.tsx", Rule::RawSelect, 1),
        ]);
        assert_eq!(
            compare(&found, &allowed),
            vec![
                "app/src/a.tsx: 1 raw-button, app/src/ui/ALLOWLIST allows 2; lower the count to `app/src/a.tsx 1 raw-button`",
                "app/src/gone.tsx: 0 raw-select, app/src/ui/ALLOWLIST allows 1; remove the line `app/src/gone.tsx 1 raw-select`",
            ]
        );
    }
}
