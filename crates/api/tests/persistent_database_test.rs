use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "quillai-persistent-db-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Server {
    child: Child,
    port: u16,
    log_path: PathBuf,
}

impl Server {
    fn start(directory: &Path, database_url: Option<&str>, environment_url: Option<&str>) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let log_path = directory.join(format!("api-{port}.log"));
        let log_file = std::fs::File::create(&log_path).unwrap();
        let port_text = port.to_string();
        let mut command = Command::new(env!("CARGO_BIN_EXE_api"));
        command
            .current_dir(directory)
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port_text,
                "--log-level",
                "warn",
            ])
            .env_remove("QUILLAI_AI_MODELS_FILE")
            .env_remove("QUILLAI_AI_MODEL")
            .env_remove("LISTEN_FDS")
            .env_remove("LISTEN_PID")
            .stdout(Stdio::null())
            .stderr(log_file);
        match environment_url {
            Some(url) => {
                command.env("QUILLAI_API_DB_URL", url);
            }
            None => {
                command.env_remove("QUILLAI_API_DB_URL");
            }
        }
        if let Some(url) = database_url {
            command.args(["--db-url", url]);
        }
        let child = command.spawn().unwrap();
        Self {
            child,
            port,
            log_path,
        }
    }

    fn log(&self) -> String {
        std::fs::read_to_string(&self.log_path).unwrap_or_default()
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

fn file_url(path: &Path) -> String {
    assert!(
        path.starts_with(std::env::temp_dir()),
        "refusing non-temporary database {}",
        path.display()
    );
    format!("sqlite:{}?mode=rwc", path.display())
}

async fn wait_ready(server: &mut Server) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    let url = format!("http://127.0.0.1:{}/series", server.port);
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = server.child.try_wait().unwrap() {
            panic!("server exited {status}: {}", server.log());
        }
        if let Ok(response) = client.get(&url).send().await {
            if response.status().is_success() {
                return;
            }
        }
        if Instant::now() >= deadline {
            panic!("server not ready:\n{}", server.log());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn create_series(port: u16, title: &str) -> serde_json::Value {
    let response = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/series"))
        .header("content-type", "application/json")
        .body(format!(r#"{{"title":"{title}","position":0}}"#))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status, 201, "{body}");
    serde_json::from_str(&body).unwrap()
}

async fn list_series(port: u16) -> serde_json::Value {
    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/series"))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    response.json().await.unwrap()
}

#[test]
fn help_shows_persistent_file_database_default() {
    let output = Command::new(env!("CARGO_BIN_EXE_api"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "{help}");
    assert!(
        help.contains("[default: sqlite:quillai.db?mode=rwc]"),
        "{help}"
    );
}

#[tokio::test]
async fn server_restart_keeps_created_series() {
    let directory = TempDir::new();
    let database = directory.0.join("quillai.db");
    assert!(!database.exists());

    let mut server = Server::start(&directory.0, None, None);
    wait_ready(&mut server).await;
    assert!(database.is_file(), "default database was not created");
    let created = create_series(server.port, "Kept across restart").await;
    server.stop();

    let mut server = Server::start(&directory.0, None, None);
    wait_ready(&mut server).await;
    assert_eq!(list_series(server.port).await, serde_json::json!([created]));
}

#[tokio::test]
async fn explicit_db_url_overrides_environment_database() {
    let directory = TempDir::new();
    let environment = directory.0.join("environment.sqlite");
    let flagged = directory.0.join("flagged.sqlite");
    let environment_url = file_url(&environment);
    let flagged_url = file_url(&flagged);

    let mut server = Server::start(&directory.0, Some(&flagged_url), Some(&environment_url));
    wait_ready(&mut server).await;
    let created = create_series(server.port, "Flagged database").await;
    server.stop();

    assert!(flagged.is_file());
    assert!(
        !environment.exists(),
        "environment database was opened despite --db-url"
    );
    assert!(!directory.0.join("quillai.db").exists());

    let mut server = Server::start(&directory.0, Some(&flagged_url), Some(&environment_url));
    wait_ready(&mut server).await;
    assert_eq!(list_series(server.port).await, serde_json::json!([created]));
}

async fn series_count_after_connection_close(url: &str) -> Result<i64, sqlx::Error> {
    let options = url
        .parse::<SqliteConnectOptions>()
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .connect_with(options.clone())
        .await
        .unwrap();
    quillai_api::database::migrate(&pool).await.unwrap();
    sqlx::query("INSERT INTO universes (title, position) VALUES ('Kept', 0)")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    sqlx::query_scalar("SELECT COUNT(*) FROM universes")
        .fetch_one(&pool)
        .await
}

#[tokio::test]
async fn closed_connection_drops_temporary_database_and_file_database_keeps_rows() {
    let error = series_count_after_connection_close("sqlite://")
        .await
        .expect_err("temporary database should not survive a closed connection");
    let message = error.to_string();
    assert!(
        message.contains("no such table"),
        "unexpected error after closing the temporary database: {message}"
    );

    let directory = TempDir::new();
    let database = directory.0.join("idle.sqlite");
    let count = series_count_after_connection_close(&file_url(&database))
        .await
        .expect("file database should reopen with its schema");
    assert_eq!(count, 1);
}
