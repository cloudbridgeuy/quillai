use chrono::Local;
use clap::Parser;
use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::flag;
use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

const API_COLOR: &str = "\x1b[35m";
const APP_COLOR: &str = "\x1b[36m";
const STDERR_COLOR: &str = "\x1b[31m";
const DIM_COLOR: &str = "\x1b[2m";
const RESET_COLOR: &str = "\x1b[0m";

#[derive(Debug, Parser)]
#[command(name = "xtask")]
#[command(about = "Run QuillAI development tasks")]
pub struct App {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    #[command(about = "Run the API and frontend in development mode")]
    Dev(DevArgs),
    #[command(
        about = "Run read-only quality checks with short output; full logs in target/xtask-lint"
    )]
    Lint(LintArgs),
}

#[derive(Debug, Parser)]
pub struct LintArgs {
    #[arg(long, help = "Print full output for each check")]
    verbose: bool,
}

#[derive(Debug, Parser)]
pub struct DevArgs {
    /// API port.
    #[clap(long, default_value_t = 2908)]
    api_port: u16,

    /// Frontend app port.
    #[clap(long, default_value_t = 3001)]
    app_port: u16,

    /// AI model name passed to the API provider.
    #[clap(long)]
    ai_model: Option<String>,

    /// OpenAI-compatible AI provider base URL.
    #[clap(long)]
    ai_base_url: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum Service {
    Api,
    App,
}

impl Service {
    fn label(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::App => "app",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Self::Api => API_COLOR,
            Self::App => APP_COLOR,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum StreamKind {
    Stdout,
    Stderr,
}

struct ManagedChild {
    service: Service,
    child: Child,
}

struct ManagedChildren {
    children: Vec<ManagedChild>,
    terminated: bool,
}

impl ManagedChildren {
    fn new(children: Vec<ManagedChild>) -> Self {
        Self {
            children,
            terminated: false,
        }
    }

    fn as_mut_slice(&mut self) -> &mut [ManagedChild] {
        &mut self.children
    }

    fn terminate(&mut self) {
        if self.terminated {
            return;
        }

        terminate_children(&mut self.children);
        self.terminated = true;
    }
}

impl Drop for ManagedChildren {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[derive(Debug)]
enum SupervisorEvent {
    Output {
        service: Service,
        stream: StreamKind,
        line: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = App::parse();

    match cli.command {
        Commands::Dev(args) => dev(args),
        Commands::Lint(args) => {
            if !lint(args)? {
                std::process::exit(1);
            }
            Ok(())
        }
    }
}

fn lint(args: LintArgs) -> Result<bool, Box<dyn Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask has no workspace directory")?;
    let logs = root.join("target/xtask-lint");
    std::fs::create_dir_all(&logs)?;
    let checks: &[(&str, &str, &[&str], &str)] = &[
        ("fmt", "cargo", &["fmt", "--all", "--check"], "."),
        (
            "check",
            "cargo",
            &["check", "--workspace", "--all-targets", "--quiet"],
            ".",
        ),
        (
            "clippy",
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--quiet",
                "--",
                "-D",
                "warnings",
            ],
            ".",
        ),
        (
            "test",
            "cargo",
            &["test", "--workspace", "--all-targets", "--quiet"],
            ".",
        ),
        (
            "doctest",
            "cargo",
            &["test", "--workspace", "--doc", "--quiet"],
            ".",
        ),
        ("dependencies", "cargo", &["rail", "unify", "--check"], "."),
        ("bun-test", "bun", &["test"], "app"),
    ];
    let mut failed = 0;
    for (name, program, command_args, directory) in checks {
        let path = logs.join(format!("{name}.log"));
        let mut log = std::fs::File::create(&path)?;
        let result = Command::new(program)
            .args(*command_args)
            .current_dir(root.join(directory))
            .env("CARGO_TERM_COLOR", "never")
            .env("NO_COLOR", "1")
            .env("SQLX_OFFLINE", "true")
            .stdout(log.try_clone()?)
            .stderr(log.try_clone()?)
            .status();
        let passed = match result {
            Ok(status) => status.success(),
            Err(error) => {
                writeln!(log, "Could not run {program}: {error}")?;
                false
            }
        };
        report_lint_check(name, passed, &path, args.verbose)?;
        failed += usize::from(!passed);
    }
    let path = logs.join("file-length.log");
    let passed = check_file_lengths(root, &path)?;
    report_lint_check("file-length", passed, &path, args.verbose)?;
    failed += usize::from(!passed);
    if failed == 0 {
        println!("All {} checks passed.", checks.len() + 1);
    } else {
        println!("{failed}/{} checks failed.", checks.len() + 1);
    }
    Ok(failed == 0)
}

fn report_lint_check(
    name: &str,
    passed: bool,
    path: &std::path::Path,
    verbose: bool,
) -> std::io::Result<()> {
    if passed {
        println!("PASS {name}");
    } else {
        println!("FAIL {name}: target/xtask-lint/{name}.log");
    }
    if verbose {
        std::io::copy(&mut std::fs::File::open(path)?, &mut std::io::stdout())?;
    }
    Ok(())
}

fn check_file_lengths(root: &std::path::Path, path: &std::path::Path) -> std::io::Result<bool> {
    let mut log = std::fs::File::create(path)?;
    let output = Command::new("git")
        .args(["ls-files", "-z", "--", "crates/**/*.rs", "xtask/**/*.rs"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        log.write_all(&output.stderr)?;
        return Ok(false);
    }
    let mut passed = true;
    for file in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|s| !s.is_empty())
    {
        let file = std::str::from_utf8(file)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        match std::fs::read_to_string(root.join(file)) {
            Ok(contents) => {
                let lines = contents.lines().count();
                if lines > 1000 {
                    writeln!(log, "{file}: {lines} lines (maximum 1000)")?;
                    passed = false;
                }
            }
            Err(error) => {
                writeln!(log, "{file}: {error}")?;
                passed = false;
            }
        }
    }
    Ok(passed)
}

fn dev(args: DevArgs) -> Result<(), Box<dyn Error>> {
    ensure_app_dependencies()?;

    let shutdown = Arc::new(AtomicBool::new(false));
    install_shutdown_signal_handlers(Arc::clone(&shutdown))?;

    let (tx, rx) = mpsc::channel();
    let mut children = ManagedChildren::new(vec![
        spawn_api(
            args.api_port,
            args.ai_model.as_deref(),
            args.ai_base_url.as_deref(),
            tx.clone(),
        )?,
        spawn_app(args.app_port, tx)?,
    ]);

    let ai_model = args.ai_model.as_deref().unwrap_or("env:QUILLAI_AI_MODEL");
    log_supervisor_line(format!(
        "dev started: api=http://127.0.0.1:{} app=http://127.0.0.1:{} ai_model={}",
        args.api_port, args.app_port, ai_model
    ));

    let exit_status = supervise(children.as_mut_slice(), rx, shutdown);
    children.terminate();

    if exit_status != 0 {
        std::process::exit(exit_status);
    }

    Ok(())
}

fn ensure_app_dependencies() -> Result<(), Box<dyn Error>> {
    if std::path::Path::new("./app/node_modules").exists() {
        return Ok(());
    }

    log_supervisor_line("app/node_modules missing; running bun install".into());
    let status = Command::new("bun")
        .arg("install")
        .current_dir("./app")
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("bun install failed with status {status}").into())
    }
}

fn spawn_api(
    port: u16,
    ai_model: Option<&str>,
    ai_base_url: Option<&str>,
    tx: Sender<SupervisorEvent>,
) -> Result<ManagedChild, Box<dyn Error>> {
    let mut command = Command::new("cargo");
    command.args([
        "run",
        "-p",
        "quillai_api",
        "--bin",
        "api",
        "--",
        "--port",
        &port.to_string(),
    ]);

    if let Some(model) = ai_model {
        command.env("QUILLAI_AI_MODEL", model);
    }

    if let Some(base_url) = ai_base_url {
        command.env("QUILLAI_AI_BASE_URL", base_url);
    }

    spawn_managed(Service::Api, command, tx)
}

fn spawn_app(port: u16, tx: Sender<SupervisorEvent>) -> Result<ManagedChild, Box<dyn Error>> {
    let mut command = Command::new("bun");
    command.args(["--hot", "app/src/index.tsx"]);
    command.env("PORT", port.to_string());

    spawn_managed(Service::App, command, tx)
}

fn spawn_managed(
    service: Service,
    mut command: Command,
    tx: Sender<SupervisorEvent>,
) -> Result<ManagedChild, Box<dyn Error>> {
    configure_process_group(&mut command);

    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        spawn_pipe_reader(service, StreamKind::Stdout, stdout, tx.clone());
    }

