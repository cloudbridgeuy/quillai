# QuillAI

AI-assisted fiction editor. Rust/Axum API backend + React/Bun frontend with Quill.js.

For agent onboarding, start with `primer.md`, then use this file as the source index.

## Workspace Layout

Cargo workspace with four members:

```
crates/api/    — Axum HTTP server, SQLite, CLI config
crates/log/    — Structured logging library (tracing-based)
crates/delta/  — Quill Delta core types, JSON serde, push/normalize, compose/transform/transform_position/invert/diff (UTF-16 lengths)
xtask/         — Development process runner (`cargo xtask dev`)
app/           — React frontend (Bun runtime, Quill.js editor)
migrations/    — SQLx migrations (SQLite)
docs/          — QuillJS implementation guide
```

## Entry Points

| What | Command | File |
|------|---------|------|
| Full dev | `cargo xtask dev` | `xtask/src/main.rs` |
| Lint and tests (short output) | `cargo xtask lint` | `xtask/src/main.rs` |
| Isolated two-page chapter sync smoke | `cargo xtask e2e` | `xtask/src/e2e.rs`, `e2e/chapter-sync.spec.ts` |
| API server only | `cargo run -p quillai_api --bin api` | `crates/api/src/main.rs` |
| Frontend dev only | `PORT=3001 bun --hot app/src/index.tsx` | `app/src/index.tsx` |

API defaults to `127.0.0.1:2908`. Frontend defaults to port `3001` (configurable via `PORT` env var).
Set the frontend URL's `apiBaseUrl` query parameter to configure both AI and manuscript requests, e.g. `http://localhost:3001/?apiBaseUrl=http%3A%2F%2F127.0.0.1%3A49152`. The default is `http://127.0.0.1:2908`. No browser-side `process.env` or rebuild is needed. Only loopback hosts (`127.0.0.1`, `localhost`, `[::1]`) over HTTP(S) with any port are accepted; malformed or non-loopback values fall back to the default without throwing (`app/src/api_config.ts`).
The override does not change CORS: the API only allows `localhost:3001` / `127.0.0.1:3001`, so the frontend must still be served on port `3001`.

## Source Index

### Rust — API (`crates/api/src/`)

| File | Purpose |
|------|---------|
| `main.rs` | Server init, SQLite pool setup, runs migrations before serving, calls `app()` from lib.rs |
| `database.rs` | Shared embedded migration runner for startup and integration tests; baselines legacy databases that have `users` but no `_sqlx_migrations` |
| `chapter_revisions.rs` | Pure Delta rebuild and transactional revision store with optimistic head checks and configurable snapshots |
| `lib.rs` | Router composition — merges resource routes via `Router::merge()` |
| `cli.rs` | CLI args via clap (host, port, db-url, log-level) |
| `error.rs` | Error enum (`Sqlx`, `InvalidLogLevel`, `NotFound`, `Validation`), manual `IntoResponse` → JSON `{ error, code }`. See [error handling context](.claude/context/error-handling.md) |
| `prelude.rs` | Re-exports Error and Result |
| `ai/` | Rich AI request parsing, prompt building, OpenAI-compatible SSE streaming, and NDJSON event output. See [AI streaming context](.claude/context/ai-streaming.md) |
| `users/` | Users CRUD module — models, handlers, route wiring. See [users CRUD context](.claude/context/users-crud.md) |
| `manuscripts/series/` | Series CRUD models, validation, handlers, and route wiring; HTTP coverage in `crates/api/tests/series_test.rs` |
| `manuscripts/books/` | Series-scoped book CRUD models, validation, handlers, and route wiring; HTTP coverage in `crates/api/tests/books_test.rs` |
| `manuscripts/chapters/` | Book-scoped chapter CRUD and revision sync, validated Delta input, rebuilt content, metadata validation, and ordering; HTTP coverage in `crates/api/tests/chapters_test.rs` |

### Rust — Logging (`crates/log/src/`)

| File | Purpose |
|------|---------|
| `lib.rs` | LogLevel enum, LogConfig struct, tracing subscriber init. Formats: Pretty, Compact, Json |

