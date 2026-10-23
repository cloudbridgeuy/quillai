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
docs/          — QuillJS implementation guide, plans, and the v1 editor design spec (`docs/editor-design.md`)
prototypes/    — Static HTML design prototypes, not built or tested (`prototypes/editor-desktop/` is the visual reference for `docs/editor-design.md`)
```

## Entry Points

| What | Command | File |
|------|---------|------|
| Full dev | `cargo xtask dev` | `xtask/src/main.rs` |
| Lint, tests, and project tsc (short output) | `cargo xtask lint` | `xtask/src/main.rs` |
| Isolated browser smoke (random frontend port) | `cargo xtask e2e` (or `cargo xtask e2e --port 3107`) | `xtask/src/e2e.rs`, `e2e/playwright.config.ts` |
| API server only | `cargo run -p quillai_api --bin api` | `crates/api/src/main.rs` |
| AI model catalog | `cargo xtask dev --ai-models models.json` (also `e2e` and API `--ai-models`; env `QUILLAI_AI_MODELS_FILE`) | `crates/api/src/ai/catalog.rs` |
| Frontend dev only | `PORT=3001 bun --hot app/src/index.tsx` | `app/src/index.tsx` |

API defaults to `127.0.0.1:2908`. Frontend defaults to port `3001` (configurable via `PORT` env var).
Set the frontend URL's `apiBaseUrl` query parameter to configure both AI and manuscript requests, e.g. `http://localhost:3001/?apiBaseUrl=http%3A%2F%2F127.0.0.1%3A49152`. The default is `http://127.0.0.1:2908`. No browser-side `process.env` or rebuild is needed. Only loopback hosts (`127.0.0.1`, `localhost`, `[::1]`) over HTTP(S) with any port are accepted; malformed or non-loopback values fall back to the default without throwing (`app/src/api_config.ts`).
The override does not change CORS: the API defaults to allowing `http://localhost:3001` / `http://127.0.0.1:3001`. Repeat `--cors-origin <origin>` on the API CLI to replace this allow-list. E2E supplies its chosen frontend origin; `cargo xtask dev` defaults stay unchanged.

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
| `ai/` | Startup JSON model catalog with per-action defaults and legacy `QUILLAI_AI_MODEL` fallback (`catalog.rs`), rich request parsing, prompt building, OpenAI-compatible SSE streaming with per-model `include_usage` (default true), and NDJSON usage before done. False omits `stream_options` and usage events; null upstream errors are ignored. See [AI streaming context](.claude/context/ai-streaming.md) |
| `users/` | Users CRUD module — models, handlers, route wiring. See [users CRUD context](.claude/context/users-crud.md) |
| `settings/` | Singleton author guidance models, UTF-8 byte validation, GET/PUT handlers, and shared server-side AI loading. |
| `manuscripts/series/` | Series CRUD models, validation, handlers, and route wiring; HTTP coverage in `crates/api/tests/series_test.rs` |
| `manuscripts/books/` | Series-scoped book CRUD models, validation, handlers, and route wiring; HTTP coverage in `crates/api/tests/books_test.rs` |
| `manuscripts/lore/` | Series-scoped author lore CRUD, field validation, JSON-array storage, and boolean pin mapping; HTTP coverage in `crates/api/tests/lore_test.rs` |
| `manuscripts/chapters/` | Book-scoped chapter CRUD and revision sync, validated Delta input, rebuilt content, metadata validation, and ordering; HTTP coverage in `crates/api/tests/chapters_test.rs` |
| `eval/`, `bin/eval.rs` | Eval set CLI (`cargo run -p quillai_api --bin eval -- export\|check`): exports one book via the chapter API to a gitignored `eval-fixtures/` fixture (refuses paths git does not ignore) and parses typed case files (Continue offset or Edit range plus instruction, Quill editor indices with embeds counted as 1). See [eval](docs/eval.md) |

### Rust — Logging (`crates/log/src/`)

| File | Purpose |
|------|---------|
| `lib.rs` | LogLevel enum, LogConfig struct, tracing subscriber init. Formats: Pretty, Compact, Json |

### Rust — Build (`xtask/src/`)

