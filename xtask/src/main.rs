//! See <https://github.com/matklad/cargo-xtask/>
//!
//! This binary defines auxiliary development commands that are not expressible
//! with just `cargo`.
use chrono::Local;
use clap::Parser;
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
    /// Run the API and frontend in development mode.
    Dev(DevArgs),
}

#[derive(Debug, Parser)]
pub struct DevArgs {
    /// API port.
    #[clap(long, default_value_t = 2908)]
    api_port: u16,

    /// Frontend app port.
    #[clap(long, default_value_t = 3001)]
    app_port: u16,
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
    }
}

fn dev(args: DevArgs) -> Result<(), Box<dyn Error>> {
    ensure_app_dependencies()?;

    let shutdown = Arc::new(AtomicBool::new(false));
    install_ctrlc_handler(Arc::clone(&shutdown))?;

    let (tx, rx) = mpsc::channel();
    let mut children = vec![
        spawn_api(args.api_port, tx.clone())?,
        spawn_app(args.app_port, tx)?,
    ];

    log_supervisor_line(format!(
        "dev started: api=http://127.0.0.1:{} app=http://127.0.0.1:{}",
        args.api_port, args.app_port
    ));

    let exit_status = supervise(&mut children, rx, shutdown);
    terminate_children(&mut children);

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

fn spawn_api(port: u16, tx: Sender<SupervisorEvent>) -> Result<ManagedChild, Box<dyn Error>> {
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

fn install_ctrlc_handler(shutdown: Arc<AtomicBool>) -> Result<(), Box<dyn Error>> {
    ctrlc::set_handler(move || {
        shutdown.store(true, Ordering::SeqCst);
    })?;

    Ok(())
}

fn supervise(
    children: &mut [ManagedChild],
    rx: Receiver<SupervisorEvent>,
    shutdown: Arc<AtomicBool>,
) -> i32 {
    loop {
        if shutdown.load(Ordering::SeqCst) {
            log_supervisor_line("received Ctrl-C; stopping dev services".into());
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
        return;
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
