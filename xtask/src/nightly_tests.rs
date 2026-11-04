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
    assert_eq!(timeout_seconds(None, None).unwrap(), 10800);
    assert_eq!(timeout_seconds(None, Some("12")).unwrap(), 12);
    assert_eq!(timeout_seconds(Some(3), Some("12")).unwrap(), 3);
    assert!(timeout_seconds(None, Some("0")).is_err());
}
#[test]
fn nightly_selects_only_chromium_and_webkit() {
    let args = playwright_args(
        std::path::Path::new("/repo/snapshot"),
        std::path::Path::new("/repo/report"),
    );
    assert_eq!(
        args.iter()
            .filter(|arg| arg.starts_with("--project"))
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["--project=chromium", "--project=webkit"]
    );
    assert!(args.contains(&"--retries=1".into()));
    assert!(args.contains(&"--output=/repo/report/artifacts".into()));
    assert!(args.contains(&"--reporter=/repo/snapshot/e2e/summary-reporter.ts,json".into()));
}
#[test]
fn report_names_skipped_firefox_and_ticket() {
    let text = report(&[], &[], None, "2026-11-04", 0, "abc", None);
    assert!(
        text.lines().any(|line| {
            let line = line.to_lowercase();
            line.contains("firefox") && line.contains("skipped") && line.contains("guz-1209")
        }),
        "report must name skipped Firefox and GUZ-1209: {text}"
    );
}
#[test]
fn skipped_project_known_failure_is_neither_stale_nor_fixed() {
    let mut result = fixture()[0].clone();
    result.project = "firefox".into();
    let current = [result.clone()];
    result.outcome = Outcome::Failed;
    let known = parse_known_failures("firefox * GUZ-1209").unwrap();
    for current in [&[][..], &current[..]] {
        let c = classify(current, &known, Some(std::slice::from_ref(&result)));
        assert_eq!(
            (c.stale.len(), c.fixed.len()),
            (0, 0),
            "skipped Firefox must not be stale or fixed"
        );
    }
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
        let input = include_str!("nightly_fixtures/results.json")
            .replace("\"status\":\"failed\"", &format!("\"status\":\"{status}\""));
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

#[test]
fn first_failure_details_survive_retry() {
    for result in [fixture()[1].clone(), fixture()[2].clone()] {
        let row = format_result(&result);
        assert!(
            row.contains(
                "Error: expect(locator).toBeVisible() failed Expected: visible Received: hidden"
            ),
            "{row}"
        );
        assert!(row.contains("a.spec.ts:42"), "{row}");
        assert!(row.contains("/artifacts/failed/trace.zip"), "{row}");
        assert!(row.contains("/artifacts/failed/error-context.md"), "{row}");
    }
}
#[test]
fn stream_retains_first_failure_details_after_pass() {
    let input = concat!(
        "{\"file\":\"a.spec.ts\",\"line\":5,\"title\":\"test\",\"project\":\"chromium\",\"status\":\"failed\",\"retry\":0,\"error\":\"First error\",\"location\":\"a.spec.ts:42\",\"artifacts\":[\"/artifacts/trace.zip\"]}\n",
        "{\"file\":\"a.spec.ts\",\"line\":5,\"title\":\"test\",\"project\":\"chromium\",\"status\":\"passed\",\"retry\":1}\n"
    );
    let row = format_result(&parse_stream(input).unwrap()[0]);
    assert!(
        row.contains("First error")
            && row.contains("a.spec.ts:42")
            && row.contains("/artifacts/trace.zip"),
        "{row}"
    );
}
#[test]
fn report_has_single_line_error_under_new_failure() {
    let text = report(&fixture(), &[], None, "2026-11-03", 1, "abc", None);
    assert!(text.contains("  Error: Error: expect(locator).toBeVisible() failed Expected: visible Received: hidden"), "{text}");
    assert!(
        text.contains("  Artifact: /repo/target/nightly/2026-11-03/artifacts/failed/trace.zip"),
        "{text}"
    );
}

#[test]
fn parses_optional_details_without_changing_test_identity() {
    let results = fixture();
    assert_eq!(results[2].line, 9);
    assert_eq!(results[2].location.as_deref(), Some("a.spec.ts:42"));
    assert_eq!(
        results[2].error.as_deref(),
        Some("Error: expect(locator).toBeVisible() failed Expected: visible Received: hidden")
    );
    assert_eq!(results[2].artifacts.len(), 2);
    assert!(results[0].error.is_none() && results[0].location.is_none());
    assert!(results[0].artifacts.is_empty());
    let legacy = parse_stream("{\"file\":\"a.spec.ts\",\"line\":9,\"title\":\"fails\",\"project\":\"webkit\",\"status\":\"failed\",\"retry\":0}\n").unwrap();
    assert!(legacy[0].error.is_none() && legacy[0].location.is_none());
    assert!(legacy[0].artifacts.is_empty());
}

#[test]
fn stack_location_and_unicode_error_survive_later_failure() {
    let mut json: serde_json::Value =
        serde_json::from_str(include_str!("nightly_fixtures/results.json")).unwrap();
    let attempts = &mut json["suites"][0]["suites"][0]["specs"][1]["tests"][0]["results"];
    attempts[0]["errors"] = serde_json::json!([{"message": format!("{}\nsecond line", "é".repeat(600))}, {"message": "cleanup error"}]);
    attempts[0]["error"] = serde_json::json!({
        "location": {"file": "/snapshot/e2e/helper.ts", "line": 8, "column": 2},
        "stack": "Error: assertion\n    at helper (/snapshot/e2e/helper.ts:8:2)\n    at /snapshot/e2e/a.spec.ts:77:7"
    });
    attempts[1]["errors"] = serde_json::json!([{"message": "later failure"}]);
    attempts[1]["attachments"] = attempts[0]["attachments"].clone();
    attempts[1]["attachments"].as_array_mut().unwrap().push(serde_json::json!({"name": "trace", "contentType": "application/zip", "path": "/repo/target/nightly/2026-11-03/artifacts/retry/trace.zip"}));
    let result = &parse_results(&json.to_string()).unwrap()[2];
    assert_eq!(result.error.as_deref(), Some("é".repeat(500).as_str()));
    assert_eq!(result.location.as_deref(), Some("a.spec.ts:77"));
    assert_eq!(result.artifacts.len(), 3);
    let text = report(
        std::slice::from_ref(result),
        &[],
        None,
        "2026-11-03",
        1,
        "abc",
        None,
    );
    assert_eq!(
        text.lines()
            .filter(|line| line.starts_with("  Error: "))
            .count(),
        1
    );
    assert!(!text.contains("cleanup error") && !text.contains("later failure"));
}
