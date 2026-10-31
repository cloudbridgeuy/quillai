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
    LinkButton,
    InlineStyle,
    ColourFn,
    FontShorthand,
    RawRadius,
    RawShadow,
    RawZIndex,
    RawLetterSpacing,
    RawSpacing,
    ClickNonInteractive,
}

impl Rule {
    fn name(self) -> &'static str {
        match self {
            Rule::RawButton => "raw-button",
            Rule::RawInput => "raw-input",
            Rule::RawTextarea => "raw-textarea",
            Rule::RawSelect => "raw-select",
            Rule::TailwindBracket => "tailwind-bracket",
            Rule::HexColour => "hex-colour",
            Rule::FontSize => "font-size",
            Rule::LinkButton => "link-button",
            Rule::InlineStyle => "inline-style",
            Rule::ColourFn => "colour-fn",
            Rule::FontShorthand => "font-shorthand",
            Rule::RawRadius => "raw-radius",
            Rule::RawShadow => "raw-shadow",
            Rule::RawZIndex => "raw-z-index",
            Rule::RawLetterSpacing => "raw-letter-spacing",
            Rule::RawSpacing => "raw-spacing",
            Rule::ClickNonInteractive => "click-non-interactive",
        }
    }

    fn fix_hint(self) -> &'static str {
        match self {
            Rule::RawButton | Rule::RawInput | Rule::RawTextarea | Rule::RawSelect => {
                "use an app/src/ui/ primitive"
            }
            Rule::TailwindBracket => "use a token utility from app/src/ui/tokens.css",
            Rule::LinkButton | Rule::ClickNonInteractive => "use an app/src/ui/ primitive",
            _ => "use a token from app/src/ui/tokens.css",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Tsx,
    Css,
    Ts,
}

pub fn file_kind(path: &str) -> Option<FileKind> {
    if path
        .rsplit('/')
        .next()
        .is_some_and(|name| name.contains(".test."))
    {
        None
    } else if path.ends_with(".tsx") && !path.starts_with(UI_DIR) {
        Some(FileKind::Tsx)
    } else if path.ends_with(".css") && path != TOKENS_PATH {
        Some(FileKind::Css)
    } else if path.ends_with(".ts") && !path.starts_with(UI_DIR) {
        Some(FileKind::Ts)
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

fn opening_tags(text: &str) -> Vec<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut tags = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'<' || !bytes.get(index + 1).is_some_and(u8::is_ascii_alphabetic) {
            index += 1;
            continue;
        }
        let start = index + 1;
        index = start;
        while bytes
            .get(index)
            .is_some_and(|b| is_identifier_byte(*b) || *b == b'.')
        {
            index += 1;
        }
        let name = &text[start..index];
        let attributes = index;
        let mut depth = 0usize;
        let mut quote = 0u8;
        while index < bytes.len() {
            let byte = bytes[index];
            if quote != 0 {
                if byte == b'\\' {
                    index += 1;
                } else if byte == quote {
                    quote = 0;
                }
            } else {
                match byte {
                    b'\'' | b'"' | b'`' => quote = byte,
                    b'{' => depth += 1,
                    b'}' => depth = depth.saturating_sub(1),
                    b'>' if depth == 0 => break,
                    _ => {}
                }
            }
            index += 1;
        }
        if index < bytes.len() {
            tags.push((name, &text[attributes..index]));
        }
        index += 1;
    }
    tags
}

fn attributes(text: &str) -> Vec<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let start = i;
        while bytes.get(i).is_some_and(|b| is_identifier_byte(*b)) {
            i += 1;
        }
        let name = &text[start..i];
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if bytes.get(i) != Some(&b'=') {
            continue;
        }
        i += 1;
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        let start = i;
        let mut depth = 0usize;
        let mut quote = 0u8;
        while i < bytes.len() {
            let b = bytes[i];
            if quote != 0 {
                if b == b'\\' {
                    i += 1;
                } else if b == quote {
                    quote = 0;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
            } else {
                match b {
                    b'\'' | b'"' | b'`' => quote = b,
                    b'{' => depth += 1,
                    b'}' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ if depth == 0 && (b.is_ascii_whitespace() || b == b'/') => break,
                    _ => {}
                }
            }
            i += 1;
        }
        result.push((name, &text[start..i.min(bytes.len())]));
    }
    result
}

fn numeric_unit(value: &str, units: &[&str]) -> bool {
    units.iter().any(|unit| {
        value.match_indices(unit).any(|(i, _)| {
            i > 0
                && value.as_bytes()[i - 1].is_ascii_digit()
                && value
                    .as_bytes()
                    .get(i + unit.len())
                    .is_none_or(|b| !is_identifier_byte(*b))
        })
    })
}

