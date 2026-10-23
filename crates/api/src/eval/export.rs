use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use super::fixture::{ApiChapter, Fixture, FIXTURE_FILE};

pub async fn export(api: &str, series_id: i64, book_id: i64, out: &Path) -> Result<PathBuf> {
    let out = std::path::absolute(out).context("failed to resolve output directory")?;
    ensure_git_ignored(&out)?;
    let url = format!(
        "{}/series/{series_id}/books/{book_id}/chapters?content=true",
        api.trim_end_matches('/')
    );
    let response = reqwest::get(&url)
        .await
        .with_context(|| format!("GET {url} failed"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("GET {url} returned {status}: {body}");
    }
    let chapters: Vec<ApiChapter> = response
        .json()
        .await
        .with_context(|| format!("GET {url} returned an unexpected body"))?;
    let fixture = Fixture::build(series_id, book_id, chapters);
    std::fs::create_dir_all(&out).with_context(|| format!("failed to create {}", out.display()))?;
    let path = out.join(FIXTURE_FILE);
    std::fs::write(&path, serde_json::to_string_pretty(&fixture)?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

fn ensure_git_ignored(path: &Path) -> Result<()> {
    let existing = path
        .ancestors()
        .find(|ancestor| ancestor.is_dir())
        .context("output directory has no existing ancestor")?;
    let status = Command::new("git")
        .arg("-C")
        .arg(existing)
        .args(["check-ignore", "--quiet", "--"])
        .arg(path)
        .status()
        .context("failed to run git check-ignore")?;
    match status.code() {
        Some(0) => Ok(()),
        Some(1) => bail!(
            "refusing to write the private manuscript to {}: git does not ignore it",
            path.display()
        ),
        _ => bail!(
            "refusing to write the private manuscript to {}: git check-ignore failed (is it inside a git repository?)",
            path.display()
        ),
    }
}
