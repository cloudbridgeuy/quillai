# Testing policy

Workers never launch browsers or run `cargo xtask e2e` inside tickets. Back changes with unit and integration tests.

## Test layers

| Layer | Use |
|-------|-----|
| Rust unit tests | Pure logic and isolated behaviour. Run the relevant test with `cargo test -p <crate> --lib <test_name>`. |
| Rust integration tests | API and SQLite behaviour in `crates/api/tests/`. Run with `cargo test -p quillai_api --test <test_file> <test_name>`. |
| Bun unit tests | Frontend logic in `app/src/*.test.ts` and subdirectories. Run by `cargo xtask lint`; target a file with `bun test src/<path>.test.ts` from `app/`. |
| Component/DOM tests | Rendering, clicks and state; use E2E for layout, sizes, focus quirks and real-browser behaviour. |
| UI stories | CSF3 `app/src/ui/*.stories.tsx` (args/render/play/name, `import type` from Storybook, plain-DOM play). `app/src/ui/stories.dom.test.tsx` renders every story and runs its play in `bun test`. Storybook lives in `storybook/` (own `package.json`; `bun install` there, then `bun run dev` or `bun run build`); lint check `storybook-tsc` type-checks the stories with `storybook/node_modules/typescript`. Tokens page: `storybook/Tokens.stories.tsx` reads `tokens.css`. New or changed primitives get a story first ([design-system.md](design-system.md)). |
| Playwright E2E specs | Real-browser behaviour. Nightly only (`cargo xtask nightly`). |

## Red/green

1. Write a unit or integration test before implementation.
2. Run it and confirm it fails for the intended reason. Record the command and failing line in the ticket report.
3. Implement the change, then run the test again and confirm it passes.
4. Run `cargo xtask lint` before committing.

## Browser regression specs

Every ticket fixing or changing browser-visible behaviour still ships with an E2E regression spec. Examples include layout, focus, and real-browser quirks. Write the spec, register it in the `e2e/playwright.config.ts` allowlist, and document its coverage in [e2e-specs.md](e2e-specs.md). Listing specs with Playwright's `--list` is optional; do not launch browsers inside the ticket.

Run `cargo xtask nightly` to test Chromium and WebKit in the sandbox. Firefox is skipped while GUZ-1209 is blocked upstream because it cannot start on macOS 27.2. The report names the skipped project and ticket; its known failures are not marked stale or fixed. Nightly freezes the local `main` commit in a detached worktree and records its SHA in `target/nightly/YYYY-MM-DD/report.md`. Results stream to `results.jsonl`, with report checkpoints, `run.log`, and `provider-audit.log` in the same directory. Failed and flaky results include the first error (up to 500 characters), assertion location, and trace/error-context paths; artifacts remain in `target/nightly/YYYY-MM-DD/artifacts` after snapshot cleanup. The default timeout is 10800 seconds (3 hours); use `--timeout SECONDS` or `QUILLAI_NIGHTLY_TIMEOUT` to change it. Timeout and incomplete results fail the command and retain partial results. Test failures alone do not fail the command. A later run removes stale snapshot worktrees left by forced termination.

Nightly API warnings and errors are in `target/nightly/YYYY-MM-DD/api.log`, linked from the report.

The tracked `e2e/known-failures.txt` uses `browser spec:line ticket`, with `*` for any browser, entire spec, or line (for example, `firefox * GUZ-1209` or `* example.spec.ts:* GUZ-123`). Each entry requires a `GUZ-` ticket number. Lines starting with `#` and blank lines are ignored. On macOS, run `cargo xtask install-nightly` from the main folder. At 02:00 local time, launchd tests the local `main` commit at run time. launchd runs a missed job when the Mac wakes. Run `cargo xtask install-nightly --uninstall` to remove the job. The installer writes `~/Library/LaunchAgents/com.quillai.nightly.plist`; launchd logs are `target/nightly/launchd.out.log` and `target/nightly/launchd.err.log`. Report delivery to the ralph inbox is tracked in GUZ-1208.

## Commit gate

`cargo xtask lint` must print `All 12 checks passed.` before commit. Run `cargo xtask install-hooks` once per clone to install the tracked pre-commit hook; worktrees share this configuration. The installer refuses to replace an existing `core.hooksPath` other than `.githooks`.

The `i18n` check scans tracked and untracked, non-ignored `app/src/**/*.tsx`, excluding `*.test.tsx`, `*.dom.test.tsx`, and `*.fixture.tsx`. It counts JSX text children containing letters and literal values in `aria-label`, `title`, `placeholder`, `label`, `alt`, `description`, and `heading`, including braces-wrapped literals and nested ternary branches. Template literals count when their static text contains letters. Call arguments, comparison operands, and other dynamic values are ignored; JSX nested inside expressions is scanned. Each text node, literal branch, or template counts once, including multiline text. Use the message helper and `app/src/i18n/messages.ts` for new UI strings.

`xtask/i18n-waivers.json` records existing counts by file. New offenders and count increases fail. Decreases also fail with instructions to lower the count or remove a zero-count entry. Never raise counts or add waivers for new strings. Full diagnostics are in `target/xtask-lint/i18n.log`.

The hook runs the full lint on the working tree, not only staged files. If `app/node_modules` is missing, run `bun install --frozen-lockfile` in `app/` first. Never use `--no-verify`.

Visual tests: nightly builds Storybook in the snapshot (`bun run build` in `storybook/`) and `e2e/stories-visual.spec.ts` screenshots every story against baselines in `e2e/stories-visual.spec.ts-snapshots/`. A failed build is named at the top of the report and the other specs still run. Update baselines with `cargo xtask e2e stories-visual.spec.ts --update-snapshots` after running `bun run build` in `storybook/`.
