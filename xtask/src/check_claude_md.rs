use std::io::Write;
use std::path::Path;
use std::process::Command;

const LIMIT: usize = 16_000;

fn within_limit(contents: &[u8]) -> bool {
    contents.len() <= LIMIT
}

pub fn run(root: &Path, log_path: &Path) -> std::io::Result<bool> {
    let mut log = std::fs::File::create(log_path)?;
    let output = Command::new("git")
        .args(["ls-files", "--error-unmatch", "--", "CLAUDE.md"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        log.write_all(&output.stderr)?;
        return Ok(false);
    }
    match std::fs::read(root.join("CLAUDE.md")) {
        Ok(contents) if within_limit(&contents) => Ok(true),
        Ok(contents) => {
            writeln!(log, "CLAUDE.md: {} bytes (maximum {LIMIT})", contents.len())?;
            Ok(false)
        }
        Err(error) => {
            writeln!(log, "CLAUDE.md: {error}")?;
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_counts_bytes_and_includes_boundary() {
        assert!(within_limit(&vec![0; LIMIT]));
        assert!(!within_limit(&vec![0; LIMIT + 1]));
        assert!(within_limit("é".repeat(LIMIT / 2).as_bytes()));
        assert!(!within_limit("é".repeat(LIMIT / 2 + 1).as_bytes()));
    }
}
