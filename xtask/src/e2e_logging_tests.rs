use super::*;

#[test]
fn api_output_survives_receiver_drop() {
    let directory = std::env::temp_dir().join(format!(
        "quillai-api-log-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    let path = directory.join("api.log");
    let signal = directory.join("continue");
    let ready = directory.join("ready");
    let mut command = Command::new("sh");
    command.arg("-c").arg(r#"echo early-out; echo early-err >&2; echo "$RUST_LOG"; touch "$1"; while [ ! -f "$2" ]; do sleep 0.01; done; echo late-out; echo late-err >&2"#).arg("sh").arg(&ready).arg(&signal);
    command.env("RUST_LOG", "trace");
    let (tx, rx) = mpsc::channel::<SupervisorEvent>();
    let mut child = spawn_logged_api(command, &path).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready.exists() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    drop(tx);
    drop(rx);
    fs::write(signal, "").unwrap();
    assert!(child.child.wait().unwrap().success());
    let output = fs::read_to_string(path).unwrap();
    for line in ["early-out", "early-err", "late-out", "late-err", "warn"] {
        assert!(output.contains(line), "missing {line}: {output}");
    }
    fs::remove_dir_all(directory).unwrap();
}
