use super::*;
use std::fs::{self, OpenOptions};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

struct SandboxDatabase(PathBuf);

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
    for address in ["127.0.0.1:3001", "[::1]:3001"] {
        if TcpStream::connect_timeout(&address.parse()?, Duration::from_millis(100)).is_ok() {
            return Err(format!(
                "E2E requires frontend port 3001; stop its current listener at {address}"
            )
            .into());
        }
    }
    let app_reservation = TcpListener::bind("127.0.0.1:3001").map_err(|error| {
        format!("E2E requires frontend port 3001; stop its current listener: {error}")
    })?;
    let shutdown = Arc::new(AtomicBool::new(false));
    install_shutdown_signal_handlers(Arc::clone(&shutdown))?;
    let database = SandboxDatabase::create()?;
    let mut children = ManagedChildren::new(Vec::new());
    let (tx, rx) = mpsc::channel();

    if !root.join("app/node_modules/@playwright/test").exists() {
        let mut install = Command::new("bun");
        install.arg("install").current_dir(root.join("app"));
        run_tool(install, &mut children, &tx, &rx, &shutdown)?;
    }
    let mut build = Command::new("cargo");
    build.args(["build", "-p", "quillai_api", "--bin", "api"]);
    build.env("SQLX_OFFLINE", "true");
    run_tool(build, &mut children, &tx, &rx, &shutdown)?;

    let api_port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    let api_url = format!("http://127.0.0.1:{api_port}");
    println!(
        "E2E sandbox: api={api_url} app=http://127.0.0.1:3001 database={}",
        database.path().display()
    );
    let mut api = Command::new("cargo");
    api.args(["run", "--quiet", "-p", "quillai_api", "--bin", "api", "--"]);
    api.env("SQLX_OFFLINE", "true");
    api.args(["--host", "127.0.0.1", "--port", &api_port.to_string()]);
    api.arg("--db-url")
        .arg(format!("sqlite://{}", database.path().display()));
    children
        .children
        .push(spawn_managed(Service::Api, api, tx.clone())?);
    drop(app_reservation);
    let mut app = Command::new("bun");
    app.arg("app/src/index.tsx").env("PORT", "3001");
    children
        .children
        .push(spawn_managed(Service::App, app, tx.clone())?);
    wait_ready(api_port, "/series", &mut children, &rx, &shutdown)?;
    wait_ready(3001, "/", &mut children, &rx, &shutdown)?;

    let mut playwright = Command::new("bun");
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
        .env("QUILLAI_E2E_API_URL", api_url);
    run_tool(playwright, &mut children, &tx, &rx, &shutdown)?;
    Ok(())
}

fn drain_output(rx: &Receiver<SupervisorEvent>) {
    while let Ok(SupervisorEvent::Output {
        service,
        stream,
        line,
    }) = rx.try_recv()
    {
        write_prefixed_line(service, stream, &line);
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
) -> Result<(), Box<dyn Error>> {
    check_services(children, shutdown)?;
    children
        .children
        .push(spawn_managed(Service::E2e, command, tx.clone())?);
    loop {
        drain_output(rx);
        if shutdown.load(Ordering::SeqCst) {
            return Err("E2E interrupted; stopping sandbox".into());
        }
        let tool = children.children.last_mut().ok_or("E2E tool missing")?;
        if let Some(status) = tool.child.try_wait()? {
            let mut tool = children.children.pop().ok_or("E2E tool missing")?;
            terminate_process_group_or_child(&mut tool.child);
            drain_output(rx);
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
) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        drain_output(rx);
        check_services(children, shutdown)?;
        if http_ready(port, path) {
            println!("E2E ready: http://127.0.0.1:{port}{path}");
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
