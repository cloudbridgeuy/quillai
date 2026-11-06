use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

type Command = (String, Vec<String>, PathBuf);

/// Commands (program, args, working directory) that build Storybook in a snapshot.
/// The app install comes first: Storybook's preview CSS resolves `tailwindcss` from `app/`.
pub(super) fn storybook_commands(snapshot: &Path) -> Vec<Command> {
    let app = snapshot.join("app");
    let dir = snapshot.join("storybook");
    [
        (&["install", "--frozen-lockfile"][..], app),
        (&["install", "--frozen-lockfile"][..], dir.clone()),
        (&["run", "build"][..], dir),
    ]
    .into_iter()
    .map(|(args, cwd)| {
        (
            "bun".to_string(),
            args.iter().map(|a| a.to_string()).collect(),
            cwd,
        )
    })
    .collect()
}

/// Report line for a failed command.
fn failure_report(command: &Command, log: &Path) -> String {
    format!(
        "Storybook build failed: `{} {}` (stories-visual.spec.ts skipped; see {})",
        command.0,
        command.1.join(" "),
        log.display()
    )
}

/// Runs the build, logging to `<directory>/storybook-build.log`; returns a report line when it fails.
pub(super) fn build(snapshot: &Path, directory: &Path) -> Option<String> {
    run_commands(
        &storybook_commands(snapshot),
        snapshot,
        &directory.join("storybook-build.log"),
    )
}

fn run_commands(commands: &[Command], snapshot: &Path, log: &Path) -> Option<String> {
    for command in commands {
        if !run_logged(command, log) {
            // Without the static output the visual spec skips instead of timing out.
            let _ = fs::remove_dir_all(snapshot.join("storybook/storybook-static"));
            return Some(failure_report(command, log));
        }
    }
    None
}

fn run_logged((program, args, dir): &Command, log: &Path) -> bool {
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log) else {
        return false;
    };
    let _ = writeln!(
        file,
        "$ {program} {} (in {})",
        args.join(" "),
        dir.display()
    );
    let (Ok(out), Ok(err)) = (file.try_clone(), file.try_clone()) else {
        return false;
    };
    let status = std::process::Command::new(program)
        .args(args)
        .current_dir(dir)
        .stdout(out)
        .stderr(err)
        .status();
    let ok = matches!(status, Ok(s) if s.success());
    let _ = writeln!(file, "-> {}", if ok { "ok" } else { "failed" });
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_install_app_then_storybook_then_build() {
        let commands = storybook_commands(Path::new("/snap"));
        let dir = PathBuf::from("/snap/storybook");
        assert_eq!(
            commands,
            vec![
                (
                    "bun".to_string(),
                    vec!["install".to_string(), "--frozen-lockfile".to_string()],
                    PathBuf::from("/snap/app")
                ),
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

    fn sh(script: &str, dir: &Path) -> Command {
        (
            "sh".to_string(),
            vec!["-c".to_string(), script.to_string()],
            dir.to_path_buf(),
        )
    }

    #[test]
    fn failed_command_logs_reports_and_removes_output() {
        let tmp = std::env::temp_dir().join(format!("quillai-sb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let out = tmp.join("storybook/storybook-static");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("index.json"), "{}").unwrap();
        let log = tmp.join("storybook-build.log");
        let commands = vec![sh("echo ok-line", &tmp), sh("echo boom >&2; exit 1", &tmp)];
        let report = run_commands(&commands, &tmp, &log).expect("failure report");
        assert!(report.contains("exit 1"), "{report}");
        assert!(report.contains(&log.display().to_string()), "{report}");
        assert!(!out.exists());
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.contains("ok-line") && text.contains("boom"), "{text}");
        assert!(text.contains("$ sh -c"), "{text}");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn successful_commands_keep_output() {
        let tmp = std::env::temp_dir().join(format!("quillai-sb-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let out = tmp.join("storybook/storybook-static");
        std::fs::create_dir_all(&out).unwrap();
        let log = tmp.join("storybook-build.log");
        assert_eq!(run_commands(&[sh("true", &tmp)], &tmp, &log), None);
        assert!(out.exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