fn css_counts(text: &str) -> Vec<(Rule, usize)> {
    let mut counts = BTreeMap::new();
    for declaration in text.split([';', '{', '}']) {
        let Some((property, value)) = declaration.rsplit_once(':') else {
            continue;
        };
        let property = property.trim();
        let value = value.trim();
        let value = value.strip_suffix("!important").unwrap_or(value).trim();
        let variable = value.starts_with("var(") && {
            let mut depth = 0usize;
            value.bytes().enumerate().skip(3).all(|(i, byte)| {
                if byte == b'(' {
                    depth += 1;
                }
                if byte == b')' {
                    depth = depth.saturating_sub(1);
                }
                depth > 0 || i + 1 == value.len()
            }) && depth == 0
        };
        let raw = !variable && !matches!(value, "0" | "none" | "normal");
        let rule = if property == "font" && numeric_unit(value, &["px", "rem", "em", "%"]) {
            Some(Rule::FontShorthand)
        } else if (property == "border-radius"
            || (property.starts_with("border-") && property.ends_with("-radius")))
            && raw
        {
            Some(Rule::RawRadius)
        } else if property == "box-shadow" && raw {
            Some(Rule::RawShadow)
        } else if property == "z-index" && raw {
            Some(Rule::RawZIndex)
        } else if property == "letter-spacing" && raw {
            Some(Rule::RawLetterSpacing)
        } else if ["padding", "margin", "gap", "row-gap", "column-gap"]
            .iter()
            .any(|p| property == *p || property.strip_prefix(p).is_some_and(|s| s.starts_with('-')))
            && numeric_unit(value, &["px", "rem", "em"])
        {
            Some(Rule::RawSpacing)
        } else {
            None
        };
        if let Some(rule) = rule {
            *counts.entry(rule).or_insert(0) += 1;
        }
    }
    counts.into_iter().collect()
}

fn count_colour_functions(text: &str) -> usize {
    ["rgb(", "rgba(", "hsl(", "hsla(", "color-mix("]
        .iter()
        .map(|pattern| {
            text.match_indices(pattern)
                .filter(|(i, _)| {
                    i.checked_sub(1)
                        .is_none_or(|p| !is_identifier_byte(text.as_bytes()[p]))
                })
                .count()
        })
        .sum()
}

fn scan_path(path: &str, kind: FileKind, text: &str) -> BTreeMap<Rule, usize> {
    let exempt_hex = if kind == FileKind::Css {
        let mut parts = text.split('{');
        let mut header = parts.next().unwrap_or_default();
        let mut count = 0;
        for part in parts {
            if header.rsplit('}').next().unwrap_or_default().trim() == "@property --ai-stream-color"
            {
                let body = part.split('}').next().unwrap_or_default();
                for declaration in body.split(';') {
                    if let Some(("initial-value", value)) =
                        declaration.split_once(':').map(|(p, v)| (p.trim(), v))
                    {
                        count += count_hex_colours(value);
                    }
                }
            }
            header = part;
        }
        count
    } else {
        0
    };
    let mut counts = vec![(Rule::HexColour, count_hex_colours(text) - exempt_hex)];
    match kind {
        FileKind::Tsx => {
            counts.extend([
                (Rule::RawButton, count_tags(text, "button")),
                (Rule::RawInput, count_tags(text, "input")),
                (Rule::RawTextarea, count_tags(text, "textarea")),
                (Rule::RawSelect, count_tags(text, "select")),
                (Rule::TailwindBracket, count_tailwind_brackets(text)),
                (Rule::ColourFn, count_colour_functions(text)),
            ]);
            for (tag, text) in opening_tags(text) {
                let attrs = attributes(text);
                for rule in [
                    (
                        Rule::LinkButton,
                        tag == "a"
                            && attrs.iter().any(|(name, value)| {
                                matches!(*name, "class" | "className") && value.contains("button")
                            }),
                    ),
                    (
                        Rule::InlineStyle,
                        attrs
                            .iter()
                            .any(|(name, value)| {
                                *name == "style" && value.starts_with('{')
                                    && !(path == "app/src/md_editor/components/ContinueWindow.tsx"
                                        && matches!(*value,
                                            "{{ top: geometry.top, height: Math.max(0, geometry.bottom - geometry.top) }}"
                                            | "{{ left: Math.max(FLAG_MIN_LEFT, geometry.left - FLAG_OFFSET) }}"))
                            }),
                    ),
                    (
                        Rule::ClickNonInteractive,
                        matches!(tag, "div" | "span" | "li" | "p" | "section")
                            && attrs.iter().any(|(name, _)| *name == "onClick"),
                    ),
                ] {
                    if rule.1 {
                        counts.push((rule.0, 1));
                    }
                }
            }
        }
        FileKind::Css => {
            counts.extend([
                (Rule::FontSize, count_font_sizes(text)),
                (Rule::ColourFn, count_colour_functions(text)),
            ]);
            counts.extend(css_counts(text));
        }
        FileKind::Ts => {}
    }
    let mut result = BTreeMap::new();
    for (rule, count) in counts {
        if count > 0 {
            *result.entry(rule).or_insert(0) += count;
        }
    }
    result
}