    if let Some(stderr) = child.stderr.take() {
        spawn_pipe_reader(service, StreamKind::Stderr, stderr, tx.clone());
    }

    Ok(ManagedChild { service, child })
}

fn configure_process_group(command: &mut Command) {
    #[cfg(unix)]
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }
}

fn spawn_pipe_reader(
    service: Service,
    stream: StreamKind,
    reader: impl Read + Send + 'static,
    tx: Sender<SupervisorEvent>,
) {
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            let Ok(line) = line else {
                break;
            };

            if tx
                .send(SupervisorEvent::Output {
                    service,
                    stream,
                    line,
                })
                .is_err()
            {
                break;
            }
        }
    });
}

fn install_shutdown_signal_handlers(shutdown: Arc<AtomicBool>) -> Result<(), Box<dyn Error>> {
    flag::register(SIGINT, Arc::clone(&shutdown))?;
    flag::register(SIGTERM, Arc::clone(&shutdown))?;
    flag::register(SIGHUP, shutdown)?;

    Ok(())
}

fn supervise(
    children: &mut [ManagedChild],
    rx: Receiver<SupervisorEvent>,
    shutdown: Arc<AtomicBool>,
) -> i32 {
    loop {
        if shutdown.load(Ordering::SeqCst) {
            log_supervisor_line("received shutdown signal; stopping dev services".into());
            return 130;
        }
        while let Ok(event) = rx.try_recv() {
            match event {
                SupervisorEvent::Output {
                    service,
                    stream,
                    line,
                } => write_prefixed_line(service, stream, &line),
            }
        }

        for child in children.iter_mut() {
            match child.child.try_wait() {
                Ok(Some(status)) => {
                    log_supervisor_line(format!(
                        "{} exited with {}",
                        child.service.label(),
                        status
                    ));
                    return status.code().unwrap_or(1);
                }
                Ok(None) => {}
                Err(err) => {
                    log_supervisor_line(format!(
                        "failed to poll {} process: {err}",
                        child.service.label()
                    ));
                    return 1;
                }
            }
        }

        thread::sleep(Duration::from_millis(50));
    }
}

