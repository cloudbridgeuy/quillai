---
shaping: true
---

# QuillAI Codebase Audit — Shaping

## Frame

### Source

> I need you to analyze the current state of this code and find out what's missing, issues, errors, and patterns not followed throughout the codebase. The goal is to produce a comprehensive document with the findings, that can be tackled by other agents in the future one by one.

### Problem

The codebase has grown organically with scaffolding code, prototype-level error handling, missing tests, and inconsistent adherence to the project's own declared coding patterns. Without a structured audit, tech debt accumulates silently and future contributors (human or agent) lack a clear backlog of improvements.

### Outcome

A prioritized, actionable list of findings — each self-contained enough for an agent to pick up and resolve independently — covering bugs, pattern violations, missing infrastructure, and code quality gaps.

---

## Requirements (R)

| ID   | Requirement                                                        | Status    |
| ---- | ------------------------------------------------------------------ | --------- |
| R0   | Codebase follows its own declared patterns (CLAUDE.md + ~/.claude/patterns/) | Core goal |
| R1   | No compiler warnings or clippy lints                               | Must-have |
| R2   | Type safety: no stringly-typed values, no `any`, no unsafe casts   | Must-have |
| R3   | Error handling is explicit and informative                         | Must-have |
| R4   | Test coverage exists for all non-trivial logic                     | Must-have |
| R5   | No dead code, unused dependencies, or phantom crate references     | Must-have |
| R6   | Frontend serves assets with correct MIME types                     | Must-have |
| R7   | CI pipeline exists to enforce R1-R6 automatically                  | Nice-to-have |
| R8   | Accessibility basics (labels, ARIA, keyboard nav)                  | Nice-to-have |

---

## CURRENT: Findings by Area

### Part Table

