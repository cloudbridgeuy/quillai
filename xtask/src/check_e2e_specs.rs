use std::io::Write;
use std::path::Path;

fn listed_specs(config: &str) -> Option<Vec<&str>> {
    let (_, after_key) = config.split_once("testMatch:")?;
    let (_, after_open) = after_key.split_once('[')?;
    let (entries, _) = after_open.split_once(']')?;
    Some(entries.split('"').skip(1).step_by(2).collect())
}

fn problems(spec_files: &[String], config: &str) -> Vec<String> {
    let Some(listed) = listed_specs(config) else {
        return vec!["testMatch array not found in e2e/playwright.config.ts".to_string()];
    };
    let missing = spec_files
        .iter()
        .filter(|file| !listed.contains(&file.as_str()))
        .map(|file| format!("{file}: not listed in testMatch"));
    let stale = listed
        .iter()
        .filter(|entry| !spec_files.iter().any(|file| file == *entry))
        .map(|entry| format!("{entry}: listed in testMatch but the file does not exist"));
    missing.chain(stale).collect()
}

pub fn run(root: &Path, log_path: &Path) -> std::io::Result<bool> {
    let mut log = std::fs::File::create(log_path)?;
    let e2e = root.join("e2e");
    let config = std::fs::read_to_string(e2e.join("playwright.config.ts"))?;
    let mut spec_files = Vec::new();
    for entry in std::fs::read_dir(e2e)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.ends_with(".spec.ts") {
            spec_files.push(name);
        }
    }
    spec_files.sort();
    let found = problems(&spec_files, &config);
    for problem in &found {
        writeln!(log, "{problem}")?;
    }
    Ok(found.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"export default defineConfig({
  testDir: ".",
  testMatch: ["a.spec.ts", "b.spec.ts"],
  outputDir: "../target",
});"#;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn reports_spec_missing_from_test_match() {
        assert_eq!(
            problems(&names(&["a.spec.ts", "b.spec.ts", "c.spec.ts"]), CONFIG),
            ["c.spec.ts: not listed in testMatch"]
        );
    }

    #[test]
    fn reports_stale_test_match_entry() {
        assert_eq!(
            problems(&names(&["a.spec.ts"]), CONFIG),
            ["b.spec.ts: listed in testMatch but the file does not exist"]
        );
    }

    #[test]
    fn passes_when_every_spec_is_listed() {
        assert_eq!(
            problems(&names(&["b.spec.ts", "a.spec.ts"]), CONFIG),
            Vec::<String>::new()
        );
    }

    #[test]
    fn reports_config_without_test_match() {
        assert_eq!(
            problems(&names(&["a.spec.ts"]), "testDir: \".\""),
            ["testMatch array not found in e2e/playwright.config.ts"]
        );
    }
}