fn terminate_children(children: &mut [ManagedChild]) {
    for child in children.iter_mut() {
        match child.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                terminate_process_group_or_child(&mut child.child);
                let _ = child.child.wait();
            }
            Err(_) => {}
        }
    }
}

fn terminate_process_group_or_child(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as libc::pid_t;
        unsafe {
            libc::kill(-pid, libc::SIGTERM);
        }
        thread::sleep(Duration::from_millis(250));
        if matches!(child.try_wait(), Ok(None)) {
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }
}

fn write_prefixed_line(service: Service, stream: StreamKind, line: &str) {
    let prefix = format_prefix(service, stream);
    match stream {
        StreamKind::Stdout => {
            let mut stdout = std::io::stdout().lock();
            let _ = writeln!(stdout, "{prefix}{line}");
        }
        StreamKind::Stderr => {
            let mut stderr = std::io::stderr().lock();
            let _ = writeln!(stderr, "{prefix}{line}");
        }
    }
}

fn format_prefix(service: Service, stream: StreamKind) -> String {
    let timestamp = Local::now().format("%H:%M:%S");
    let stream_color = match stream {
        StreamKind::Stdout => "",
        StreamKind::Stderr => STDERR_COLOR,
    };

    format!(
        "{DIM_COLOR}{timestamp}{RESET_COLOR} {service_color}{label:>3}{RESET_COLOR} {stream_color}│{RESET_COLOR} ",
        service_color = service.color(),
        label = service.label(),
    )
}

fn log_supervisor_line(line: String) {
    let timestamp = Local::now().format("%H:%M:%S");
    println!("{DIM_COLOR}{timestamp}{RESET_COLOR} \x1b[32mdev{RESET_COLOR} │ {line}");
}