| File | Purpose |
|------|---------|
| `main.rs` | `cargo xtask dev` runner: starts API and frontend concurrently, prefixes logs, and stops both on Ctrl-C. `cargo xtask lint` runs fmt, clippy, tests, dependency and file-size checks, plus the project compiler `app/node_modules/typescript/bin/tsc --noEmit -p app`, with short output (logs in `target/xtask-lint/`) |
| `e2e.rs` | `cargo xtask e2e`: temporary SQLite file, random loopback API and frontend ports, HTTP readiness, managed Playwright runner, process-group and database cleanup. Catalog precedence: `--ai-models` > `QUILLAI_AI_MODELS_FILE` > workspace-root `e2e/fixtures/ai-models.json`; only the default fixture starts managed Bun TypeScript `e2e/fake-ai-provider.ts`, sets API `QUILLAI_AI_BASE_URL` to its random loopback `/v1` URL with a fake key, and supplies Playwright `QUILLAI_E2E_FAKE_AI_URL`. The fixture immediately rejects and counts unarmed requests; only `real-route-stream.spec.ts` arms a one-use model/document identity for real `/ai/generate` Finish/Stop convergence at 1100px, with request/fragment/finish controls and actual upstream-close observation. Set the frontend port with `--port 3107` before Playwright arguments. Remaining arguments pass directly, e.g. `cargo xtask e2e --headed --trace on`. The runner sets API `--cors-origin` and Playwright `QUILLAI_E2E_APP_URL` for the chosen frontend. With `NO_COLOR` present or non-terminal stdout, prefixes are plain and all children receive `NO_COLOR=1`, `FORCE_COLOR=0`, and `CARGO_TERM_COLOR=never`. |

### Browser Smoke (`e2e/`)
`continue-tie-order.spec.ts` also arms one-use real-provider identities at 1100px: server-priority ties can place remote paragraph-start typing in the AI paragraph (documented limitation); Finish pins `REMOTE AI first tail` / `Later paragraph`, Stop keeps `REMOTE Later paragraph`, with exact two-tab/server convergence and no API errors.
`stream-remote-edit.spec.ts` checks exact two-tab/server convergence after Finish or Stop during desktop Continue at 1100px, preserving author and remote edits at distinct insertion points, retaining AI text only on Finish, and rejecting API errors; AI transport is mocked, not the real generation API.

`emoji-cursor-remote.spec.ts` checks caret/selection surrogate boundaries during remote insert/delete, subsequent typing, HTTP errors, and exact two-tab/server convergence at 1100px, including adjacent 😀😃 and a deliberately requested mid-surrogate caret.

`reload-in-flight.spec.ts` checks server-authoritative reload, full unsaved recovery export, and resumed saves at 1100px.

`offline-stale-base.spec.ts` holds one tab's delta submission offline while the other commits three edits, then verifies HTTP 200 for the stale base and exact convergence of both tabs and server with all in-flight and pending edits at 1100px.

`model-picker.spec.ts` also runs in the explicit allowlist. It mocks the model catalog, checks persisted per-action choices in Continue/Edit requests, and checks omitted `model` after catalog failure. Screenshots at 1100px and 1440px go to `target/guz-680/<browser>/`.

