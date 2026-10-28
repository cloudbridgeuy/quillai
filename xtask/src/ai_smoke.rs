use clap::Parser;
use serde_json::{json, Value};
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const CATALOG: &str = r#"{"models":[{"id":"glm-5.3-flash","label":"GLM 5.3 Flash (smoke)","input_price_per_million":0,"output_price_per_million":0,"reasoning_effort":"low"}],"defaults":{"generate":"glm-5.3-flash","edit":"glm-5.3-flash"}}"#;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(long)]
    model: Option<String>,
    #[arg(long)]
    ai_models: Option<PathBuf>,
}

struct Api {
    child: Child,
    stdout: Option<JoinHandle<Vec<u8>>>,
    stderr: Option<JoinHandle<Vec<u8>>>,
    log: PathBuf,
    key: String,
}

impl Drop for Api {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Ok(mut log) = File::create(&self.log) {
            for reader in [&mut self.stdout, &mut self.stderr] {
                if let Some(Ok(bytes)) = reader.take().map(JoinHandle::join) {
                    let _ = log.write_all(redact(&bytes, &self.key).as_bytes());
                }
            }
        }
    }
}

fn redact(bytes: &[u8], key: &str) -> String {
    let escaped = serde_json::to_string(key).unwrap_or_default();
    let escaped = escaped.trim_matches('"');
    String::from_utf8_lossy(bytes)
        .replace(key, "[redacted]")
        .replace(escaped, "[redacted]")
}

fn read_pipe(mut pipe: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

fn env_key(contents: &str) -> Option<String> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim().strip_prefix("export ").unwrap_or(line.trim());
            let (name, value) = line.split_once('=')?;
            if name.trim() != "QUILLAI_ZEN_API_KEY" {
                return None;
            }
            let value = value.trim();
            let value = if value.len() >= 2
                && ((value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\'')))
            {
                &value[1..value.len() - 1]
            } else {
                value
            };
            (!value.is_empty()).then(|| value.to_owned())
        })
        .next_back()
}

#[derive(Default, Debug)]
struct Summary {
    text: String,
    finish: Option<String>,
    usage: Option<(u64, u64)>,
    errors: Vec<Value>,
}

impl Summary {
    fn failed(&self) -> bool {
        !self.errors.is_empty() || self.text.trim().is_empty() || self.finish.is_none()
    }
}

fn summarize(stream: &str) -> Result<Summary, &'static str> {
    let mut summary = Summary::default();
    for line in stream.lines().filter(|line| !line.trim().is_empty()) {
        let event: Value = serde_json::from_str(line).map_err(|_| "invalid NDJSON")?;
        match event["type"].as_str() {
            Some("delta") => {
                let ops = event["delta"]["ops"].as_array().ok_or("invalid delta")?;
                for op in ops {
                    summary
                        .text
                        .push_str(op["insert"].as_str().ok_or("invalid insert")?);
                }
            }
            Some("usage") => {
                summary.usage = Some((
                    event["promptTokens"].as_u64().ok_or("invalid usage")?,
                    event["completionTokens"].as_u64().ok_or("invalid usage")?,
                ))
            }
            Some("done") => {
                summary.finish = Some(
                    event["finishReason"]
                        .as_str()
                        .ok_or("invalid finish reason")?
                        .to_owned(),
                )
            }
            Some("error" | "cancelled") => summary.errors.push(event),
            _ => return Err("invalid event"),
        }
    }
    Ok(summary)
}

pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let root = super::workspace_root()?;
    let env_file = std::env::var_os("QUILLAI_AI_SMOKE_ENV_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".env"));
    let key = std::env::var("QUILLAI_ZEN_API_KEY")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| fs::read_to_string(env_file).ok().and_then(|s| env_key(&s)));
    let Some(key) = key else {
        println!("SKIP ai-smoke: QUILLAI_ZEN_API_KEY is not set");
        return Ok(());
    };
    let dir = root.join("target/ai-smoke");
    fs::create_dir_all(&dir)?;
    let catalog = match args.ai_models {
        Some(path) => fs::canonicalize(path)?,
        None => {
            let path = dir.join("models.json");
            fs::write(&path, CATALOG)?;
            path
        }
    };
    let models: Value = serde_json::from_slice(&fs::read(&catalog)?)?;
    if let Some(model) = &args.model {
        if !models["models"].as_array().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry["id"].as_str() == Some(model))
        }) {
            return Err("model is not in the catalog".into());
        }
    }
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let path = dir.join(format!("smoke.db{suffix}"));
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    let built = Command::new("cargo")
        .args(["build", "-p", "quillai_api", "--bin", "api"])
        .current_dir(root)
        .env_remove("QUILLAI_ZEN_API_KEY")
        .status()?;
    if !built.success() {
        return Err("API build failed".into());
    }
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    let address = "127.0.0.1";
    let db = dir.join("smoke.db");
    let child = Command::new(root.join("target/debug/api"))
        .args([
            "--host",
            address,
            "--port",
            &port.to_string(),
            "--ai-models",
        ])
        .arg(catalog)
        .arg("--db-url")
        .arg(format!("sqlite:{}?mode=rwc", db.display()))
        .current_dir(root)
        .env_remove("QUILLAI_ZEN_API_KEY")
        .env("QUILLAI_AI_API_KEY", &key)
        .env(
            "QUILLAI_AI_BASE_URL",
            "https://opencode.ai/inference/openai/v1",
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut api = Api {
        child,
        stdout: None,
        stderr: None,
        log: dir.join("api.log"),
        key,
    };
    api.stdout = api.child.stdout.take().map(read_pipe);
    api.stderr = api.child.stderr.take().map(read_pipe);
    let url = format!("http://127.0.0.1:{port}/ai/models");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if api.child.try_wait()?.is_some() {
            return Err("API exited before readiness; see target/ai-smoke/api.log".into());
        }
        let status = Command::new("curl")
            .args(["-sf", "--max-time", "1", "-o", "/dev/null", &url])
            .env_remove("QUILLAI_ZEN_API_KEY")
            .status()?;
        if status.success() {
            break;
        }
        if Instant::now() >= deadline {
            return Err("API readiness timed out; see target/ai-smoke/api.log".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
    let mut failed = false;
    let continue_body = include_str!("../fixtures/ai-smoke/continue-body.json");
    let edit_body = include_str!("../fixtures/ai-smoke/edit-body.json");
    for (name, body) in [("continue", continue_body), ("edit", edit_body)] {
        let mut body: Value = serde_json::from_str(body)?;
        if let Some(model) = &args.model {
            body["model"] = json!(model);
        }
        let result = call(port, &body, &dir.join(format!("{name}.ndjson")), &api.key);
        match result {
            Ok(summary) => {
                let status = if summary.failed() { "FAIL" } else { "PASS" };
                let safe_text = redact(summary.text.as_bytes(), &api.key);
                let preview: String = safe_text.chars().take(200).collect();
                println!(
                    "{status} {name}: {preview:?}; finish={:?}; usage={:?}",
                    summary.finish, summary.usage
                );
                failed |= summary.failed();
            }
            Err(_) => {
                println!("FAIL {name}: request or stream failed");
                failed = true;
            }
        }
    }
    if failed {
        Err("AI smoke failed".into())
    } else {
        Ok(())
    }
}

fn call(
    port: u16,
    body: &Value,
    path: &std::path::Path,
    key: &str,
) -> Result<Summary, Box<dyn Error>> {
    let mut curl = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--max-time",
            "120",
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            "@-",
            &format!("http://127.0.0.1:{port}/ai/generate"),
        ])
        .env_remove("QUILLAI_ZEN_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    curl.stdin
        .take()
        .ok_or("curl stdin unavailable")?
        .write_all(body.to_string().as_bytes())?;
    let output = curl.wait_with_output()?;
    let response = redact(&output.stdout, key);
    fs::write(path, &response)?;
    if !output.status.success() {
        return Err("curl failed".into());
    }
    Ok(summarize(&response)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_live_streams_and_failures() {
        for (stream, prefix, usage) in [
            (
                include_str!("../fixtures/ai-smoke/continue.ndjson"),
                "It was a signal",
                (131, 36),
            ),
            (
                include_str!("../fixtures/ai-smoke/edit.ndjson"),
                "Mara stepped inside",
                (195, 21),
            ),
        ] {
            let result = summarize(stream).unwrap();
            assert!(!result.failed());
            assert!(result.text.starts_with(prefix));
            assert_eq!(result.finish.as_deref(), Some("stop"));
            assert_eq!(result.usage, Some(usage));
        }
        let error = summarize(include_str!("../fixtures/ai-smoke/error.ndjson")).unwrap();
        assert!(error.failed());
        assert_eq!(error.errors[0]["error"], "missing replacement tag");
        assert!(summarize(include_str!("../fixtures/ai-smoke/empty.ndjson"))
            .unwrap()
            .failed());
    }
}