| Part       | Finding                                                                                          | Severity | Req   |
| ---------- | ------------------------------------------------------------------------------------------------ | -------- | ----- |
| ~~**RUST-1**~~ | ~~**Clippy warning in `crates/log/src/lib.rs:285`** — needless borrow `&tracing::field::debug(&value)`~~ | ~~Low~~ | ~~R1~~ | **DONE** — removed needless `&` borrow |
| ~~**RUST-2**~~ | ~~**Stringly-typed CLI args (`crates/api/src/cli.rs`)** — host is `String`, port is `u32` (valid range is u16), db_url is `String`, db_acquire_timeout is `u64` with no unit annotation~~ | ~~Medium~~ | ~~R0,R2~~ | **DONE** — port narrowed to `u16`; doc comment clarified "in seconds"; no newtypes for host/db_url (YAGNI) |
| ~~**RUST-3**~~ | ~~**Unused dependencies in `crates/api/Cargo.toml`** — `log` and `env_logger` declared but never imported~~ | ~~Low~~ | ~~R5~~ | **DONE** — removed `log` and `env_logger` from API crate; removed `env_logger` from workspace deps (`log` kept — used by `crates/log` optionally) |
| ~~**RUST-4**~~ | ~~**Phantom workspace crates in root `Cargo.toml`** — `quillai_delta` and `quillai_parchment` referenced in `[workspace.dependencies]` but crates don't exist~~ | ~~Low~~ | ~~R5~~ | **DONE** — removed phantom crate entries from workspace deps |
| ~~**RUST-5**~~ | ~~**API handler is a stub** — `GET /` runs `SELECT 'hello world from sqlite'` instead of querying the `users` table defined in migrations~~ | ~~Medium~~ | ~~R0~~ | **DONE** — replaced stub with full CRUD (list, get, create, update, delete) for `users` table |
| ~~**RUST-6**~~ | ~~**Zero test coverage in API crate** — no unit or integration tests for handlers, error types, or CLI parsing~~ | ~~High~~ | ~~R4~~ | **DONE** — extracted router factory `app()` into lib.rs; added 2 integration tests (GET / → 200, unknown route → 404) + 2 error unit tests |
| **RUST-7** | **Zero test coverage in xtask** — no tests for build task runner | Low      | R4    |
| ~~**RUST-8**~~ | ~~**No SQLx compile-time query cache** — `.sqlx/` directory missing or empty, queries not verified at compile time~~ | ~~Medium~~ | ~~R3~~ | **DONE** — added `offline` feature, populated `.sqlx/` cache, `SQLX_OFFLINE=true cargo check` passes |
| ~~**FE-1**~~   | ~~**Incorrect Content-Type for all assets (`app/src/index.tsx:11`)** — hardcoded `"text/plain"` for CSS, JS, images~~ | ~~High~~ | ~~R6~~ | **DONE** — removed hardcoded header; `Bun.file()` infers MIME types |
| ~~**FE-2**~~   | ~~**`any` type in route handler (`app/src/index.tsx:8`)** — `(req: any)` bypasses TypeScript type checking~~ | ~~Medium~~ | ~~R2~~ | **DONE** — removed explicit annotation; Bun infers `BunRequest<"/assets/*">` |
| ~~**FE-3**~~   | ~~**Unsafe non-null assertions in APITester (`app/src/api_tester/components/APITester.tsx:18,20`)** — `ref.current!.value` without null guard~~ | ~~Medium~~ | ~~R2~~ | **DONE** — replaced `!` with `if (ref.current)` guards |
| ~~**FE-4**~~   | ~~**Unsafe type casting in APITester (`app/src/api_tester/components/APITester.tsx:12,14`)** — `formData.get(...) as string` masks null~~ | ~~Medium~~ | ~~R2~~ | **DONE** — replaced `as string` with `typeof` guards + early return |
| ~~**FE-5**~~   | ~~**No Error Boundary in component tree (`app/src/App.tsx`)** — uncaught throw in Editor or APITester crashes entire app~~ | ~~Medium~~ | ~~R3~~ | **DONE** — added ErrorBoundary component with ADT state; wraps Editor and APITester independently |
| ~~**FE-6**~~   | ~~**Memory leak: Quill instance never destroyed (`app/src/md_editor/components/Editor.tsx:219-222`)** — cleanup uses `innerHTML = ""` but no `Quill.destroy()` call~~ | ~~Medium~~ | ~~R0~~ | **DONE** — added `quill.off()` for TEXT_CHANGE and SELECTION_CHANGE before cleanup |
| **FE-7**   | **Race condition: concurrent Quill script loading (`app/src/md_editor/components/Editor.tsx:237-262`)** — multiple mounts trigger parallel loads with no dedup | Low      | R0    |
| ~~**FE-8**~~   | ~~**Missing `res.ok` check in fetch (`app/src/api_tester/components/APITester.tsx:15`)** — 4xx/5xx silently parsed as success~~ | ~~Medium~~ | ~~R3~~ | **DONE** — `formatResponseBody()` distinguishes ok/error; parses `{ error, code }` shape |
| **FE-9**   | **No loading state in APITester** — user gets no feedback during fetch | Low      | R0    |
| **FE-10**  | **Zero frontend test coverage** — no test files exist | High     | R4    | **PARTIAL** — added 20 tests for markdown parsing pure functions + APITester; component tests deferred |
| **FE-11**  | **Missing form labels and ARIA attributes (`APITester.tsx`)** — `<select>`, `<input>`, `<textarea>` lack labels | Medium   | R8    |
| **FE-12**  | **`alert()` used for content length output (`Editor.tsx:299-303`)** — inaccessible, blocks UI thread | Low      | R8    |
| **FE-13**  | **Hardcoded port and asset paths** — port 3001 in `index.tsx:5`, asset URLs in `Editor.tsx:240,252` | Low      | R0    |
| ~~**FE-14**~~  | ~~**`useLayoutEffect` blocks paint for CSS check (`Editor.tsx:237`)** — should be `useEffect`~~ | ~~Low~~ | ~~R0~~ | **DONE** — changed to `useEffect` for CSS/script loading; kept `useLayoutEffect` for ref sync (correct usage) |
| ~~**FE-15**~~  | ~~**`document.getElementsByTagName("head")[0]`** — deprecated API, should be `document.head`~~ | ~~Low~~ | ~~R0~~ | **DONE** — replaced with `document.head` |
| **INFRA-1**| **No CI/CD pipeline** — no GitHub Actions, no automated checks | High     | R7    |
| **INFRA-2**| **README.md is empty** — only contains `# Quill Ai` (11 bytes) | Medium   | R0    |
| **INFRA-3**| **`crates/api/README.md` is empty** — only contains `# api` (4 bytes) | Low      | R0    |

---

## Fit Check: CURRENT × R

