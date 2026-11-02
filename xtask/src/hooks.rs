use std::error::Error;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
enum InstallDecision {
    Set,
    AlreadyInstalled,
    Refuse(String),
    MissingHook,
}

fn install_decision(current: Option<String>, hook_exists: bool) -> InstallDecision {
    if !hook_exists {
        return InstallDecision::MissingHook;
    }
    match current {
        None => InstallDecision::Set,
        Some(path) if path == ".githooks" => InstallDecision::AlreadyInstalled,
        Some(path) => InstallDecision::Refuse(path),
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let root = super::workspace_root()?;
    let output = Command::new("git")
        .args(["config", "--get", "core.hooksPath"])
        .current_dir(root)
        .output()?;
    let current = match output.status.code() {
        Some(0) => {
            let path = String::from_utf8(output.stdout)?;
            Some(path.strip_suffix('\n').unwrap_or(&path).to_owned())
        }
        Some(1) => None,
        _ => return Err(String::from_utf8_lossy(&output.stderr).into_owned().into()),
    };
    match install_decision(current, root.join(".githooks/pre-commit").is_file()) {
        InstallDecision::Set => {
            let status = Command::new("git")
                .args(["config", "core.hooksPath", ".githooks"])
                .current_dir(root)
                .status()?;
            if !status.success() {
                return Err(format!("Could not set core.hooksPath: {status}").into());
            }
            println!("Installed hooks: core.hooksPath=.githooks");
            Ok(())
        }
        InstallDecision::AlreadyInstalled => {
            println!("Hooks already installed: core.hooksPath=.githooks");
            Ok(())
        }
        InstallDecision::Refuse(path) => {
            Err(format!("Refusing to replace existing core.hooksPath: {path:?}").into())
        }
        InstallDecision::MissingHook => Err("Missing .githooks/pre-commit".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_path_when_unconfigured() {
        assert_eq!(install_decision(None, true), InstallDecision::Set);
    }

    #[test]
    fn recognizes_installed_path() {
        assert_eq!(
            install_decision(Some(".githooks".into()), true),
            InstallDecision::AlreadyInstalled
        );
    }

    #[test]
    fn refuses_other_paths_unchanged() {
        for path in ["custom/hooks", "/tmp/hooks", "", ".githooks/", "hooks\n"] {
            assert_eq!(
                install_decision(Some(path.into()), true),
                InstallDecision::Refuse(path.into())
            );
        }
    }

    #[test]
    fn missing_hook_prevents_installation_for_every_path() {
        for current in [None, Some(".githooks".into()), Some("custom/hooks".into())] {
            assert_eq!(
                install_decision(current, false),
                InstallDecision::MissingHook
            );
        }
    }
}
