use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::Command;

type Counts = BTreeMap<String, usize>;
const WAIVERS: &str = "xtask/i18n-waivers.json";
const PROPS: &str = "aria-label title placeholder label alt description heading";

fn included(path: &str) -> bool {
    path.starts_with("app/src/")
        && path.ends_with(".tsx")
        && ![".test.tsx", ".fixture.tsx"]
            .iter()
            .any(|suffix| path.ends_with(suffix))
}

fn scan(path: &str, source: &str) -> Vec<usize> {
    let mut scanner = Scanner {
        source,
        pos: 0,
        found: Vec::new(),
    };
    if included(path) {
        scanner.code(false);
    }
    scanner.found
}

struct Scanner<'a> {
    source: &'a str,
    pos: usize,
    found: Vec<usize>,
}

impl Scanner<'_> {
    fn byte(&self) -> u8 {
        self.source.as_bytes().get(self.pos).copied().unwrap_or(0)
    }

    fn space(&mut self) {
        while self.byte().is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn quoted(&mut self) -> bool {
        let quote = self.byte() as char;
        self.pos += 1;
        let mut letters = false;
        while let Some(ch) = self.source[self.pos..].chars().next() {
            self.pos += ch.len_utf8();
            if ch == '\\' {
                self.pos += self.source[self.pos..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8);
            } else if ch == quote {
                break;
            } else if quote == '`' && ch == '$' && self.byte() == b'{' {
                self.pos += 1;
                self.code(true);
            } else {
                letters |= ch.is_alphabetic();
            }
        }
        letters
    }

    fn expression(&mut self, end: u8) -> Vec<usize> {
        let mut found = Vec::new();
        let mut value = Vec::new();
        let mut at_start = true;
        while self.pos < self.source.len() {
            self.space();
            match self.byte() {
                byte if byte == end => {
                    self.pos += 1;
                    break;
                }
                b'\'' | b'"' | b'`' => {
                    let start = self.pos + 1;
                    if self.quoted() && at_start {
                        value.push(start);
                    }
                    at_start = false;
                }
                b'(' | b'[' => {
                    let grouping = self.byte() == b'(' && at_start;
                    let close = if self.byte() == b'(' { b')' } else { b']' };
                    self.pos += 1;
                    let nested = self.expression(close);
                    if grouping {
                        value = nested;
                    }
                    at_start = false;
                }
                b'?' | b':' => {
                    if self.byte() == b':' {
                        found.append(&mut value);
                    }
                    value.clear();
                    at_start = true;
                    self.pos += 1;
                }
                b'/' if self.source[self.pos..].starts_with("//") => self.through("\n"),
                b'/' if self.source[self.pos..].starts_with("/*") => self.through("*/"),
                _ => {
                    value.clear();
                    at_start = false;
                    if self.byte() == b'{' {
                        self.pos += 1;
                        self.code(true);
                    } else if self.byte() == b'<'
                        && self
                            .source
                            .as_bytes()
                            .get(self.pos + 1)
                            .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'>')
                    {
                        self.jsx();
                    } else {
                        self.pos += 1;
                    }
                }
            }
        }
        found.extend(value);
        found
    }

    fn record(&mut self, start: usize, end: usize) {
        if self.source[start..end].chars().any(char::is_alphabetic) {
            self.found.push(start);
        }
    }

    fn code(&mut self, nested: bool) {
        while self.pos < self.source.len() {
            match self.byte() {
                b'\'' | b'"' | b'`' => {
                    self.quoted();
                }
                b'/' if self.source[self.pos..].starts_with("//") => {
                    self.through("\n");
                }
                b'/' if self.source[self.pos..].starts_with("/*") => {
                    self.through("*/");
                }
                b'{' => {
                    self.pos += 1;
                    self.code(true);
                }
                b'}' if nested => {
                    self.pos += 1;
                    return;
                }
                b'<' if self
                    .source
                    .as_bytes()
                    .get(self.pos + 1)
                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'>') =>
                {
                    self.jsx()
                }
                _ => self.pos += 1,
            }
        }
    }

    fn through(&mut self, delimiter: &str) {
        self.pos += self.source[self.pos..]
            .find(delimiter)
            .map_or(self.source.len() - self.pos, |i| i + delimiter.len());
    }

    fn jsx(&mut self) {
        self.pos += 1;
        let name_start = self.pos;
        while self.byte().is_ascii_alphanumeric()
            || matches!(self.byte(), b'_' | b'-' | b'.' | b':')
        {
            self.pos += 1;
        }
        let name = &self.source[name_start..self.pos];
        let closed = self.source[self.pos..].contains(&format!("</{name}"));
        while self.pos < self.source.len() && self.byte() != b'>' {
            self.space();
            let start = self.pos;
            while self.byte().is_ascii_alphanumeric() || matches!(self.byte(), b'_' | b'-' | b':') {
                self.pos += 1;
            }
            let prop = &self.source[start..self.pos];
            let checked = PROPS.split_whitespace().any(|name| name == prop);
            self.space();
            if !prop.is_empty() && self.byte() != b'=' {
                continue;
            }
            if self.byte() == b'=' {
                self.pos += 1;
                self.space();
            }
            let value = self.pos;
            match self.byte() {
                b'\'' | b'"' => {
                    self.quoted();
                    if checked {
                        self.record(value + 1, self.pos.saturating_sub(1));
                    }
                }
                b'{' => {
                    self.pos += 1;
                    if checked {
                        let literals = self.expression(b'}');
                        self.found.extend(literals);
                    } else {
                        self.code(true);
                    }
                }
                b'>' => break,
                _ => {
                    let next = self.source[self.pos..].chars().next();
                    self.pos += next.map_or(0, char::len_utf8);
                }
            }
        }
        let self_closing = self.pos > 0 && self.source.as_bytes().get(self.pos - 1) == Some(&b'/');
        self.pos = (self.pos + 1).min(self.source.len());
        if self_closing || !closed {
            return;
        }
        while self.pos < self.source.len() {
            match self.byte() {
                b'<' if self.source[self.pos..].starts_with("</") => {
                    self.through(">");
                    return;
                }
                b'<' => self.jsx(),
                b'{' => {
                    self.pos += 1;
                    self.code(true);
                }
                _ => {
                    let start = self.pos;
                    while self.pos < self.source.len() && !matches!(self.byte(), b'<' | b'{') {
                        self.pos += 1;
                    }
                    self.record(start, self.pos);
                }
            }
        }
    }
}