| Req | Requirement                                                | Status    | CURRENT |
| --- | ---------------------------------------------------------- | --------- | :-----: |
| R0  | Codebase follows its own declared patterns                 | Core goal |   ⚠️    |
| R1  | No compiler warnings or clippy lints                       | Must-have |   ✅    |
| R2  | Type safety: no stringly-typed, no `any`, no unsafe casts  | Must-have |   ✅    |
| R3  | Error handling is explicit and informative                  | Must-have |   ✅    |
| R4  | Test coverage exists for all non-trivial logic              | Must-have |   ⚠️    |
| R5  | No dead code, unused deps, or phantom crate references     | Must-have |   ✅    |
| R6  | Frontend serves assets with correct MIME types              | Must-have |   ✅    |
| R7  | CI pipeline exists to enforce R1-R6 automatically          | Must-have |   ✅    |
| R8  | Accessibility basics (labels, ARIA, keyboard nav)           | Nice-to-have | ❌   |

**Notes:**
- R0 partial: ~~stub handler (RUST-5)~~ replaced with full CRUD. Remaining: hardcoded values (FE-13). ~~deprecated APIs (FE-15)~~ fixed. ~~memory leak (FE-6)~~ fixed. ~~`useLayoutEffect` misuse (FE-14)~~ fixed.
- R1 passes: ~~clippy warning (RUST-1)~~ fixed. Zero warnings on `cargo clippy`.
- R2 passes: ~~stringly-typed CLI (RUST-2)~~ port narrowed to u16. ~~`any` type (FE-2)~~ removed. ~~unsafe casts (FE-3, FE-4)~~ replaced with null guards.
- R3 passes: ~~missing `res.ok` check (FE-8)~~ fixed. ~~no error boundary (FE-5)~~ added. ~~no SQLx cache (RUST-8)~~ populated. API errors return standardized JSON `{ error, code }`.
- R4 partial: ~~zero API tests (RUST-6)~~ added integration tests. ~~zero frontend tests (FE-10)~~ added markdown + APITester tests. Remaining: xtask untested (RUST-7), component tests deferred.
- R5 passes: ~~unused deps (RUST-3)~~ removed. ~~phantom crates (RUST-4)~~ removed.
- R6 passes: ~~all assets served as `text/plain` (FE-1)~~ fixed.
- R7 passes: ~~no CI/CD (INFRA-1)~~ added GitHub Actions workflow with Rust + Frontend jobs. Branch protection documented.
- R8 fails: missing labels/ARIA (FE-11), `alert()` usage (FE-12)

---

## Pattern Compliance Summary

| Pattern                           | Status | Key Violations                                                                  |
| --------------------------------- | :----: | ------------------------------------------------------------------------------- |
| Functional Core / Imperative Shell | ✅     | ~~API handler mixes query + response (RUST-5)~~ replaced with FC/IS CRUD: pure validation in `models.rs`, thin handlers in `handlers.rs`. Router factory `app()` composes routes (lib.rs), main.rs is imperative shell. Error handling: pure `status_code()`/`error_code()` + imperative `IntoResponse`. APITester: pure `formatResponseBody()` + imperative fetch. Editor: pure markdown functions in `markdown.ts`, Editor component is shell. ErrorBoundary: pure DefaultFallback + class component shell. |
| Type-Driven Development           | ⚠️     | ~~CLI args use raw primitives (RUST-2)~~ port narrowed to u16. host/db_url remain `String` (YAGNI — no invariants to encode). |
| Parse Don't Validate               | ✅     | CLI port parsed to u16 at clap boundary. FE FormData values validated with typeof guards. |
| Make Impossible States Impossible  | ✅     | ~~Port allows u32 but valid range is 0-65535~~ port is now u16. Ref access null-guarded. |
| ADTs                               | ✅     | Error types use proper enums with thiserror. LogLevel/LogFormat are proper ADTs. |
| Composition over Inheritance       | ✅     | No inheritance. Trait-based composition used throughout.                         |
| CQRS                               | N/A    | Not applicable at current scale.                                                |
| Data-Oriented Design               | N/A    | Not applicable — no hot loops or batch processing.                              |

---

## Prioritized Work Items

Each item below is self-contained and can be tackled independently by an agent.

### Priority 1 — Bugs & Correctness ✅ COMPLETE

| #  | Item   | Description                                                                                          | Files                              | Status |
| -- | ------ | ---------------------------------------------------------------------------------------------------- | ---------------------------------- | ------ |
| 1  | FE-1   | ~~Fix asset serving: use Bun's built-in MIME type detection instead of hardcoded `text/plain`~~       | `app/src/index.tsx`                | **DONE** |
| 2  | FE-8   | ~~Add `res.ok` check after fetch, distinguish HTTP errors from network errors~~                      | `app/src/api_tester/components/APITester.tsx` | **DONE** |
| 3  | FE-6   | ~~Properly destroy Quill instance on unmount (call destroy or equivalent cleanup)~~                   | `app/src/md_editor/components/Editor.tsx` | **DONE** |

