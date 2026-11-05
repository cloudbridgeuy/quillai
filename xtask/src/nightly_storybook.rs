use std::path::{Path, PathBuf};

/// Commands (program, args, working directory) that build Storybook in a snapshot.
pub(super) fn storybook_commands(snapshot: &Path) -> Vec<(String, Vec<String>, PathBuf)> {
    let dir = snapshot.join("storybook");
    [&["install", "--frozen-lockfile"][..], &["run", "build"][..]]
        .into_iter()
        .map(|args| {
            (
                "bun".to_string(),
                args.iter().map(|a| a.to_string()).collect(),
                dir.clone(),
            )
        })
        .collect()
}

/// Runs the build; returns a report line when it fails.
pub(super) fn build(snapshot: &Path) -> Option<String> {
    for (program, args, dir) in storybook_commands(snapshot) {
        let status = std::process::Command::new(&program)
            .args(&args)
            .current_dir(&dir)
            .status();
        if !matches!(status, Ok(s) if s.success()) {
            return Some(format!(
                "Storybook build failed: `{program} {}` (stories-visual.spec.ts skipped)",
                args.join(" ")
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_install_then_build_in_storybook_dir() {
        let commands = storybook_commands(Path::new("/snap"));
        let dir = PathBuf::from("/snap/storybook");
        assert_eq!(
            commands,
            vec![
                (
                    "bun".to_string(),
                    vec!["install".to_string(), "--frozen-lockfile".to_string()],
                    dir.clone()
                ),
                (
                    "bun".to_string(),
                    vec!["run".to_string(), "build".to_string()],
                    dir
                ),
            ]
        );
    }
}
