use super::*;
use std::fs::{self, File, OpenOptions};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

struct SandboxDatabase(PathBuf);

struct E2eModelCatalog {
    path: PathBuf,
    test_provider: bool,
}

fn resolve_model_catalog(
    root: &Path,
    explicit: Option<PathBuf>,
    environment: Option<PathBuf>,
) -> E2eModelCatalog {
    match explicit.or(environment) {
        Some(path) => E2eModelCatalog {
            path,
            test_provider: false,
        },
        None => E2eModelCatalog {
            path: root.join("e2e/fixtures/ai-models.json"),
            test_provider: true,
        },
    }
}

impl SandboxDatabase {
    fn create() -> Result<Self, Box<dyn Error>> {
        let directory = std::env::temp_dir().join(format!(
            "quillai-e2e-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir(&directory)?;
        let database = Self(directory);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(database.path())?;
        Ok(database)
    }

    fn path(&self) -> PathBuf {
        self.0.join("sandbox.sqlite")
    }
}

impl Drop for SandboxDatabase {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "Could not remove E2E database {}: {error}",
                self.0.display()
            );
        }
    }
}

pub(super) fn run(args: E2eArgs) -> Result<(), Box<dyn Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask has no workspace directory")?;
    std::env::set_current_dir(root)?;
    if args.verbose {
        return sandbox(args, root, &mut None, None, None);
    }
    let directory = root.join("target/xtask-e2e");
    fs::create_dir_all(&directory)?;
    let summary = directory.join("summary.txt");
    let _ = fs::remove_file(&summary);
    let mut log = Some(File::create(directory.join("e2e.log"))?);
    let result = sandbox(args, root, &mut log, Some(&summary), None);
    if let Ok(text) = fs::read_to_string(&summary) {
        println!("{}", text.trim_end());
    }
    println!("Full log: target/xtask-e2e/e2e.log");
    result
}