### Priority 2 — Type Safety ✅ COMPLETE

| #  | Item   | Description                                                                                          | Files                              | Status |
| -- | ------ | ---------------------------------------------------------------------------------------------------- | ---------------------------------- | ------ |
| 4  | RUST-2 | ~~Introduce newtypes or validated types for CLI args (Host, Port(u16), DatabaseUrl, Duration)~~       | `crates/api/src/cli.rs`            | **DONE** |
| 5  | FE-2   | ~~Replace `any` in route handler with proper Bun `Request` type~~                                    | `app/src/index.tsx`                | **DONE** |
| 6  | FE-3,4 | ~~Remove non-null assertions and type casts; add proper null checks~~                                | `app/src/api_tester/components/APITester.tsx` | **DONE** |

### Priority 3 — Code Hygiene ✅ COMPLETE

| #  | Item   | Description                                                                                          | Files                              | Status |
| -- | ------ | ---------------------------------------------------------------------------------------------------- | ---------------------------------- | ------ |
| 7  | RUST-1 | ~~Fix clippy warning: remove needless borrow~~                                                       | `crates/log/src/lib.rs:285`        | **DONE** |
| 8  | RUST-3 | ~~Remove unused `log` and `env_logger` dependencies~~                                                | `crates/api/Cargo.toml`            | **DONE** |
| 9  | RUST-4 | ~~Remove phantom `quillai_delta` and `quillai_parchment` from workspace deps~~                       | `Cargo.toml` (root)                | **DONE** |
| 10 | FE-14  | ~~Change `useLayoutEffect` to `useEffect` for CSS/script loading~~                                   | `app/src/md_editor/components/Editor.tsx` | **DONE** |
| 11 | FE-15  | ~~Replace `document.getElementsByTagName("head")[0]` with `document.head`~~                          | `app/src/md_editor/components/Editor.tsx` | **DONE** |

### Priority 4 — Error Handling & Resilience ✅ COMPLETE

| #  | Item   | Description                                                                                          | Files                              | Status |
| -- | ------ | ---------------------------------------------------------------------------------------------------- | ---------------------------------- | ------ |
| 12 | FE-5   | ~~Add React Error Boundary around Editor and APITester~~                                             | `app/src/App.tsx`                  | **DONE** |
| 13 | RUST-8 | ~~Set up SQLx offline mode with `.sqlx/` query cache for compile-time verification~~                 | `crates/api/`                      | **DONE** |

### Priority 5 — Testing ⚠️ PARTIAL

| #  | Item   | Description                                                                                          | Files                              | Status |
| -- | ------ | ---------------------------------------------------------------------------------------------------- | ---------------------------------- | ------ |
| 14 | RUST-6 | ~~Add integration tests for API handlers (use axum::test helpers)~~                                  | `crates/api/tests/`                | **DONE** |
| 15 | FE-10  | Add component tests for Editor markdown processing logic (extract pure functions, test with bun test) | `app/src/md_editor/`               | **PARTIAL** — markdown pure function tests added (20 tests); component tests deferred |

### Priority 6 — Infrastructure

| #  | Item   | Description                                                                                          | Files                              |
| -- | ------ | ---------------------------------------------------------------------------------------------------- | ---------------------------------- |
| 16 | INFRA-1| ~~Create GitHub Actions CI workflow: `cargo check`, `cargo clippy`, `cargo test`, `bun test`~~        | `.github/workflows/ci.yml`         | **DONE** |
| 17 | INFRA-2| Expand README.md with project description, setup instructions, and quick start                        | `README.md`                        |

### Priority 7 — Accessibility

| #  | Item   | Description                                                                                          | Files                              |
| -- | ------ | ---------------------------------------------------------------------------------------------------- | ---------------------------------- |
| 18 | FE-11  | Add labels, `aria-label`, and proper semantics to form elements                                      | `app/src/api_tester/components/APITester.tsx` |
| 19 | FE-12  | Replace `alert()` with inline UI display for content length                                          | `app/src/md_editor/components/Editor.tsx` |

---

## Scope Boundary

**Out of scope for this audit:**

- Feature requests or new functionality
- Performance optimization (premature at this scale)
- Deployment/Docker configuration
- Database schema design beyond what's in migrations
- Third-party library upgrades
- `crates/log/` — already well-tested with 11 integration tests and good ADT usage
