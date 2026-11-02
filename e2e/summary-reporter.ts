import { appendFileSync, writeFileSync } from "node:fs";
import { dirname, relative } from "node:path";
import { stripVTControlCharacters } from "node:util";
import type { FullConfig, FullResult, Reporter, Suite, TestCase, TestResult } from "../app/node_modules/@playwright/test/reporter";

const MAX_ERROR_LINES = 20;

function duration(milliseconds: number): string {
  const seconds = milliseconds / 1000;
  return seconds < 60 ? `${seconds.toFixed(1)}s` : `${(seconds / 60).toFixed(1)}m`;
}

export default class SummaryReporter implements Reporter {
  private root = "";
  private suite!: Suite;

  onBegin(config: FullConfig, suite: Suite) {
    this.root = dirname(config.rootDir);
    this.suite = suite;
  }

  onTestEnd(test: TestCase, result: TestResult) {
    const file = relative(this.root, test.location.file);
    const project = test.parent.project()?.name ?? "";
    const target = process.env.QUILLAI_E2E_RESULTS_JSONL;
    if (target) {
      appendFileSync(target, `${JSON.stringify({ file, line: test.location.line, title: test.title, project, status: result.status, retry: result.retry })}\n`);
    }
    console.log(`${file}:${test.location.line} [${project}] ${result.status} ${duration(result.duration)}`);
  }

  onEnd(result: FullResult) {
    const lines: string[] = [];
    const counts = { passed: 0, failed: 0, flaky: 0, skipped: 0 };
    for (const test of this.suite.allTests()) {
      const outcome = test.outcome();
      if (outcome === "expected") counts.passed++;
      else if (outcome === "flaky") counts.flaky++;
      else if (outcome === "skipped") counts.skipped++;
      else {
        counts.failed++;
        const last = test.results[test.results.length - 1];
        lines.push(`FAILED ${relative(this.root, test.location.file)}:${test.location.line}`);
        lines.push(`  ${test.titlePath().filter(Boolean).join(" › ")}`);
        for (const error of last?.errors ?? []) {
          const message = stripVTControlCharacters(error.message ?? error.stack ?? "").split("\n");
          for (const line of message.slice(0, MAX_ERROR_LINES)) lines.push(`    ${line}`);
          if (message.length > MAX_ERROR_LINES) lines.push("    ...");
        }
        const paths = new Set(
          (last?.attachments ?? []).flatMap((attachment) =>
            attachment.path ? [relative(this.root, attachment.path)] : [],
          ),
        );
        for (const path of paths) lines.push(`  artifact: ${path}`);
      }
    }
    lines.push(
      `${counts.passed} passed, ${counts.failed} failed, ${counts.flaky} flaky, ${counts.skipped} skipped (${duration(result.duration)})`,
    );
    const target = process.env.QUILLAI_E2E_SUMMARY;
    if (target) writeFileSync(target, `${lines.join("\n")}\n`);
  }
}