### Rust — Build (`xtask/src/`)

| File | Purpose |
|------|---------|
| `main.rs` | `cargo xtask dev` runner: starts API and frontend concurrently, prefixes logs, and stops both on Ctrl-C. `cargo xtask lint` runs fmt, clippy, tests, dependency and file-size checks with short output (logs in `target/xtask-lint/`) |
| `e2e.rs` | `cargo xtask e2e`: temporary SQLite file, random loopback API port, frontend port 3001, HTTP readiness, managed Playwright runner, process-group and database cleanup. Pass Playwright flags directly, e.g. `cargo xtask e2e --headed --trace on`. |

### Browser Smoke (`e2e/`)

`playwright.config.ts` loads the app's `@playwright/test` dependency and runs `chapter-sync.spec.ts` and `chapter-emoji-sync.spec.ts` with Chromium. Install the browser once with `cd app && bun --bun x playwright install chromium`. The smoke seeds one chapter through the API, opens two pages with the sandbox `apiBaseUrl` and chapter IDs, edits from each page, checks remote convergence and server revisions/content, then reloads both pages. Artifacts go to `target/xtask-e2e/results/`. The directory is outside `app/`, so `cargo xtask lint`'s `bun test` does not discover browser tests. See [browser sandbox context](.claude/context/ai-editor.md#browser-smoke-sandbox).

### Frontend (`app/src/`)

| File | Purpose |
|------|---------|
| `index.tsx` | Bun.serve entry — routes, static assets, HMR. Port via `PORT` env var |
| `index.html` | HTML shell, loads frontend.tsx |
| `App.tsx` | Root React component — renders the editor inside an ErrorBoundary. See [error boundary context](.claude/context/error-boundary.md) |
| `components/ErrorBoundary.tsx` | React error boundary with ADT state, crash isolation per component. See [error boundary context](.claude/context/error-boundary.md) |
| `frontend.tsx` | React root setup, CSS import, StrictMode |
| `index.css` | Tailwind + editor shell layout, dark theme, and responsive styling |
| `md_editor/types.ts` | TypeScript types: Delta, Range, EmitterSource, event callbacks |
| `md_editor/ai_contract.ts` | Pure AI request contract and browser-safe API URL helper. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/prompt_sections.ts` | Pure server-parity prompt sections from accepted AI requests; labeled UTF-8 byte token estimates and summed totals. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/context_inspector.ts` | Pure live Continue/Resume context view-model; Editor reuses request previews, refreshes on all Quill changes and settings/thread changes, sends per-section text overrides, and hides the panel at 800px. |
| `fixtures/prompt/parity.json` | Shared prompt section ids, tags, text, and history fixtures replayed by `crates/api/tests/prompt_parity_test.rs` and `app/src/md_editor/prompt_parity.test.ts`; provider model names pass through unchanged. |
| `md_editor/ai_stream.ts` | Pure incremental NDJSON parser for AI stream events. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/ai_state.ts` | AI run-state ADT and transition helpers. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/window.ts` | Pure context-window calculation for transmitted document slices. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/delta_bridge.ts` | Quill delta normalization at the request and stream boundaries. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/editor_ai.ts` | Pure selection-memory, rollback, and status helpers for the editor shell. See [AI editor context](.claude/context/ai-editor.md) |
| `manuscripts/contract.ts`, `manuscripts/client.ts` | Typed manuscript hierarchy and revision API client with response parsing and structured errors. |
| `md_editor/chapter_sync.ts` | Pure revision sync core: one in-flight Delta, composed pending edits, server-priority transforms, and acknowledgement reconciliation. |
| `md_editor/chapter_session.ts`, `md_editor/use_chapter_sync.ts` | Chapter selection/bootstrap, serialized save/poll shell, and per-tab unsaved recovery. Server chapter content/head_rev is authoritative on reload. |
| `md_editor/editor_persistence.ts` | Pure parse/serialize of local AI settings and thread under `quillai.editor.state.v1`; the editor writes an empty document placeholder and never restores chapter content from this key. Invalid data falls back to defaults. |
| `../fixtures/delta/` | Language-neutral Delta fixtures generated by `quill-delta` (`cd app && bun run fixtures:delta`), replayed by `src/delta_fixtures/delta_fixtures.test.ts`; also replayed by `crates/delta/tests/fixtures.rs` (`cargo test -p quillai_delta`). |
| `md_editor/markdown.ts` | Pure markdown analysis functions — `analyzeMarkdownContext`, `isPatternComplete`. See [markdown parsing context](.claude/context/markdown-parsing.md) |
| `md_editor/components/Editor.tsx` | Quill editor shell for Continue/Edit streaming, cancellation, rollback, and markdown formatting. See [AI editor context](.claude/context/ai-editor.md) and [markdown parsing context](.claude/context/markdown-parsing.md) |
| `api_tester/components/APITester.tsx` | HTTP method selector + URL input + loading state + structured error parsing. See [error handling context](.claude/context/error-handling.md) |
| `api_tester/components/__tests__/APITester.test.ts` | Unit tests for `buttonLabel()` pure function |

### Database

`migrations/20250407204144_init.up.sql` creates `users` (id, name, email). `migrations/20261019000000_series_books_chapters.up.sql` adds `series` → `books` → `chapters` with integer IDs, titles, positions, and cascading parent foreign keys. Chapters restrict kind to `chapter`, `prologue`, or `epilogue`; content stores valid JSON text with an empty Delta (`{"ops":[]}`) default. Parent/position indexes support ordered child queries. The down migration drops children first and preserves users.

Startup applies embedded migrations before serving and explicitly enables SQLite foreign keys. Integration tests share `crates/api/tests/common/mod.rs`, which creates isolated, single-connection in-memory pools and runs the same migrations. `crates/api/tests/book_schema_test.rs` covers cascades, orphan rejection, kinds, position ordering, Delta storage, and rollback/reapply.

`migrations/20261019000001_chapter_revisions.up.sql` adds retained `chapter_revisions` (chapter FK cascade, unique chapter/revision, Delta JSON, creation timestamp) and chapter `head_rev`/`snapshot_rev` markers. `chapters.content` is the snapshot Delta at `snapshot_rev` (existing content starts at revision zero). `RevisionStore` takes a nonzero snapshot interval, replays revisions strictly above the snapshot in revision order, and atomically appends against an expected head. HTTP sync snapshots every 100 revisions, retains all revisions, transforms concurrent submissions with server priority, and retries optimistic head conflicts. GET list/detail and PUT responses rebuild snapshot plus revisions within the same transaction as the chapter metadata read/write. Sync accepts a base/since only when every subsequent revision through the head is retained; the earliest retained revision minus one is the floor for contiguous history, independent of snapshot_rev. Missing required revisions, negative bases, and bases above the head return 409. `crates/api/tests/chapter_revisions_test.rs` covers rebuild, snapshots, stale heads, rollback, and constraints; `chapters_test.rs` covers HTTP sync, concurrency, convergence, input rejection, and parent scoping.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Axum 0.8, Tokio 1.44, SQLx (SQLite), Sea-ORM |
| Logging | tracing + tracing-subscriber |
| CLI | clap 4.5 |
| Frontend | React 19, TypeScript, Quill.js 2.0.3 |
| Styling | Tailwind CSS 4 |
| Runtime | Bun (server + bundler + test runner) |
| Build | Cargo workspace + xtask |

## API Routes

**Rust backend** (`crates/api/src/`):
- `GET /users` — List all users (JSON array)
- `POST /users` — Create user (201 + JSON)
- `GET /users/{id}` — Get user by ID (JSON or 404)
- `PUT /users/{id}` — Partial update user (JSON, 404, or 422)
- `DELETE /users/{id}` — Delete user (204 or 404)
- `GET /series` — List series ordered by position, then ID (JSON array)
- `POST /series` — Create series with nonblank title and nonnegative integer position (201 + JSON or 422)
- `GET /series/{id}` — Get series by ID (JSON or 404)
- `PUT /series/{id}` — Partial update title and/or position (JSON, 404, or 422); omitted/null fields stay unchanged
- `DELETE /series/{id}` — Delete series and cascade children (204 or 404)
- `GET /series/{id}/books` — List books ordered by position, then ID (JSON array or 404 for missing series)
- `POST /series/{id}/books` — Create book with nonblank title and nonnegative integer position; parent from path (201 + JSON, 404, or 422)
- `GET /series/{id}/books/{book_id}` — Get book scoped to series (JSON or 404)
- `PUT /series/{id}/books/{book_id}` — Partial update title and/or position, scoped to series (JSON, 404, or 422); omitted/null fields stay unchanged
- `DELETE /series/{id}/books/{book_id}` — Delete book scoped to series and cascade chapters (204 or 404)
- `GET /series/{id}/books/{book_id}/chapters` — List chapters ordered by position, then ID; includes rebuilt head content and head_rev (JSON array or 404)
- `POST /series/{id}/books/{book_id}/chapters` — Create chapter with title, position, kind (chapter/prologue/epilogue), and optional Delta content; snapshot at revision zero (201 + JSON, 404, or 422). Delta input requires an object with an ops array and valid operations; omitted content defaults to an empty Delta
- `GET /series/{id}/books/{book_id}/chapters/{chapter_id}` — Get chapter rebuilt head content and head_rev, scoped to book and series (JSON or 404)
- `PUT /series/{id}/books/{book_id}/chapters/{chapter_id}` — Partial update title, kind, and/or position; omitted/null fields unchanged; any content key rejected (JSON, 404, or 422)
- `DELETE /series/{id}/books/{book_id}/chapters/{chapter_id}` — Delete chapter scoped to book and series (204 or 404)
- `POST /series/{id}/books/{book_id}/chapters/{chapter_id}/deltas` — Submit `{baseRev, delta}`; transform against later retained revisions with server priority, append atomically, return `{rev, delta}` (200, 404, 409 unavailable/future history, or 422 malformed JSON/Delta or compose failure)
- `GET /series/{id}/books/{book_id}/chapters/{chapter_id}/deltas?since=rev` — Return missed revisions ordered ascending as `[{rev, delta}]`; current head returns `[]` (200, 404, 409 unavailable/future history, or 422 invalid query)
- `POST /ai/generate` — Stream AI delta fragments as NDJSON. See [AI streaming context](.claude/context/ai-streaming.md)

**Bun frontend** (`app/src/index.tsx`):
- `GET /assets/*` — static file serving
- `GET /api/hello` — JSON hello
- `PUT /api/hello` — JSON hello (PUT)
- `GET /api/hello/:name` — parameterized JSON hello
- `GET /*` — SPA fallback to index.html

## Coding Patterns

Reference patterns at `~/.claude/patterns/`:

| Pattern | File | Core Idea |
|---------|------|-----------|
| Functional Core - Imperative Shell | `functional-core-imperative-shell.md` | Pure logic separated from I/O at every level |
| Type-Driven Development | `type-driven-development.md` | Design types first, let compiler guide implementation |
| ADTs | `algebraic-data-types.md` | Sum types (enum) + product types (struct) to model domains |
| Make Impossible States Impossible | `make-impossible-states-impossible.md` | Only valid states representable in the type system |
| Parse Don't Validate | `parse-dont-validate.md` | Convert raw input to typed values at boundaries |
| CQRS | `cqrs.md` | Separate write models from read models |
| Composition over Inheritance | `composition-over-inheritance.md` | Trait-based components, factory functions |
| Data-Oriented Design | `data-oriented-design.md` | SoA layout, batch processing for cache efficiency |
| Relationships | `RELATIONSHIPS.md` | Visual graph of how patterns interconnect |

## Conventions

Default to using Bun instead of Node.js.

- Use `bun <file>` instead of `node <file>` or `ts-node <file>`
- Use `bun test` instead of `jest` or `vitest`
- Use `bun build <file.html|file.ts|file.css>` instead of `webpack` or `esbuild`
- Use `bun install` instead of `npm install` or `yarn install` or `pnpm install`
- Use `bun run <script>` instead of `npm run <script>` or `yarn run <script>` or `pnpm run <script>`
- Bun automatically loads .env, so don't use dotenv.

### Bun APIs

- `Bun.serve()` supports WebSockets, HTTPS, and routes. Don't use `express`.
- `bun:sqlite` for SQLite. Don't use `better-sqlite3`.
- `Bun.redis` for Redis. Don't use `ioredis`.
- `Bun.sql` for Postgres. Don't use `pg` or `postgres.js`.
- `WebSocket` is built-in. Don't use `ws`.
- Prefer `Bun.file` over `node:fs`'s readFile/writeFile
- Bun.$\`ls\` instead of execa.

### Rust Conventions

- `crates/delta` parity rule: the crate is a superset of JS `quill-delta` 5.1.0 (the version the browser runs). It may add functions but must never drop or change JS behavior: for every Delta `quill-delta` accepts that holds valid Unicode, parse/serialize, `compose`, `transform`, `transform_position`, `invert` and `diff` give exactly the JS JSON, including zero-length ops (`{"insert":""}`, `{"retain":0}`, `{"delete":0}`) and odd ops (negative/fractional counts, several op keys, unknown keys, `"attributes": null`, embed retains; parsed as `Op::Raw`, lengths as `f64`). The only intended exception is half a UTF-16 surrogate pair (half an emoji), which returns a Rust error; `crates/delta/tests/fixtures.rs` pins those cases by count (9 parse-level, 7 compose inputs, 2 compose results, 3 invert slices, 0 diff cases) and any other mismatch must fail. Also matched (GUZ-718): array `attributes` (read as index-keyed objects), whole-number floats (`1.0` prints `1`; floats print like JS), non-object `ops` entries and extra top-level keys next to `ops` (ignored), a top-level array (read as the ops) and a non-array or missing `ops` or a scalar top level (read as an empty Delta; GUZ-720), and out-of-range numbers (`1e400` reads as `Infinity`, prints `null`; `Delta::from_json` only, since serde_json's own parser rejects them); `fixtures/delta/parse.json` pins these with raw JSON text, replayed by Rust only. Object key order is not preserved, and cases where JS throws are out of scope.
- MSRV is Rust 1.95.0 or higher, set as `rust-version` in the workspace `Cargo.toml` and inherited by every member crate.
- Workspace crates under `crates/`. New crates go there.
- `cargo xtask` for build automation — extend `xtask/src/main.rs` for new tasks.
- Tracked `.rs` files under `crates/` and `xtask/` must stay at or below 1000 lines (enforced by `cargo xtask lint`).
- SQLx with compile-time checked queries (`.sqlx/` cache). See [SQLx offline context](.claude/context/sqlx-offline.md).
- Errors via `thiserror` with manual `IntoResponse` for JSON responses. See [error handling context](.claude/context/error-handling.md).

### Frontend Conventions

- HTML imports with `Bun.serve()`. Don't use `vite`.
- HTML files import .tsx/.jsx/.js directly — Bun transpiles and bundles.
- `<link>` tags point to stylesheets — Bun's CSS bundler handles them.
- Tailwind via `bun-plugin-tailwind`.
- For Bun API docs: `node_modules/bun-types/docs/**.md`.

## Context Docs

`.claude` is gitignored; the docs below stay tracked, but a new context doc must be added with `git add -f`.

- `.claude/context/ai-streaming.md` — AI request validation, prompt rules, provider env vars, and NDJSON stream behavior
- `.claude/context/ai-editor.md` — Quill AI editor shell, helper modules, and the current browser URL constraint
- `.claude/context/markdown-parsing.md` — markdown shortcut logic inside the editor shell
- `.claude/context/error-handling.md` — JSON error contract across backend and frontend
- `.claude/context/error-boundary.md` — React crash isolation pattern
- `.claude/context/sqlx-offline.md` — SQLx offline workflow
- `.claude/context/users-crud.md` — users CRUD resource template