#[cfg(test)]
fn scan(kind: FileKind, text: &str) -> BTreeMap<Rule, usize> {
    scan_path("", kind, text)
}

pub type Counts = BTreeMap<(String, Rule), usize>;

pub fn parse_allowlist(text: &str) -> Result<Counts, Vec<String>> {
    let errors: Vec<_> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, _)| {
            format!(
                "{ALLOWLIST_PATH}:{}: remove this entry; the allowlist must be empty",
                index + 1
            )
        })
        .collect();
    if errors.is_empty() {
        Ok(Counts::new())
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
                scan_path(file, kind, &text)
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
        assert_eq!(file_kind("app/src/api_config.ts"), Some(FileKind::Ts));
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
                (Rule::HexColour, 1),
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
        assert_eq!(
            scan(FileKind::Css, text),
            BTreeMap::from([(Rule::FontShorthand, 1)])
        );
    }

    #[test]
    fn new_rules_flag_and_ignore_expected_values() {
        let cases = [
            (
                FileKind::Tsx,
                Rule::LinkButton,
                r#"<a className="editor-button">x</a>"#,
                r#"<a className="link">button</a><div class="button"/>"#,
            ),
            (
                FileKind::Tsx,
                Rule::InlineStyle,
                "<Panel style={{width: 1}}/>",
                "<div data-style={{width: 1}}/>",
            ),
            (
                FileKind::Tsx,
                Rule::ColourFn,
                "rgb(1,2,3) rgba(1,2,3,1) hsl(1,2,3) hsla(1,2,3,1) color-mix(a,b)",
                "myrgb(1)",
            ),
            (
                FileKind::Css,
                Rule::ColourFn,
                "a {color: rgb(1,2,3)}",
                "a {color: var(--ink)}",
            ),
            (
                FileKind::Css,
                Rule::FontShorthand,
                "a {font: 500 11px/1.3 sans}",
                "a {font: inherit}",
            ),
            (
                FileKind::Css,
                Rule::RawRadius,
                "a {border-top-left-radius: var(--r) 2px}",
                "a {border-radius: 0 !important; border-top-right-radius: var(--radius)}",
            ),
            (
                FileKind::Css,
                Rule::RawShadow,
                "a {box-shadow: 0 1px black}",
                "a {box-shadow: none}",
            ),
            (
                FileKind::Css,
                Rule::RawZIndex,
                "a {z-index: 10}",
                "a {z-index: var(--layer)}",
            ),
            (
                FileKind::Css,
                Rule::RawLetterSpacing,
                "a {letter-spacing: .2em}",
                "a {letter-spacing: normal}",
            ),
            (
                FileKind::Css,
                Rule::RawSpacing,
                "a {padding-left: 2px; row-gap: 1rem}",
                "a {margin: 0; gap: var(--gap)}",
            ),
            (
                FileKind::Tsx,
                Rule::ClickNonInteractive,
                "<div title={x => x > 1}\n onClick={go}/>",
                "<button onClick={go}/><div><span onClickCapture={go}/></div>",
            ),
            (
                FileKind::Ts,
                Rule::HexColour,
                "const c = '#fff'",
                "const c = '#xyz'",
            ),
        ];
        for (kind, rule, flagged, ignored) in cases {
            assert!(
                scan(kind, flagged).contains_key(&rule),
                "{rule:?}: {flagged}"
            );
            assert!(
                !scan(kind, ignored).contains_key(&rule),
                "{rule:?}: {ignored}"
            );
        }
        for path in [
            "app/src/a.test.ts",
            "app/src/a.test.tsx",
            "app/src/a.test.css",
        ] {
            assert_eq!(file_kind(path), None);
        }
    }

    #[test]
    fn allowlist_must_be_empty_even_when_counts_match() {
        assert!(parse_allowlist(" \n").is_ok());
        assert!(parse_allowlist("app/src/a.tsx 2 raw-button\n").is_err());
    }

    #[test]
    fn exemptions_are_limited_to_the_registered_colour_and_caret_styles() {
        let css = "@property --ai-stream-color { initial-value: #fff; color: #abc; } @property --other { initial-value: #def; } .x { initial-value: #123; }";
        assert_eq!(scan(FileKind::Css, css).get(&Rule::HexColour), Some(&3));
        let styles = "<div style={{ top: geometry.top, height: Math.max(0, geometry.bottom - geometry.top) }}/><div style={{ left: Math.max(FLAG_MIN_LEFT, geometry.left - FLAG_OFFSET) }}/><div style={{ top: geometry.top }}/ >";
        let path = "app/src/md_editor/components/ContinueWindow.tsx";
        assert_eq!(
            scan_path(path, FileKind::Tsx, styles).get(&Rule::InlineStyle),
            Some(&1)
        );
        assert_eq!(
            scan_path("app/src/other.tsx", FileKind::Tsx, styles).get(&Rule::InlineStyle),
            Some(&3)
        );
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