`playwright.config.ts` loads the app's `@playwright/test` dependency and runs every allowlisted spec in Chromium, Firefox and WebKit projects by default. Install the browsers once with `cd app && bun --bun x playwright install chromium firefox webkit`. Use `cargo xtask e2e --project chromium` as the quick gate; a full three-browser run takes about 10 minutes. The chapter smoke seeds one chapter through the API, opens two pages with the sandbox `apiBaseUrl` and chapter IDs, edits from each page, checks remote convergence and server revisions/content, then reloads both pages. The guidance smoke verifies locked System, explicit save, escaped preview, reload, and clearing. The mocked Continue stream checks concurrent author edits, selection preservation, exact rollback, local tint preference, and completion. A `usage` event before `done` does not end the run, and `finishReason: length` still arms Continue with thread. Desktop edge cases check Settings dismissal and live tint during streaming, plus transport failure and premature EOF rollback with author inserts, at 1100px and 1440px. Continue window coverage checks bracket geometry and unchanged transmitted slices, single-marker thread history, off-screen flag navigation, persisted visibility, and the 1100px breakpoint; screenshots at 1100px and 1440px go to `target/guz-757/<browser>/`. Lore coverage creates, edits, matches, reloads, and deletes series lore, verifies author-only notes never enter Continue/Edit requests, and captures the panel/inspector at 1100px and 1440px in `target/guz-671/<browser>/`. Named screenshot and artifact directories include the browser project (`chromium`, `firefox` or `webkit`). Save recovery forces a chapter delta POST failure, waits for the red Not saved pill, releases the route, and checks the client resumes so server content matches text typed before and during the failure at 1100px. Other artifacts go to project-specific test directories under `target/xtask-e2e/results/`. The directory is outside `app/`, so `cargo xtask lint`'s `bun test` does not discover browser tests. See [browser sandbox context](.claude/context/ai-editor.md#browser-smoke-sandbox).

### Frontend (`app/src/`)

| File | Purpose |
|------|---------|
| `index.tsx` | Bun.serve entry — routes, static assets, HMR. Port via `PORT` env var |
| `index.html` | HTML shell, loads frontend.tsx |
| `App.tsx` | Root React component — renders the editor inside an ErrorBoundary. See [error boundary context](.claude/context/error-boundary.md) |
| `components/ErrorBoundary.tsx` | React error boundary with ADT state, crash isolation per component. See [error boundary context](.claude/context/error-boundary.md) |
| `components/StickyNote.tsx` | Shared native textarea with yellow Story Context, Continue instruction, Thread continuation prompt, and author guidance styling; pink remains the rail Edit note. Forwards native props and ref. Enter and Apply share selection/readiness checks; Shift+Enter adds a newline and Esc cancels. Guidance saves explicitly. |
| `frontend.tsx` | React root setup, CSS import, StrictMode |
| `index.css` | Tailwind + editor shell layout, white paper/desk theme tokens and responsive styling. Right drawer uses 12px labels/eyebrows and 13px body/controls/placeholders, retaining 12px mono prompt text. Desktop drawers span 100dvh with contained internal scrolling; the right drawer heading stays pinned above a thin divider. Neither drawer has a Close button. Manuscript and Context rail toggles stay unmuted with ink text on green while open. |
| `i18n/messages.ts` | Pure `en`/`es` UI message tables and `formatMessage` (`Intl` plurals, interpolation, numbers, dates, currency). English is the key source; Spanish is written to match it. UI language is separate from series language. See [i18n](docs/i18n.md). |
| `md_editor/types.ts` | TypeScript types: Delta, Range, EmitterSource, event callbacks |
| `md_editor/ai_contract.ts` | Pure AI request contract with optional model ID pass-through (absent when omitted), visible-only matched lore assembly, and browser-safe API URL helper. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/model_catalog.ts` | Pure public catalog parsing and per-action model resolution. The right drawer's Advanced AI controls offer persisted Continue/Edit selects from GET /ai/models; missing/unknown IDs use server action defaults. Failed/empty catalogs disable selects and omit model from requests. |
| `md_editor/cost.ts` | Pure actual-usage cost ADT: request-time model/pricing snapshot, latest usage only, exact bigint rational USD or explicit unknown. `inspector_cost.ts` adds separate input-only pending estimates, output rates, display rounding, and request-scoped actual usage for the inspector. See [cost policy](.claude/context/ai-editor.md#actual-usage-cost-policy). |
| `md_editor/prompt_sections.ts` | Pure server-parity prompt sections from accepted AI requests; labeled UTF-8 byte token estimates and summed totals. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/context_inspector.ts` | Pure live Continue and Edit context view-model; each preview uses its action's instruction. Continue shows "with thread" for effective history. Injected lore reports keyword/pin reasons, levels, standalone token estimates, and budget skips using the shared matcher. The right drawer also holds Story Context, Continue instruction, AI Thread/Reset Thread, and advanced AI controls. Desktop Settings holds Window Size, Whole document, Show Continue window, and Tint AI text; window controls remain in the drawer below 1100px. It replaces the separate Preview Context UI. System, saved guidance, and derived Lore are read-only; other content and speaker turns support edits/reset. Inline below 1100px. |
| `md_editor/editor_panels.ts`, `md_editor/components/ManuscriptTree.tsx` | Pure single-drawer state and Ctrl/Cmd bracket shortcut mapping; series/books/chapters outline with save-before-switch chapter navigation and a single-chapter GET through the manuscript client. Failed saves keep the current chapter. Manuscript and Context rail toggles expose `aria-expanded`; a second click or Escape closes and restores editor selection/focus. Manuscript rail browser coverage at 1100px and 1440px captures open/closed states in `target/guz-798/`. |
| `fixtures/prompt/parity.json` | Shared prompt section ids, tags, text, and history fixtures replayed by `crates/api/tests/prompt_parity_test.rs` and `app/src/md_editor/prompt_parity.test.ts`; provider model names pass through unchanged. |
| `md_editor/ai_stream.ts` | Pure incremental NDJSON parser for AI stream events, including `usage`. The editor shell ignores that event. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/ai_state.ts` | AI run-state ADT and transition helpers. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/window.ts` | Pure context-window calculation for transmitted document slices. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/lore_match.ts` | Pure name/alias/keyword matcher; carries every level unchanged and exposes no hidden notes. Whole words use NFC, Unicode simple case folding (no ß/SS expansion), and edges without letters, marks, numbers, connector punctuation, emoji modifiers, or ZWJ (including emoji/CJK terms). Budget counts name + visible text UTF-16 units: pins always included and charged first, then input-order matches, skipping optional entries that do not fit; pins may exceed the cap and then suppress all optional matches. |
| `md_editor/components/ContinueWindow.tsx` | Desktop live Continue marker from exact calculated window start to end, including context after the caret and full length in Whole document mode. DOM geometry follows author edits and concurrent streams; the top-viewport scroll flag reports integer viewport heights or just above below one screen. Settings persists Show Continue window alongside Tint AI text; neither marker changes the transmitted slice or recenters the caret. |
| `md_editor/delta_bridge.ts` | Quill delta normalization at the request and stream boundaries. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/editor_ai.ts` | Pure selection-memory, shared Enter/Apply availability, rollback, and status helpers. Header save labels omit the repeated title and keep save/recovery failures prominent. See [AI editor context](.claude/context/ai-editor.md) |
| `manuscripts/contract.ts`, `manuscripts/client.ts` | Typed manuscript hierarchy and revision API client with response parsing and structured errors. |
| `settings/client.ts` | Typed author guidance GET/PUT client with response parsing and structured errors; inspector loads saved server text on mount and saves explicitly. |
| `md_editor/chapter_sync.ts` | Pure revision sync core: one in-flight Delta, composed pending edits, server-priority transforms, and acknowledgement reconciliation. |
| `md_editor/selection_boundary.ts` | Pure UTF-16 endpoint correction: moves an index inside a surrogate pair to its start; remote sync applies it to both transformed selection ends. |
| `md_editor/chapter_sync.property.test.ts` | Seeded two-client/server convergence below the browser: 12 fixed seeds, 400 codepoint-safe edits per run, random send/commit/remote/ack interleavings. Replay with `QUILLAI_SYNC_SEED=729 bun test --cwd app src/md_editor/chapter_sync.property.test.ts`. |
| `md_editor/chapter_session.ts`, `md_editor/use_chapter_sync.ts` | Chapter selection/bootstrap, read-only selected chapter title, serialized save/poll shell, and per-tab unsaved recovery; failed POST recovery waits for the next existing one-second background poll, and flush cannot bypass that wait. Clean Continue reserves the session before awaiting an active poll, excludes later ticks, and retains Stop and unsaved rejection (`e2e/continue-poll.spec.ts`). Server chapter content/head_rev is authoritative on reload. |
| `md_editor/editor_persistence.ts` | Pure parse/serialize of local AI settings and thread under `quillai.editor.state.v1`, including separate `editInstruction`/`continueInstruction` and `threadState.settings.continueModel`/`editModel`. Legacy `instruction` loads into Continue only; Edit starts empty. The editor writes an empty document placeholder and never restores chapter content from this key. Invalid data falls back to defaults. |
| `../fixtures/delta/` | Language-neutral Delta fixtures generated by `quill-delta` (`cd app && bun run fixtures:delta`), replayed by `src/delta_fixtures/delta_fixtures.test.ts`; also replayed by `crates/delta/tests/fixtures.rs` (`cargo test -p quillai_delta`). |
| `md_editor/markdown.ts` | Pure markdown analysis functions — `analyzeMarkdownContext`, `isPatternComplete`. See [markdown parsing context](.claude/context/markdown-parsing.md) |
| `md_editor/prose.ts` | Pure passive-voice detector — `analyzeProse` returns UTF-16 `Hint` spans from a be-verb through a following participle. |
| `md_editor/components/Editor.tsx` | Quill Continue/Edit shell with separate pink-note Edit and drawer Continue instructions, used only by their matching preview/request. Desktop Continue (>=1100px) permits concurrent typing, grows below the original caret paragraph, preserves the logical caret/selection without recentering, and rolls back only surviving AI text on Stop/cancel/failure. `continue_stream.ts` transforms its anchor, append end and exact ownership through author edits; `continue_tint.ts` renders local-only tint with a completion fade and persisted on/off control in the bottom-gear Settings popover alongside Window Size and Whole document. Edit and narrow-screen Continue retain locked-editor behavior. Writing controls live in the right drawer; Export local chapter lives in the left drawer. The flowing header centers quiet save status above the sheet; save/recovery failure shows a floating red "Not saved" pill without shifting the sheet or covering prose. The full reason remains in its tooltip and screen-reader description. See [AI editor context](.claude/context/ai-editor.md) and [markdown parsing context](.claude/context/markdown-parsing.md) |
| `api_tester/components/APITester.tsx` | HTTP method selector + URL input + loading state + structured error parsing. See [error handling context](.claude/context/error-handling.md) |
| `api_tester/components/__tests__/APITester.test.ts` | Unit tests for `buttonLabel()` pure function |

The editor loads series-scoped lore through `manuscripts/lore_client.ts` and `use_lore.ts`. The left manuscript drawer's `LorePanel.tsx` lists, creates, edits, and deletes author lore, including levels and pins. Desktop reserves min(320px, 40dvh) for all manuscript tree states with internal scrolling; lore scrolls separately and Export local chapter stays fixed below it (`tree-load-shift.spec.ts`, screenshots in `target/guz-820/`). `lore_panel.ts` contains pure draft, term, and list helpers. Series changes clear stale entries; failed loads offer retry. A caller-supplied `loreEntries` list remains available for isolated tests. Lore stays derived/read-only so request-local text overrides cannot remove its non-reveal marker. Hidden notes remain author data, not AI context. Desktop `LoreWindow.tsx` uses the inspector's shared lore model for visual-only underlines, read-only hover cards, and the budgeted Continue-only "Lore in window" list. Missing `CSS.highlights` or `Highlight` disables only underline/hover coloring; DOM-range hover cards, list navigation, Alt+L, and Escape remain functional. Tests simulate missing APIs and run in the Chromium, Firefox and WebKit projects. It never changes Quill formats or saved content; browser proof at 1100px and 1440px goes to `target/guz-759/<browser>/`. See [lore prompt contract](.claude/context/ai-streaming.md#injected-lore).

### Database

`migrations/20250407204144_init.up.sql` creates `users` (id, name, email). `migrations/20261019000000_series_books_chapters.up.sql` adds `series` → `books` → `chapters` with integer IDs, titles, positions, and cascading parent foreign keys. Chapters restrict kind to `chapter`, `prologue`, or `epilogue`; content stores valid JSON text with an empty Delta (`{"ops":[]}`) default. Parent/position indexes support ordered child queries. The down migration drops children first and preserves users.

Startup applies embedded migrations before serving and explicitly enables SQLite foreign keys. Integration tests share `crates/api/tests/common/mod.rs`, which creates isolated, single-connection in-memory pools and runs the same migrations. `crates/api/tests/book_schema_test.rs` covers cascades, orphan rejection, kinds, position ordering, Delta storage, and rollback/reapply.

`migrations/20261019000001_chapter_revisions.up.sql` adds retained `chapter_revisions` (chapter FK cascade, unique chapter/revision, Delta JSON, creation timestamp) and chapter `head_rev`/`snapshot_rev` markers. `chapters.content` is the snapshot Delta at `snapshot_rev` (existing content starts at revision zero). `RevisionStore` takes a nonzero snapshot interval, replays revisions strictly above the snapshot in revision order, and atomically appends against an expected head. HTTP sync snapshots every 100 revisions, retains all revisions, transforms concurrent submissions with server priority, and retries optimistic head conflicts. GET detail, GET list with `?content=true`, and PUT responses rebuild snapshot plus revisions within the same transaction as the chapter metadata read/write. Default GET list selects only chapter headers without reading content or revisions. Sync accepts a base/since only when every subsequent revision through the head is retained; the earliest retained revision minus one is the floor for contiguous history, independent of snapshot_rev. Missing required revisions, negative bases, and bases above the head return 409. `crates/api/tests/chapter_revisions_test.rs` covers rebuild, snapshots, stale heads, rollback, and constraints; `chapters_test.rs` covers HTTP sync, concurrency, convergence, input rejection, and parent scoping.

`migrations/20261019000002_author_guidance.up.sql` adds a singleton `author_guidance` table (id constrained to 1, text capped at 65,536 UTF-8 bytes). Missing row and empty text both mean no guidance. PUT upserts the one row; the down migration drops only this table.

`migrations/20261020000000_lore_entries.up.sql` adds series-scoped `lore_entries` with cascading deletion and a series index: kind (`character`, `place`, `term`, `note`), nonblank name, separate `keywords` and `aliases` JSON-array text columns (each default `[]`, preserving search terms versus alternate names), visible text and hidden notes (default empty), pinned (0/1, default 0), and AI-request level (`include`, `known_dont_reveal`, `leave_out`, default `include`). The down migration drops only `lore_entries`; `crates/api/tests/lore_schema_test.rs` covers storage, defaults, constraints, cascades, and rollback/reapply.

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
- `GET /series/{id}/lore` — List author lore ordered by name, then ID, including hidden notes (200 JSON array or 404 for missing series)
- `POST /series/{id}/lore` — Create lore with required kind (character/place/term/note) and nonblank name; optional fields retain migration defaults (201 + JSON, 404, or standard JSON 422 for invalid fields/types). Keywords/aliases are arrays of nonblank strings; pinned is boolean; level is include/known_dont_reveal/leave_out
- `GET /series/{id}/lore/{lore_id}` — Get author lore scoped to series, including hidden notes (200 JSON or 404)
- `PUT /series/{id}/lore/{lore_id}` — Partial lore update scoped to series (200 JSON, 404, or standard JSON 422); omitted/null fields unchanged, including pinned=false as an explicit update
- `DELETE /series/{id}/lore/{lore_id}` — Delete lore scoped to series (204 or 404)
- `GET /series/{id}/books/{book_id}/chapters` — List chapters ordered by position, then ID; default returns only id, title, position, kind, head_rev without reading content or revisions. `?content=true` returns full chapters with book_id and rebuilt head content; any other supplied content value, including false or empty, returns 422 (JSON array, 404, or 422)
- `POST /series/{id}/books/{book_id}/chapters` — Create chapter with title, position, kind (chapter/prologue/epilogue), and optional Delta content; snapshot at revision zero (201 + JSON, 404, or 422). Delta input requires an object with an ops array and valid operations; omitted content defaults to an empty Delta
- `GET /series/{id}/books/{book_id}/chapters/{chapter_id}` — Get chapter rebuilt head content and head_rev, scoped to book and series (JSON or 404)
- `PUT /series/{id}/books/{book_id}/chapters/{chapter_id}` — Partial update title, kind, and/or position; omitted/null fields unchanged; any content key rejected (JSON, 404, or 422)
- `DELETE /series/{id}/books/{book_id}/chapters/{chapter_id}` — Delete chapter scoped to book and series (204 or 404)
- `POST /series/{id}/books/{book_id}/chapters/{chapter_id}/deltas` — Submit `{baseRev, delta}`; transform against later retained revisions with server priority, append atomically, return `{rev, delta}` (200, 404, 409 unavailable/future history, or 422 malformed JSON/Delta or compose failure)
- `GET /series/{id}/books/{book_id}/chapters/{chapter_id}/deltas?since=rev` — Return missed revisions ordered ascending as `[{rev, delta}]`; current head returns `[]` (200, 404, 409 unavailable/future history, or 422 invalid query)
- `POST /ai/generate` — Stream AI delta fragments as NDJSON; optional `model` must exactly match a startup catalog ID (422 if invalid); omission uses the action default. See [AI streaming context](.claude/context/ai-streaming.md)
- `GET /ai/models` — Return startup catalog model IDs, labels, input/output prices per million tokens, and `defaults.generate` (Continue) / `defaults.edit` IDs. No provider configuration or `include_usage`; missing catalog returns the standard 500 configuration error.
- `GET /settings/author-guidance` — Return `{text: string}`; missing row returns empty text (200).
- `PUT /settings/author-guidance` — Save `{text: string}` and return saved text (200); empty clears guidance, more than 65,536 UTF-8 bytes returns 422. Invalid JSON/types return the standard 400 JSON error.

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