pub(super) fn sandbox(
    args: E2eArgs,
    root: &Path,
    log: &mut Option<File>,
    summary: Option<&Path>,
    json: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
    let app_reservation = TcpListener::bind(("127.0.0.1", args.port.unwrap_or(0)))?;
    let app_port = app_reservation.local_addr()?.port();
    let app_url = format!("http://127.0.0.1:{app_port}");
    let shutdown = Arc::new(AtomicBool::new(false));
    install_shutdown_signal_handlers(Arc::clone(&shutdown))?;
    let database = SandboxDatabase::create()?;
    let mut children = ManagedChildren::new(Vec::new());
    let (tx, rx) = mpsc::channel();

    if !root.join("app/node_modules/@playwright/test").exists() {
        let mut install = e2e_command("bun");
        install
            .args(["install", "--frozen-lockfile"])
            .current_dir(root.join("app"));
        run_tool(install, &mut children, &tx, &rx, &shutdown, log)?;
    }
    let mut build = e2e_command("cargo");
    build.args(["build", "-p", "quillai_api", "--bin", "api"]);
    build.env("SQLX_OFFLINE", "true");
    run_tool(build, &mut children, &tx, &rx, &shutdown, log)?;

    let api_port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    let api_url = format!("http://127.0.0.1:{api_port}");
    note(
        log,
        &format!(
            "E2E sandbox: api={api_url} app={app_url} database={}",
            database.path().display()
        ),
    );
    let mut api = e2e_command("cargo");
    api.args(["run", "--quiet", "-p", "quillai_api", "--bin", "api", "--"]);
    api.env("SQLX_OFFLINE", "true");
    api.args(["--host", "127.0.0.1", "--port", &api_port.to_string()]);
    api.args(["--cors-origin", &app_url]);
    let catalog = resolve_model_catalog(
        root,
        args.ai_models,
        std::env::var_os("QUILLAI_AI_MODELS_FILE").map(PathBuf::from),
    );
    api.arg("--ai-models").arg(catalog.path);
    let mut provider_url = None;
    if catalog.test_provider {
        let readiness = database.0.join("provider-port");
        let mut provider = e2e_command("bun");
        provider
            .arg(root.join("e2e/fake-ai-provider.ts"))
            .arg(&readiness);
        children
            .children
            .push(spawn_managed(Service::E2e, provider, tx.clone())?);
        let deadline = Instant::now() + Duration::from_secs(120);
        let port = loop {
            drain_output(&rx, log);
            check_services(&mut children, &shutdown)?;
            match fs::read_to_string(&readiness) {
                Ok(value) => break value.trim().parse::<u16>()?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            if Instant::now() >= deadline {
                return Err("E2E provider readiness timed out".into());
            }
            thread::sleep(Duration::from_millis(50));
        };
        wait_ready(port, "/control", &mut children, &rx, &shutdown, log)?;
        let url = format!("http://127.0.0.1:{port}");
        api.env("QUILLAI_AI_BASE_URL", format!("{url}/v1"))
            .env("QUILLAI_AI_API_KEY", "quillai-e2e-fake-key-not-a-secret");
        provider_url = Some(url);
    }
    api.arg("--db-url")
        .arg(format!("sqlite://{}", database.path().display()));
    children
        .children
        .push(spawn_managed(Service::Api, api, tx.clone())?);
    drop(app_reservation);
    let mut app = e2e_command("bun");
    app.arg("src/index.tsx")
        .current_dir(root.join("app"))
        .env("PORT", app_port.to_string());
    children
        .children
        .push(spawn_managed(Service::App, app, tx.clone())?);
    wait_ready(api_port, "/series", &mut children, &rx, &shutdown, log)?;
    wait_ready(app_port, "/", &mut children, &rx, &shutdown, log)?;

    let mut playwright = e2e_command("bun");
    playwright
        .args([
            "--bun",
            "x",
            "playwright",
            "test",
            "--config",
            "../e2e/playwright.config.ts",
        ])
        .args(args.playwright_args)
        .current_dir(root.join("app"))
        .env("QUILLAI_E2E_API_URL", api_url)
        .env("QUILLAI_E2E_APP_URL", app_url)
        .env_remove("QUILLAI_E2E_FAKE_AI_URL");
    if let Some(url) = provider_url {
        playwright.env("QUILLAI_E2E_FAKE_AI_URL", url);
    }
    if let Some(summary) = summary {
        playwright.env("QUILLAI_E2E_SUMMARY", summary);
    }
    if let Some(json) = json {
        playwright.env("PLAYWRIGHT_JSON_OUTPUT_FILE", json);
    }
    run_tool(playwright, &mut children, &tx, &rx, &shutdown, log)?;
    Ok(())
}

fn e2e_command(program: &str) -> Command {
    let mut command = Command::new(program);
    if !color_enabled() {
        command
            .env("NO_COLOR", "1")
            .env("FORCE_COLOR", "0")
            .env("CARGO_TERM_COLOR", "never");
    }
    command
}

fn drain_output(rx: &Receiver<SupervisorEvent>, log: &mut Option<File>) {
    while let Ok(SupervisorEvent::Output {
        service,
        stream,
        line,
    }) = rx.try_recv()
    {
        match log {
            Some(file) => {
                let timestamp = Local::now().format("%H:%M:%S");
                let _ = writeln!(file, "{timestamp} {:>3} │ {line}", service.label());
            }
            None => write_prefixed_line(service, stream, &line),
        }
    }
}

fn note(log: &mut Option<File>, text: &str) {
    match log {
        Some(file) => {
            let _ = writeln!(file, "{text}");
        }
        None => println!("{text}"),
    }
}

fn check_services(
    children: &mut ManagedChildren,
    shutdown: &AtomicBool,
) -> Result<(), Box<dyn Error>> {
    if shutdown.load(Ordering::SeqCst) {
        return Err("E2E interrupted; stopping sandbox".into());
    }
    for child in &mut children.children {
        if let Some(status) = child.child.try_wait()? {
            return Err(format!(
                "E2E {} exited unexpectedly: {status}",
                child.service.label()
            )
            .into());
        }
    }
    Ok(())
}

fn run_tool(
    command: Command,
    children: &mut ManagedChildren,
    tx: &Sender<SupervisorEvent>,
    rx: &Receiver<SupervisorEvent>,
    shutdown: &AtomicBool,
    log: &mut Option<File>,
) -> Result<(), Box<dyn Error>> {
    check_services(children, shutdown)?;
    children
        .children
        .push(spawn_managed(Service::E2e, command, tx.clone())?);
    loop {
        drain_output(rx, log);
        if shutdown.load(Ordering::SeqCst) {
            return Err("E2E interrupted; stopping sandbox".into());
        }
        let tool = children.children.last_mut().ok_or("E2E tool missing")?;
        if let Some(status) = tool.child.try_wait()? {
            let mut tool = children.children.pop().ok_or("E2E tool missing")?;
            terminate_process_group_or_child(&mut tool.child);
            drain_output(rx, log);
            if !status.success() {
                return Err(format!("E2E command failed: {status}").into());
            }
            check_services(children, shutdown)?;
            return Ok(());
        }
        for service in children.children.iter_mut().rev().skip(1) {
            if let Some(status) = service.child.try_wait()? {
                return Err(format!(
                    "E2E {} exited unexpectedly: {status}",
                    service.service.label()
                )
                .into());
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn wait_ready(
    port: u16,
    path: &str,
    children: &mut ManagedChildren,
    rx: &Receiver<SupervisorEvent>,
    shutdown: &AtomicBool,
    log: &mut Option<File>,
) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        drain_output(rx, log);
        check_services(children, shutdown)?;
        if http_ready(port, path) {
            note(log, &format!("E2E ready: http://127.0.0.1:{port}{path}"));
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("E2E readiness timed out: http://127.0.0.1:{port}{path}").into());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn http_ready(port: u16, path: &str) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let timeout = Duration::from_millis(250);
    let response = (|| -> std::io::Result<String> {
        let mut stream = TcpStream::connect_timeout(&address, timeout)?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
        )?;
        let mut status = String::new();
        BufReader::new(stream).read_line(&mut status)?;
        Ok(status)
    })();
    response.is_ok_and(|status| status.starts_with("HTTP/1.1 200 "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_catalog_defaults_to_workspace_fixture_and_preserves_overrides() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let default = resolve_model_catalog(root, None, None);
        assert_eq!(default.path, root.join("e2e/fixtures/ai-models.json"));
        assert!(default.path.is_absolute());
        assert!(default.path.is_file());
        assert!(default.test_provider);
        for (explicit, environment, expected) in [
            (
                Some("explicit.json"),
                Some("environment.json"),
                "explicit.json",
            ),
            (Some("explicit.json"), None, "explicit.json"),
            (None, Some("environment.json"), "environment.json"),
        ] {
            let catalog = resolve_model_catalog(
                root,
                explicit.map(PathBuf::from),
                environment.map(PathBuf::from),
            );
            assert_eq!(catalog.path, PathBuf::from(expected));
            assert!(!catalog.test_provider);
        }
    }

    #[test]
    fn frontend_port_and_playwright_arguments() {
        let args = E2eArgs::try_parse_from(["e2e", "--port", "3107", "--headed", "--trace", "on"])
            .unwrap();
        assert_eq!(args.port, Some(3107));
        assert_eq!(args.playwright_args, ["--headed", "--trace", "on"]);
        let args = E2eArgs::try_parse_from(["e2e", "chapter-sync.spec.ts", "--list"]).unwrap();
        assert_eq!(args.port, None);
        assert_eq!(args.playwright_args, ["chapter-sync.spec.ts", "--list"]);
        let args =
            E2eArgs::try_parse_from(["e2e", "--ai-models", "models.json", "--headed"]).unwrap();
        assert_eq!(args.ai_models, Some(PathBuf::from("models.json")));
        assert_eq!(args.playwright_args, ["--headed"]);
        assert!(!args.verbose);
        let args = E2eArgs::try_parse_from(["e2e", "--verbose", "--project", "chromium"]).unwrap();
        assert!(args.verbose);
        assert_eq!(args.playwright_args, ["--project", "chromium"]);
        assert!(E2eArgs::try_parse_from(["e2e", "--port", "0"]).is_err());
        assert!(E2eArgs::try_parse_from(["e2e", "--port", "65536"]).is_err());
    }
}
