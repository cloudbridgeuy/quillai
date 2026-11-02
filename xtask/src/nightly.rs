#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Passed,
    Failed,
    Flaky,
    Skipped,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct TestResult {
    file: String,
    line: u64,
    title: String,
    project: String,
    outcome: Outcome,
}
#[derive(Debug, Clone)]
struct Known {
    browser: String,
    spec: String,
    ticket: String,
}
#[derive(Default)]
struct Classification {
    new_failures: Vec<TestResult>,
    known_failures: Vec<(TestResult, String)>,
    flaky: Vec<TestResult>,
    fixed: Vec<TestResult>,
    stale: Vec<Known>,
}
struct Meta {
    date: String,
    duration: u64,
    count: usize,
    passed: usize,
    failed: usize,
    flaky: usize,
    skipped: usize,
}
fn parse_results(input: &str) -> Result<Vec<TestResult>, String> {
    let json: serde_json::Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let suites = json
        .get("suites")
        .and_then(|v| v.as_array())
        .ok_or("missing suites")?;
    let mut results = Vec::new();
    fn walk(suites: &[serde_json::Value], results: &mut Vec<TestResult>) -> Result<(), String> {
        for suite in suites {
            if let Some(specs) = suite.get("specs").and_then(|v| v.as_array()) {
                for spec in specs {
                    let field = |name| {
                        spec.get(name)
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| format!("missing spec {name}"))
                    };
                    let file = field("file")?;
                    let title = field("title")?;
                    let line = spec
                        .get("line")
                        .and_then(|v| v.as_u64())
                        .ok_or("missing spec line")?;
                    let tests = spec
                        .get("tests")
                        .and_then(|v| v.as_array())
                        .ok_or("missing tests")?;
                    for test in tests {
                        let project = test
                            .get("projectName")
                            .and_then(|v| v.as_str())
                            .ok_or("missing projectName")?;
                        let attempts = test
                            .get("results")
                            .and_then(|v| v.as_array())
                            .ok_or("missing results")?;
                        let mut failed = false;
                        let mut outcome = Outcome::Skipped;
                        for attempt in attempts {
                            let status = attempt
                                .get("status")
                                .and_then(|v| v.as_str())
                                .ok_or("missing status")?;
                            let retry = attempt
                                .get("retry")
                                .and_then(|v| v.as_u64())
                                .ok_or("missing retry")?;
                            outcome = match status {
                                "passed" if failed && retry > 0 => Outcome::Flaky,
                                "passed" => Outcome::Passed,
                                "skipped" => Outcome::Skipped,
                                "failed" | "timedOut" | "interrupted" => {
                                    failed = true;
                                    Outcome::Failed
                                }
                                _ => return Err(format!("unknown status {status}")),
                            };
                        }
                        results.push(TestResult {
                            file: file.into(),
                            line,
                            title: title.into(),
                            project: project.into(),
                            outcome,
                        });
                    }
                }
            }
            if let Some(children) = suite.get("suites").and_then(|v| v.as_array()) {
                walk(children, results)?;
            }
        }
        Ok(())
    }
    walk(suites, &mut results)?;
    if results.is_empty() {
        return Err("no test results".into());
    }
    Ok(results)
}
fn parse_known_failures(input: &str) -> Result<Vec<Known>, String> {
    input
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            Some((|| {
                let parts: Vec<_> = line.split_whitespace().collect();
                if parts.len() != 3 || !["*", "chromium", "firefox", "webkit"].contains(&parts[0]) {
                    return Err(format!("invalid known failure at line {}", i + 1));
                }
                let valid_spec = parts[1] == "*"
                    || parts[1].rsplit_once(':').is_some_and(|(file, line)| {
                        !file.is_empty()
                            && (line == "*" || line.parse::<u64>().is_ok_and(|n| n > 0))
                    });
                let valid_ticket = parts[2]
                    .strip_prefix("GUZ-")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
                if !valid_spec || !valid_ticket {
                    return Err(format!("invalid known failure at line {}", i + 1));
                }
                Ok(Known {
                    browser: parts[0].into(),
                    spec: parts[1].into(),
                    ticket: parts[2].into(),
                })
            })())
        })
        .collect()
}
fn matches(known: &Known, result: &TestResult) -> bool {
    if known.browser != "*" && known.browser != result.project {
        return false;
    }
    if known.spec == "*" {
        return true;
    }
    known.spec.rsplit_once(':').is_some_and(|(file, line)| {
        (result.file == file
            || result
                .file
                .strip_suffix(file)
                .is_some_and(|prefix| prefix.ends_with('/')))
            && (line == "*" || line.parse::<u64>() == Ok(result.line))
    })
}
fn classify(
    current: &[TestResult],
    known: &[Known],
    previous: Option<&[TestResult]>,
) -> Classification {
    let mut c = Classification::default();
    for result in current {
        match result.outcome {
            Outcome::Failed => match known.iter().find(|entry| matches(entry, result)) {
                Some(entry) => c
                    .known_failures
                    .push((result.clone(), entry.ticket.clone())),
                None => c.new_failures.push(result.clone()),
            },
            Outcome::Flaky => c.flaky.push(result.clone()),
            Outcome::Passed => {
                if previous.is_some_and(|old| {
                    old.iter().any(|p| {
                        p.file == result.file
                            && p.line == result.line
                            && p.title == result.title
                            && p.project == result.project
                            && p.outcome == Outcome::Failed
                    })
                }) {
                    c.fixed.push(result.clone());
                }
            }
            Outcome::Skipped => {}
        }
    }
    c.stale = known
        .iter()
        .filter(|entry| {
            let matched: Vec<_> = current.iter().filter(|r| matches(entry, r)).collect();
            !matched.is_empty() && matched.iter().all(|r| r.outcome == Outcome::Passed)
        })
        .cloned()
        .collect();
    c
}
fn render_report(c: &Classification, meta: &Meta) -> String {
    render(c, meta, usize::MAX)
}
fn render(c: &Classification, meta: &Meta, limit: usize) -> String {
    let mut lines = vec![
        format!("# Nightly {}", meta.date),
        format!(
            "Duration: {}s | Tests: {} | Passed: {} | Failed: {} | Flaky: {} | Skipped: {}",
            meta.duration, meta.count, meta.passed, meta.failed, meta.flaky, meta.skipped
        ),
    ];
    for (title, rows) in [
        (
            "New failures",
            c.new_failures.iter().map(format_result).collect::<Vec<_>>(),
        ),
        (
            "Known failures",
            c.known_failures
                .iter()
                .map(|(r, ticket)| format!("{} ({ticket})", format_result(r)))
                .collect(),
        ),
        ("Flaky", c.flaky.iter().map(format_result).collect()),
        (
            "Fixed since previous",
            c.fixed.iter().map(format_result).collect(),
        ),
        (
            "Stale known failures",
            c.stale
                .iter()
                .map(|k| format!("{} {} {}", k.browser, k.spec, k.ticket))
                .collect(),
        ),
    ] {
        let count = rows.len();
        lines.push(format!("\n## {title} ({count})"));
        lines.extend(rows.into_iter().take(limit).map(|row| format!("- {row}")));
        if count > limit {
            lines.push(format!("... and {} more", count - limit));
        }
    }
    lines.join("\n") + "\n"
}
fn format_result(r: &TestResult) -> String {
    format!(
        "{}:{} [{}] {}",
        r.file,
        r.line,
        r.project,
        r.title.replace(['\n', '\r'], " ")
    )
}
#[cfg(test)]
fn render_summary(c: &Classification, meta: &Meta) -> String {
    render(c, meta, 7)
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_stream(input: &str) -> Result<Vec<TestResult>, String> {
    let mut rows: Vec<(TestResult, bool)> = Vec::new();
    for line in input.split_inclusive('\n') {
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) if !line.ends_with('\n') => continue,
            Err(error) => return Err(error.to_string()),
        };
        let string = |field| {
            value
                .get(field)
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("missing {field}"))
        };
        let file = string("file")?;
        let title = string("title")?;
        let project = string("project")?;
        let line = value
            .get("line")
            .and_then(|v| v.as_u64())
            .ok_or("missing line")?;
        let _retry = value
            .get("retry")
            .and_then(|v| v.as_u64())
            .ok_or("missing retry")?;
        let outcome = match string("status")? {
            "passed" => Outcome::Passed,
            "skipped" => Outcome::Skipped,
            "failed" | "timedOut" | "interrupted" => Outcome::Failed,
            status => return Err(format!("unknown status {status}")),
        };
        let result = TestResult {
            file: file.into(),
            line,
            title: title.into(),
            project: project.into(),
            outcome,
        };
        if let Some((old, failed)) = rows.iter_mut().find(|(old, _)| {
            old.file == result.file
                && old.line == result.line
                && old.title == result.title
                && old.project == result.project
        }) {
            *failed |= old.outcome == Outcome::Failed;
            *old = result;
            if old.outcome == Outcome::Passed && *failed {
                old.outcome = Outcome::Flaky;
            }
        } else {
            rows.push((result, false));
        }
    }
    Ok(rows.into_iter().map(|(result, _)| result).collect())
}
fn snapshot_args(path: &std::path::Path, sha: &str) -> Vec<String> {
    vec![
        "worktree".into(),
        "add".into(),
        "--detach".into(),
        path.display().to_string(),
        sha.into(),
    ]
}
fn cleanup_args(path: &std::path::Path) -> Vec<String> {
    vec![
        "worktree".into(),
        "remove".into(),
        "--force".into(),
        path.display().to_string(),
    ]
}
fn report_with_snapshot(report: &str, sha: &str, timeout: Option<u64>) -> String {
    format!(
        "Snapshot: {sha}\n{}\n{report}",
        timeout
            .map(|n| format!("incomplete (timed out after {n}s)"))
            .unwrap_or_default()
    )
}
fn timeout_seconds(flag: Option<u64>, env: Option<&str>) -> Result<u64, String> {
    let value = match flag {
        Some(value) => value,
        None => env
            .map(str::parse)
            .transpose()
            .map_err(|_| "invalid nightly timeout")?
            .unwrap_or(7200),
    };
    if value == 0 {
        Err("nightly timeout must be positive".into())
    } else {
        Ok(value)
    }
}
fn report(
    current: &[TestResult],
    known: &[Known],
    previous: Option<&[TestResult]>,
    date: &str,
    duration: u64,
    sha: &str,
    timeout: Option<u64>,
) -> String {
    let count = |outcome| current.iter().filter(|r| r.outcome == outcome).count();
    let meta = Meta {
        date: date.into(),
        duration,
        count: current.len(),
        passed: count(Outcome::Passed),
        failed: count(Outcome::Failed),
        flaky: count(Outcome::Flaky),
        skipped: count(Outcome::Skipped),
    };
    report_with_snapshot(
        &render_report(&classify(current, known, previous), &meta),
        sha,
        timeout,
    )
}
fn git(root: &std::path::Path, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {args:?} failed: {status}").into())
    }
}
pub(super) fn run(flag: Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::{self, File};
    use std::time::{Duration, Instant};
    let root = super::workspace_root()?;
    let timeout = timeout_seconds(
        flag,
        std::env::var("QUILLAI_NIGHTLY_TIMEOUT").ok().as_deref(),
    )?;
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let base = root.join("target/nightly");
    let directory = base.join(&date);
    fs::create_dir_all(&directory)?;
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--verify", "main^{commit}"])
        .current_dir(root)
        .output()?;
    if !sha.status.success() {
        return Err("cannot resolve main".into());
    }
    let sha = String::from_utf8(sha.stdout)?.trim().to_string();
    for entry in fs::read_dir(&base)? {
        let entry = entry?;
        if chrono::NaiveDate::parse_from_str(&entry.file_name().to_string_lossy(), "%Y-%m-%d")
            .is_ok()
            && entry.path().join("src").exists()
        {
            git(root, &cleanup_args(&entry.path().join("src")))?;
        }
    }
    let snapshot = directory.join("src");
    let original = std::env::current_dir()?;
    git(root, &snapshot_args(&snapshot, &sha))?;
    let execution = (|| -> Result<(), Box<dyn std::error::Error>> {
        std::env::set_current_dir(&snapshot)?;
        std::env::set_var("CARGO_TARGET_DIR", root.join("target"));
        let json = directory.join("results.json");
        let stream = directory.join("results.jsonl");
        for path in [&json, &stream, &directory.join("report.md")] {
            match fs::remove_file(path) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.into()),
            }
        }
        File::create(&stream)?;
        fs::write(
            directory.join("report.md"),
            format!("Incomplete: run in progress\nSnapshot: {sha}\n"),
        )?;
        std::env::set_var("QUILLAI_E2E_RESULTS_JSONL", &stream);
        File::create(directory.join("provider-audit.log"))?;
        std::env::set_var(
            "QUILLAI_E2E_PROVIDER_AUDIT",
            directory.join("provider-audit.log"),
        );
        let known = parse_known_failures(&fs::read_to_string(
            snapshot.join("e2e/known-failures.txt"),
        )?)?;
        let mut earlier = fs::read_dir(&base)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                (chrono::NaiveDate::parse_from_str(&name, "%Y-%m-%d").is_ok()
                    && name < date
                    && entry.path().join("results.json").is_file())
                .then_some((name, entry.path().join("results.json")))
            })
            .collect::<Vec<_>>();
        earlier.sort_by(|a, b| b.0.cmp(&a.0));
        let previous = earlier
            .first()
            .map(|(_, path)| -> Result<_, Box<dyn std::error::Error>> {
                Ok(parse_results(&fs::read_to_string(path)?)?)
            })
            .transpose()?;
        let mut log = Some(File::create(directory.join("run.log"))?);
        let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        super::install_shutdown_signal_handlers(shutdown)?;
        let start = Instant::now();
        let (stop, receiver) = std::sync::mpsc::channel();
        let timed_out = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let timed_out_thread = timed_out.clone();
        let checkpoint_stream = stream.clone();
        let checkpoint_report = directory.join("report.md");
        let checkpoint_date = date.clone();
        let checkpoint_sha = sha.clone();
        let checkpoint_known = known.clone();
        let checkpoint_previous = previous.clone();
        let watchdog = std::thread::spawn(move || loop {
            if receiver.recv_timeout(Duration::from_secs(1)).is_ok() {
                break;
            }
            if let Ok(data) = fs::read_to_string(&checkpoint_stream) {
                if let Ok(current) = parse_stream(&data) {
                    let text = report(
                        &current,
                        &checkpoint_known,
                        checkpoint_previous.as_deref(),
                        &checkpoint_date,
                        start.elapsed().as_secs(),
                        &checkpoint_sha,
                        None,
                    );
                    let temporary = checkpoint_report.with_extension("tmp");
                    if fs::write(&temporary, format!("Incomplete: run in progress\n{text}")).is_ok()
                    {
                        let _ = fs::rename(temporary, &checkpoint_report);
                    }
                }
            }
            if start.elapsed() >= Duration::from_secs(timeout) {
                timed_out_thread.store(true, std::sync::atomic::Ordering::SeqCst);
                let _ = signal_hook::low_level::raise(super::SIGTERM);
            }
        });
        let args = super::E2eArgs {
            ai_models: None,
            port: None,
            verbose: false,
            playwright_args: vec![
                "--retries=1".into(),
                format!(
                    "--reporter={},json",
                    snapshot.join("e2e/summary-reporter.ts").display()
                ),
            ],
        };
        let execution = super::e2e::sandbox(
            args,
            &snapshot,
            &mut log,
            Some(&directory.join("summary.txt")),
            Some(&json),
        );
        let _ = stop.send(());
        watchdog.join().map_err(|_| "nightly watchdog panicked")?;
        let expired = timed_out.load(std::sync::atomic::Ordering::SeqCst);
        let json_data = fs::read_to_string(&json).ok();
        let parsed = json_data
            .as_deref()
            .and_then(|data| parse_results(data).ok());
        let infrastructure_error = json_data
            .as_deref()
            .and_then(|data| serde_json::from_str::<serde_json::Value>(data).ok())
            .and_then(|value| {
                value
                    .get("errors")
                    .and_then(|errors| errors.as_array())
                    .map(|errors| !errors.is_empty())
            })
            .unwrap_or(false);
        let incomplete = parsed.is_none() || infrastructure_error;
        let current = match parsed {
            Some(current) => current,
            None => parse_stream(&fs::read_to_string(&stream).unwrap_or_default())?,
        };
        let text = report(
            &current,
            &known,
            previous.as_deref(),
            &date,
            start.elapsed().as_secs(),
            &sha,
            expired.then_some(timeout),
        );
        let text = if incomplete && !expired {
            format!("Incomplete: missing or invalid final results, or infrastructure error\n{text}")
        } else {
            text
        };
        fs::write(directory.join("report.md"), &text)?;
        println!("{text}");
        if expired {
            return Err(format!("nightly timed out after {timeout}s").into());
        }
        if let Err(error) = execution {
            if !error.to_string().starts_with("E2E command failed:") {
                return Err(format!("nightly sandbox failed: {error}").into());
            }
        }
        if incomplete {
            return Err("nightly produced incomplete results or an infrastructure error".into());
        }
        if current.is_empty() {
            return Err("nightly produced no results".into());
        }
        Ok(())
    })();
    let restored = std::env::set_current_dir(original);
    let cleaned = git(root, &cleanup_args(&snapshot));
    restored?;
    cleaned?;
    execution
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stream_ignores_torn_tail_and_last_status_wins() {
        let input = concat!(
            "{\"file\":\"a.spec.ts\",\"line\":5,\"title\":\"test\",\"project\":\"chromium\",\"status\":\"failed\",\"retry\":1}\n",
            "{\"file\":\"a.spec.ts\",\"line\":5,\"title\":\"test\",\"project\":\"chromium\",\"status\":\"passed\",\"retry\":0}\n",
            "{\"file\":\"a.spec.ts\",\"line\":5,\"title\":\"test\",\"project\":\"firefox\",\"status\":\"passed\",\"retry\":1}\n",
            "{\"file\":\"a.spec.ts\",\"line\":5,\"title\":\"test\",\"project\":\"firefox\",\"status\":\"failed\",\"retry\":0}\n",
            "{\"file\":"
        );
        let results = parse_stream(input).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].outcome, Outcome::Flaky);
        assert_eq!(results[1].outcome, Outcome::Failed);
        let complete = input
            .strip_suffix("{\"file\":")
            .expect("torn tail")
            .trim_end();
        assert_eq!(parse_stream(complete).expect("valid final record"), results);
        assert!(parse_stream("{\n").is_err());
    }
    #[test]
    fn partial_timeout_report_keeps_counts_failures_and_sha() {
        let current = parse_stream(concat!(
            "{\"file\":\"a.spec.ts\",\"line\":5,\"title\":\"passes\",\"project\":\"chromium\",\"status\":\"passed\",\"retry\":0}\n",
            "{\"file\":\"a.spec.ts\",\"line\":9,\"title\":\"fails\",\"project\":\"chromium\",\"status\":\"failed\",\"retry\":0}\n",
        )).expect("valid stream");
        let report = report(&current, &[], None, "2026-11-02", 10, "abc123", Some(10));
        assert!(report.contains("abc123"));
        assert!(report.contains("incomplete (timed out after 10s)"));
        assert!(report.contains("Tests: 2 | Passed: 1 | Failed: 1"));
        assert!(report.contains(&format_result(&current[1])));
    }
    #[test]
    fn worktree_commands_are_detached_and_remove_stale_snapshot() {
        let path = std::path::Path::new("/repo/target/nightly/2026-11-02/src");
        assert_eq!(
            snapshot_args(path, "abc"),
            vec![
                "worktree",
                "add",
                "--detach",
                "/repo/target/nightly/2026-11-02/src",
                "abc"
            ]
        );
        assert_eq!(
            cleanup_args(path),
            vec![
                "worktree",
                "remove",
                "--force",
                "/repo/target/nightly/2026-11-02/src"
            ]
        );
    }
    #[test]
    fn timeout_flag_env_and_default() {
        assert_eq!(timeout_seconds(None, None).unwrap(), 7200);
        assert_eq!(timeout_seconds(None, Some("12")).unwrap(), 12);
        assert_eq!(timeout_seconds(Some(3), Some("12")).unwrap(), 3);
        assert!(timeout_seconds(None, Some("0")).is_err());
    }
    fn fixture() -> Vec<TestResult> {
        parse_results(include_str!("nightly_fixtures/results.json")).expect("valid fixture")
    }
    #[test]
    fn parses_nested_results_and_retry_outcomes() {
        let results = fixture();
        assert_eq!(
            results.iter().map(|r| &r.outcome).collect::<Vec<_>>(),
            vec![
                &Outcome::Passed,
                &Outcome::Flaky,
                &Outcome::Failed,
                &Outcome::Skipped
            ]
        );
        assert_eq!(
            (
                &results[0].file[..],
                results[0].line,
                &results[0].title[..],
                &results[0].project[..]
            ),
            ("a.spec.ts", 5, "passes", "chromium")
        );
        assert!(parse_results("{}").is_err());
        for status in ["timedOut", "interrupted"] {
            let input = include_str!("nightly_fixtures/results.json").replace("failed", status);
            assert_eq!(parse_results(&input).expect("valid status"), results);
        }
    }
    #[test]
    fn known_entries_require_valid_tickets() {
        assert_eq!(
            parse_known_failures("# entries\nfirefox * GUZ-1209\n* a.spec.ts:5 GUZ-1210\n")
                .expect("valid known failures")
                .len(),
            2
        );
        for entry in [
            "firefox *",
            "safari * GUZ-1",
            "firefox a.spec.ts GUZ-1",
            "firefox :5 GUZ-1",
            "firefox a.spec.ts:0 GUZ-1",
            "firefox a.spec.ts:x GUZ-1",
            "firefox * GUZ-",
            "firefox * GUZ-x",
        ] {
            assert!(parse_known_failures(entry).is_err(), "{entry}");
        }
    }
    #[test]
    fn classifies_failures_fixed_and_stale() {
        let current = fixture();
        let known = parse_known_failures("webkit a.spec.ts:9 GUZ-1\nchromium a.spec.ts:5 GUZ-2")
            .expect("valid known failures");
        let mut previous = current.clone();
        previous[0].outcome = Outcome::Failed;
        let c = classify(&current, &known, Some(&previous));
        assert_eq!(
            (
                c.new_failures.len(),
                c.known_failures.len(),
                c.flaky.len(),
                c.fixed.len(),
                c.stale.len()
            ),
            (0, 1, 1, 1, 1)
        );
        assert_eq!(c.known_failures[0].1, "GUZ-1");
        let c = classify(&current, &[], None);
        assert_eq!(
            (c.new_failures.len(), c.known_failures.len(), c.fixed.len()),
            (1, 0, 0)
        );
    }
    #[test]
    fn matches_wildcards_without_marking_failed_or_skipped_entries_stale() {
        let current = fixture();
        let mut failure = current[2].clone();
        failure.file = "/tmp/a.spec.ts".into();
        for entry in [
            "* * GUZ-1",
            "webkit * GUZ-1",
            "* a.spec.ts:9 GUZ-1",
            "webkit a.spec.ts:* GUZ-1",
        ] {
            let known = parse_known_failures(entry).expect("valid wildcard");
            assert!(matches(&known[0], &failure), "{entry}");
            assert_eq!(
                classify(&[failure.clone()], &known, None)
                    .known_failures
                    .len(),
                1
            );
        }
        let known = parse_known_failures("firefox * GUZ-1").expect("valid browser");
        assert!(!matches(&known[0], &failure));
        let known = parse_known_failures("* a.spec.ts:* GUZ-1").expect("valid wildcard");
        for result in [&current[2], &current[3]] {
            assert!(
                classify(&[current[0].clone(), result.clone()], &known, None)
                    .stale
                    .is_empty()
            );
        }
    }
    #[test]
    fn renders_report_and_bounded_summary() {
        let rows = vec![fixture()[2].clone(); 100];
        let c = Classification {
            new_failures: rows.clone(),
            known_failures: vec![(fixture()[2].clone(), "GUZ-1".into()); 100],
            flaky: rows.clone(),
            fixed: rows,
            stale: vec![parse_known_failures("* * GUZ-1").expect("valid wildcard")[0].clone(); 100],
        };
        let meta = Meta {
            date: "2026-11-02".into(),
            duration: 10,
            count: 100,
            passed: 0,
            failed: 100,
            flaky: 0,
            skipped: 0,
        };
        assert!(render_report(&c, &meta).contains("2026-11-02"));
        let summary = render_summary(&c, &meta);
        assert!(summary.lines().count() <= 50);
        assert_eq!(summary.matches("... and 93 more").count(), 5);
        for title in [
            "New failures",
            "Known failures",
            "Flaky",
            "Fixed since previous",
            "Stale known failures",
        ] {
            assert!(summary.contains(&format!("## {title} (100)")), "{title}");
        }
    }
}