fn compare(found: &Counts, allowed: &Counts) -> Vec<String> {
    let keys: std::collections::BTreeSet<_> = found.keys().chain(allowed.keys()).collect();
    keys.into_iter().filter_map(|path| {
        let count = found.get(path).copied().unwrap_or(0);
        let limit = allowed.get(path).copied().unwrap_or(0);
        if count > limit {
            Some(format!("{path}: {count} hard-coded UI strings; {WAIVERS} allows {limit}; use the i18n message helper"))
        } else if count < limit {
            let action = if count == 0 { "remove the entry".into() } else { format!("lower the count to {count}") };
            Some(format!("{path}: {count} hard-coded UI strings; {WAIVERS} allows {limit}; {action}"))
        } else { None }
    }).collect()
}

pub fn run(root: &Path, log_path: &Path) -> std::io::Result<bool> {
    let mut log = std::fs::File::create(log_path)?;
    let output = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "app/src",
        ])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        log.write_all(&output.stderr)?;
        return Ok(false);
    }
    let mut found = Counts::new();
    for file in output.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let file = std::str::from_utf8(file).map_err(std::io::Error::other)?;
        if included(file) {
            let source = std::fs::read_to_string(root.join(file))?;
            let count = scan(file, &source).len();
            if count > 0 {
                found.insert(file.to_owned(), count);
            }
        }
    }
    let allowed: Counts = serde_json::from_str(&std::fs::read_to_string(root.join(WAIVERS))?)
        .map_err(std::io::Error::other)?;
    if allowed
        .iter()
        .any(|(path, count)| !included(path) || *count == 0)
    {
        writeln!(
            log,
            "{WAIVERS}: expected app/src TSX paths and positive counts"
        )?;
        return Ok(false);
    }
    let problems = compare(&found, &allowed);
    for problem in &problems {
        writeln!(log, "{problem}")?;
    }
    Ok(problems.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(source: &str) -> usize {
        scan("app/src/Example.tsx", source).len()
    }

    #[test]
    fn jsx_text_flagged() {
        assert_eq!(count("const view = <p>Hello <b>世界</b>!</p>;"), 2);
    }

    #[test]
    fn whitespace_punctuation_and_expressions_ignored() {
        assert_eq!(count("<p> \n… ! 123 {name}{'literal'}</p>"), 0);
    }

    #[test]
    fn target_prop_literals_flagged() {
        assert_eq!(
            count(
                r#"<Input disabled aria-label="Name" title='Help' placeholder="Type" label="Email" />"#
            ),
            4
        );
        assert_eq!(
            count(r#"<Input title={"Help"} label={'Name'} title="123…" data-label="Skip" />"#),
            2
        );
    }

    #[test]
    fn helper_calls_and_other_props_ignored() {
        assert_eq!(
            count(r#"<I title={formatMessage("en", "key")} label={t("key")} className="x"/>"#),
            0
        );
    }

    #[test]
    fn conditional_prop_branches_flagged_once() {
        for prop in PROPS.split_whitespace() {
            let source = format!(r#"<I {prop}={{a ? "A" : b ? 'B' : "世界"}}/>"#);
            assert_eq!(count(&source), 3, "{prop}");
        }
    }

    #[test]
    fn template_prop_static_letters_flagged_once() {
        let source = r#"<I title={`Delete ${name} now`} label={a ? `Hello ${name}` : `世界`}/>"#;
        assert_eq!(count(source), 3);
        let source = r#"<I title={` ${name} — ${t("key")} 123!`} label={`…`}/>"#;
        assert_eq!(count(source), 0);
    }

    #[test]
    fn additional_prop_literals_flagged() {
        for prop in ["alt", "description", "heading"] {
            let source = format!(r#"<I {prop}="Name"/><I {prop}={{'Name'}}/>"#);
            assert_eq!(count(&source), 2, "{prop}");
        }
    }

    #[test]
    fn prop_calls_comparisons_and_unchecked_props_ignored() {
        let source = r#"<I title={t("lore.title")} label={msg.foo} alt={kind === "script" ? a : b} description={"script" === kind ? a : b} heading={a ? t("key") : format("x", "y")} className={a ? "A" : "B"} data-title={`Hello ${name}`} title={(kind < "script") ? a : b}/>"#;
        assert_eq!(count(source), 0);
        let source =
            r#"<I title={kind === "script" ? "Script" : "Other"} label={f(a ? "x" : "y")}/>"#;
        assert_eq!(count(source), 2);
    }

    #[test]
    fn test_and_fixture_files_skipped() {
        for path in [
            "app/src/a.test.tsx",
            "app/src/a.dom.test.tsx",
            "app/src/a.fixture.tsx",
            "app/src/a.ts",
            "other/a.tsx",
        ] {
            assert!(scan(path, "<p>Hello</p>").is_empty(), "{path}");
        }
        assert_eq!(scan("app/src/ui/Label.tsx", "<p>Hello</p>").len(), 1);
    }

    #[test]
    fn nested_jsx_and_quoted_delimiters() {
        assert_eq!(
            count(
                r#"<><p>{items.map(x => <b title="A > B">Hello</b>)}</p><Input title="{Name}" /></>"#
            ),
            3
        );
    }

    #[test]
    fn code_strings_and_comments_ignored() {
        assert_eq!(count("const fake = '<p>Hello</p>'; /* <p>Skip</p> */\n// <p>Skip</p>\nconst x = <p>Real</p>;"), 1);
    }

    #[test]
    fn type_generics_and_multiline_text() {
        assert_eq!(
            count("const r = useRef<HTMLDivElement | null>(null);\nconst e: ReactElement<ComponentProps<\"button\">> = x;"),
            0
        );
        assert_eq!(count("<p>\n  Hello\n  world\n</p>"), 1);
    }

    fn counts(count: usize) -> Counts {
        BTreeMap::from([("app/src/A.tsx".into(), count)])
    }

    #[test]
    fn new_file_failure() {
        assert_eq!(
            compare(&counts(1), &Counts::new()),
            ["app/src/A.tsx: 1 hard-coded UI strings; xtask/i18n-waivers.json allows 0; use the i18n message helper"]
        );
    }

    #[test]
    fn count_increase_failure() {
        assert!(compare(&counts(2), &counts(1))[0].contains("allows 1"));
    }

    #[test]
    fn count_decrease_prompt() {
        assert!(compare(&counts(1), &counts(2))[0].contains("lower the count to 1"));
        assert!(compare(&Counts::new(), &counts(1))[0].contains("remove the entry"));
        assert!(compare(&counts(2), &counts(2)).is_empty());
    }
}
