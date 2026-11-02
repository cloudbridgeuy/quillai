use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const LABEL: &str = "com.quillai.nightly";

struct LaunchdConfig {
    label: String,
    hour: u8,
    minute: u8,
    working_directory: PathBuf,
    program_arguments: [String; 3],
    path_env: String,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn plist(config: &LaunchdConfig) -> String {
    let label = escape(&config.label);
    let arguments = config
        .program_arguments
        .iter()
        .map(|argument| format!("<string>{}</string>", escape(argument)))
        .collect::<String>();
    let root = escape(&config.working_directory.to_string_lossy());
    let path = escape(&config.path_env);
    let stdout = escape(&config.stdout_path.to_string_lossy());
    let stderr = escape(&config.stderr_path.to_string_lossy());
    let hour = config.hour;
    let minute = config.minute;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>{label}</string>
<key>ProgramArguments</key><array>{arguments}</array>
<key>WorkingDirectory</key><string>{root}</string>
<key>StartCalendarInterval</key><dict><key>Hour</key><integer>{hour}</integer><key>Minute</key><integer>{minute}</integer></dict>
<key>EnvironmentVariables</key><dict><key>PATH</key><string>{path}</string></dict>
<key>StandardOutPath</key><string>{stdout}</string>
<key>StandardErrorPath</key><string>{stderr}</string>
</dict></plist>
"#
    )
}

