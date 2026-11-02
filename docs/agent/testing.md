# Testing policy

Workers never launch browsers or run `cargo xtask e2e` inside tickets. Back changes with unit and integration tests.

## Test layers

| Layer | Use |
|-------|-----|
| Rust unit tests | Pure logic and isolated behaviour. Run the relevant test with `cargo test -p <crate> --lib <test_name>`. |
| Rust integration tests | API and SQLite behaviour in `crates/api/tests/`. Run with `cargo test -p quillai_api --test <test_file> <test_name>`. |
| Bun unit tests | Frontend logic in `app/src/*.test.ts` and subdirectories. Run by `cargo xtask lint`; target a file with `bun test src/<path>.test.ts` from `app/`. |
| Component/DOM tests | Planned in GUZ-1207; not available yet. |
| Playwright E2E specs | Real-browser behaviour. Nightly only (GUZ-1206, not built yet). |

## Red/green

1. Write a unit or integration test before implementation.
2. Run it and confirm it fails for the intended reason. Record the command and failing line in the ticket report.
3. Implement the change, then run the test again and confirm it passes.
4. Run `cargo xtask lint` before committing.

## Browser regression specs

Every ticket fixing or changing browser-visible behaviour still ships with an E2E regression spec. Examples include layout, focus, and real-browser quirks. Write the spec, register it in the `e2e/playwright.config.ts` allowlist, and document its coverage in [e2e-specs.md](e2e-specs.md). Listing specs with Playwright's `--list` is optional; do not launch browsers inside the ticket.

The nightly run (GUZ-1206) will classify failures against a tracked known-failures list. Every entry must name a ticket. Its report will go to the ralph inbox for next-day triage.

## Commit gate

`cargo xtask lint` must print `All 11 checks passed.` before commit. Run `cargo xtask install-hooks` once per clone to install the tracked pre-commit hook; worktrees share this configuration. The installer refuses to replace an existing `core.hooksPath` other than `.githooks`.

The hook runs the full lint on the working tree, not only staged files. If `app/node_modules` is missing, run `bun install --frozen-lockfile` in `app/` first. Never use `--no-verify`.
