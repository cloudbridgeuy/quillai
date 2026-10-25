use std::path::Path;

/// Arguments that choose the API database for `cargo xtask dev`.
///
/// `configured` is the xtask `--db-url` flag, or `QUILLAI_API_DB_URL` when the
/// flag was omitted. That value is forwarded unchanged so it still wins.
/// When neither is set, the API is given an absolute URL for
/// `<workspace>/quillai.db`, including `mode=rwc` so the file is created the
/// same way as the API's own default.
pub(crate) fn api_database_args(workspace_root: &Path, configured: Option<&str>) -> Vec<String> {
    let url = match configured {
        Some(url) => url.to_string(),
        None => format!(
            "sqlite:{}?mode=rwc",
            workspace_root.join("quillai.db").display()
        ),
    };
    vec!["--db-url".to_string(), url]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "quillai-dev-db-decision-{}-never-created",
            std::process::id()
        ))
    }

    #[test]
    fn default_url_is_the_workspace_file_and_creates_nothing() {
        let root = root();
        let database = root.join("quillai.db");
        let args = api_database_args(&root, None);

        assert_eq!(
            args,
            vec![
                "--db-url".to_string(),
                format!("sqlite:{}?mode=rwc", database.display()),
            ]
        );
        assert!(database.is_absolute());
        assert!(!database.exists());
        assert!(!root.exists());
    }

    #[test]
    fn explicit_url_wins_over_the_workspace_default() {
        let root = root();
        let explicit = "sqlite:elsewhere.db?mode=rwc";
        assert_eq!(
            api_database_args(&root, Some(explicit)),
            vec!["--db-url".to_string(), explicit.to_string()]
        );
        assert!(!root.join("quillai.db").exists());
    }
}
