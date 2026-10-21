use std::{path::PathBuf, process::Command};

struct CatalogFile(PathBuf);

impl CatalogFile {
    fn new(contents: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "quillai-catalog-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, contents).unwrap();
        Self(path)
    }
}

impl Drop for CatalogFile {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).unwrap();
    }
}

#[test]
fn bad_catalog_fails_startup_before_database_connection() {
    let file = CatalogFile::new("{bad json");
    let output = Command::new(env!("CARGO_BIN_EXE_api"))
        .env("QUILLAI_AI_MODELS_FILE", &file.0)
        .env_remove("QUILLAI_AI_MODEL")
        .arg("--db-url")
        .arg("invalid-database-url")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("invalid model catalog JSON"), "{error}");
    assert!(error.contains(file.0.to_str().unwrap()), "{error}");
}

#[test]
fn explicit_missing_catalog_does_not_silently_use_legacy_model() {
    let file = CatalogFile::new("");
    let missing = file.0.with_extension("missing");
    let output = Command::new(env!("CARGO_BIN_EXE_api"))
        .env("QUILLAI_AI_MODEL", "legacy-model")
        .env("QUILLAI_AI_MODELS_FILE", &file.0)
        .arg("--ai-models")
        .arg(&missing)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("cannot read model catalog"), "{error}");
    assert!(error.contains(missing.to_str().unwrap()), "{error}");
}
