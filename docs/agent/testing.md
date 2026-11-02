# Testing policy

Workers never launch browsers or run `cargo xtask e2e` inside tickets. Back changes with unit and integration tests.

## Test layers

| Layer | Use |
|-------|-----|
| Rust unit tests | Pure logic and isolated behaviour. Run the relevant test with `cargo test -p <crate> --lib <test_name>`. |
| Rust integration tests | API and SQLite behaviour in `crates/api/tests/`. Run with `cargo test -p quillai_api --test <test_file> <test_name>`. |
| Bun unit tests | Frontend logic in `app/src/*.test.ts` and subdirectories. Run by `cargo xtask lint`; target a file with `bun test src/<path>.test.ts` from `app/`. |
| Component/DOM tests | Planned in GUZ-1207; not available yet. |
| Playwright E2E specs | Real-browser behaviour. Nightly only (`cargo xtask nightly`). |

## Red/green

1. Write a unit or integration test before implementation.
2. Run it and confirm it fails for the intended reason. Record the command and failing line in the ticket report.
3. Implement the change, then run the test again and confirm it passes.
4. Run `cargo xtask lint` before committing.

## Browser regression specs

Every ticket fixing or changing browser-visible behaviour still ships with an E2E regression spec. Examples include layout, focus, and real-browser quirks. Write the spec, register it in the `e2e/playwright.config.ts` allowlist, and document its coverage in [e2e-specs.md](e2e-specs.md). Listing specs with Playwright's `--list` is optional; do not launch browsers inside the ticket.

Run `cargo xtask nightly` to test all three browsers in the sandbox. Reports are written to `target/nightly/YYYY-MM-DD/report.md`; raw results and logs are in the same directory. Test failures do not make the command fail; a missing or invalid result does.

The tracked `e2e/known-failures.txt` uses `browser spec:line ticket`, with `*` for any browser, entire spec, or line (for example, `firefox * GUZ-1209` or `* example.spec.ts:* GUZ-123`). Each entry requires a `GUZ-` ticket number. Lines starting with `#` and blank lines are ignored. Scheduling is tracked in GUZ-1214.

## Commit gate

`cargo xtask lint` must print `All 11 checks passed.` before commit. Run `cargo xtask install-hooks` once per clone to install the tracked pre-commit hook; worktrees share this configuration. The installer refuses to replace an existing `core.hooksPath` other than `.githooks`.

The hook runs the full lint on the working tree, not only staged files. If `app/node_modules` is missing, run `bun install --frozen-lockfile` in `app/` first. Never use `--no-verify`.
