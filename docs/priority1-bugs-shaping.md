---
shaping: true
---

# Priority 1 — Bugs & Correctness — Shaping

## Frame

### Source

> Read @docs/codebase-audit-shaping.md and grab the items with highest priority, and let's start shaping a fix.

From the codebase audit, Priority 1 items:
- **FE-1:** All assets served as `text/plain` — broken CSS, JS, images
- **FE-8:** Missing `res.ok` check — 4xx/5xx silently parsed as success
- **FE-6:** Quill instance never destroyed on unmount — memory leak

### Problem

Three correctness bugs in the frontend: assets don't render because MIME types are wrong, the API tester silently swallows HTTP errors, and the editor leaks event listeners on unmount.

### Outcome

Assets load with correct MIME types, HTTP errors are clearly surfaced in the API tester (using a standardized error payload shape), and the Quill editor cleans up properly on unmount.

---

## Requirements (R)

| ID | Requirement | Status |
|----|-------------|--------|
| R0 | Assets served with correct MIME types (CSS, JS, images load properly) | Core goal |
| R1 | HTTP errors distinguished from successful responses in APITester | Must-have |
| R2 | Quill instance cleaned up on component unmount (no leaked listeners/DOM) | Must-have |
| R3 | Fixes are minimal — no refactoring, no feature additions beyond scope | Must-have |
| R4 | API error responses follow a standard shape: `{ "error": "<message>", "code": "<CODE>" }` | Must-have |

---

## A: Fix bugs + standardize error shape

### Part Table

| Part | Mechanism | Flag |
|------|-----------|:----:|
| **A1** | **Remove hardcoded Content-Type header** — delete the `headers` option from the `/assets/*` route; `Bun.file()` handles MIME types automatically | |
| **A2** | **Rust: JSON error responses** — drop `axum_thiserror`, add `serde`/`serde_json`, impl `IntoResponse` manually: `Json({ "error": self.to_string(), "code": self.error_code() })` with status from a `status_code(&self)` method. Two variants: `Sqlx → (500, "SQLX")`, `InvalidLogLevel → (500, "INVALID_LOG_LEVEL")` | |
| **A4** | **APITester: parse error responses** — check `res.ok`; if false, parse body as JSON expecting `{ error, code }`, display both; fall back to `status + statusText` if body doesn't match the shape | |
| **A5** | **Clean up Quill on unmount** — call `quill.off()` for both `TEXT_CHANGE` and `SELECTION_CHANGE` listeners before clearing DOM and nulling the ref | |

### Removed Parts

- ~~**A3** Bun error helper~~ — removed (YAGNI: no Bun route currently produces errors)

---

## Fit Check: R × A

| Req | Requirement | Status | A |
|-----|-------------|--------|---|
| R0 | Assets served with correct MIME types (CSS, JS, images load properly) | Core goal | ✅ |
| R1 | HTTP errors distinguished from successful responses in APITester | Must-have | ✅ |
| R2 | Quill instance cleaned up on component unmount (no leaked listeners/DOM) | Must-have | ✅ |
| R3 | Fixes are minimal — no refactoring, no feature additions beyond scope | Must-have | ✅ |
| R4 | API error responses follow standard shape: `{ "error", "code" }` | Must-have | ✅ |

---

## Slices

| Slice | Parts | Demo | Files |
|-------|-------|------|-------|
| **V1** | A1 | Load app → CSS styled, JS executes, images render | `app/src/index.tsx` |
| **V2** | A2 | Hit a failing Rust endpoint → get `{ "error", "code" }` JSON with correct status code | `crates/api/src/error.rs`, `crates/api/Cargo.toml` |
| **V3** | A4 | APITester: send request to bad endpoint → error displayed with message and code | `app/src/api_tester/components/APITester.tsx` |
| **V4** | A5 | Mount/unmount Editor → no leaked event listeners | `app/src/md_editor/components/Editor.tsx` |

**Order:** V1 → V2 → V3 → V4

V1 is a one-line fix (quick win). V2 before V3 so the standard error shape exists when we build the parser. V4 is independent.