fn path_env(cargo: &Path, bun: &Path) -> String {
    let mut dirs = Vec::new();
    for dir in [
        cargo.parent().unwrap_or(Path::new("")),
        bun.parent().unwrap_or(Path::new("")),
        Path::new("/usr/bin"),
        Path::new("/bin"),
        Path::new("/usr/sbin"),
        Path::new("/sbin"),
    ] {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs.iter()
        .map(|dir| dir.to_string_lossy())
        .collect::<Vec<_>>()
        .join(":")
}

fn executable(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = std::env::var_os("PATH").ok_or("PATH is not set")?;
    let cwd = std::env::current_dir()?;
    std::env::split_paths(&path)
        .map(|dir| cwd.join(dir).join(name))
        .find(|path| {
            path.metadata()
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
        .ok_or_else(|| format!("Executable {name} not found in PATH").into())
}

fn checked_path(path: &Path) -> Result<(), Box<dyn Error>> {
    let value = path.to_str().ok_or("Paths must be valid UTF-8")?;
    if value
        .chars()
        .any(|c| c == ':' || (c < ' ' && !matches!(c, '\t' | '\n' | '\r')))
    {
        return Err(format!("Path contains an unsupported character: {value:?}").into());
    }
    Ok(())
}

fn bootout_succeeded(code: Option<i32>) -> bool {
    matches!(code, Some(0 | libc::ESRCH | 113))
}

fn bootout(domain: &str) -> Result<(), Box<dyn Error>> {
    let output = Command::new("launchctl")
        .args(["bootout", &format!("{domain}/{LABEL}")])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if bootout_succeeded(output.status.code()) {
        Ok(())
    } else {
        Err(format!("launchctl bootout failed ({}): {stderr}", output.status).into())
    }
}

pub fn run(uninstall: bool) -> Result<(), Box<dyn Error>> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    let home = PathBuf::from(home);
    if !home.is_absolute() {
        return Err("HOME must be an absolute path".into());
    }
    let plist_path = home
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"));
    let domain = format!("gui/{}", unsafe { libc::getuid() });
    if uninstall {
        bootout(&domain)?;
        match std::fs::remove_file(&plist_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        println!("Removed {}", plist_path.display());
        return Ok(());
    }
    let root = super::workspace_root()?;
    let cargo = executable("cargo")?;
    let bun = executable("bun")?;
    for path in [root, &cargo, &bun, &plist_path] {
        checked_path(path)?;
    }
    let config = LaunchdConfig {
        label: LABEL.into(),
        hour: 2,
        minute: 0,
        working_directory: root.to_path_buf(),
        program_arguments: [
            cargo.to_string_lossy().into_owned(),
            "xtask".into(),
            "nightly".into(),
        ],
        path_env: path_env(&cargo, &bun),
        stdout_path: root.join("target/nightly/launchd.out.log"),
        stderr_path: root.join("target/nightly/launchd.err.log"),
    };
    std::fs::create_dir_all(root.join("target/nightly"))?;
    std::fs::create_dir_all(plist_path.parent().ok_or("Invalid plist path")?)?;
    std::fs::write(&plist_path, plist(&config))?;
    bootout(&domain)?;
    let output = Command::new("launchctl")
        .arg("bootstrap")
        .arg(&domain)
        .arg(&plist_path)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "launchctl bootstrap failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    println!(
        "Installed {}. Next run: 02:00 local time.",
        plist_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nightly_launchd_bootout_accepts_only_success_or_absent_service() {
        for code in [0, 3, 113] {
            assert!(bootout_succeeded(Some(code)), "rejected exit code {code}");
        }
        for code in [None, Some(1), Some(5), Some(112), Some(114)] {
            assert!(!bootout_succeeded(code), "accepted exit code {code:?}");
        }
    }

    #[test]
    fn nightly_launchd_plist_contains_schedule_paths_and_arguments() {
        let config = LaunchdConfig {
            label: "com.example.changed".into(),
            hour: 17,
            minute: 43,
            working_directory: "/work/project".into(),
            program_arguments: ["/tools/cargo".into(), "xtask".into(), "nightly".into()],
            path_env: "/tools:/bun/bin:/usr/bin:/bin:/usr/sbin:/sbin".into(),
            stdout_path: "/logs/stdout.log".into(),
            stderr_path: "/errors/stderr.log".into(),
        };
        let result = plist(&config);
        assert!(result.starts_with(concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n"
        )));
        for value in [
            "<plist version=\"1.0\">",
            "<key>Label</key><string>com.example.changed</string>",
            "<key>ProgramArguments</key><array><string>/tools/cargo</string><string>xtask</string><string>nightly</string></array>",
            "<key>WorkingDirectory</key><string>/work/project</string>",
            "<key>StartCalendarInterval</key><dict><key>Hour</key><integer>17</integer><key>Minute</key><integer>43</integer></dict>",
            "<key>EnvironmentVariables</key><dict><key>PATH</key><string>/tools:/bun/bin:/usr/bin:/bin:/usr/sbin:/sbin</string></dict>",
            "<key>StandardOutPath</key><string>/logs/stdout.log</string>",
            "<key>StandardErrorPath</key><string>/errors/stderr.log</string>",
        ] { assert!(result.contains(value), "missing {value}"); }
    }

    #[test]
    fn nightly_launchd_escapes_xml_in_all_paths() {
        let config = LaunchdConfig {
            label: "com.example.&<>\"'".into(),
            hour: 2,
            minute: 0,
            working_directory: "/work/&<>\"'".into(),
            program_arguments: [
                "/tools/&<>\"'/cargo".into(),
                "xtask".into(),
                "nightly".into(),
            ],
            path_env: "/tools/&<>\"':/bun/&<>\"':/usr/bin".into(),
            stdout_path: "/stdout/&<>\"'".into(),
            stderr_path: "/stderr/&<>\"'".into(),
        };
        let result = plist(&config);
        for value in [
            "<key>Label</key><string>com.example.&amp;&lt;&gt;&quot;&apos;</string>",
            "<key>WorkingDirectory</key><string>/work/&amp;&lt;&gt;&quot;&apos;</string>",
            "<key>ProgramArguments</key><array><string>/tools/&amp;&lt;&gt;&quot;&apos;/cargo</string>",
            "<key>PATH</key><string>/tools/&amp;&lt;&gt;&quot;&apos;:/bun/&amp;&lt;&gt;&quot;&apos;:/usr/bin</string>",
            "<key>StandardOutPath</key><string>/stdout/&amp;&lt;&gt;&quot;&apos;</string>",
            "<key>StandardErrorPath</key><string>/stderr/&amp;&lt;&gt;&quot;&apos;</string>",
            "<key>StartCalendarInterval</key><dict><key>Hour</key><integer>2</integer><key>Minute</key><integer>0</integer></dict>",
        ] {
            assert!(result.contains(value), "missing {value}");
        }
    }

    #[test]
    fn nightly_launchd_path_preserves_order_and_deduplicates() {
        assert_eq!(
            path_env(Path::new("/usr/bin/cargo"), Path::new("/usr/bin/bun")),
            "/usr/bin:/bin:/usr/sbin:/sbin"
        );
        assert_eq!(
            path_env(Path::new("/cargo/cargo"), Path::new("/bun/bun")),
            "/cargo:/bun:/usr/bin:/bin:/usr/sbin:/sbin"
        );
    }
}
