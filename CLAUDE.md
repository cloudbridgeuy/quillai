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
docs/          — QuillJS implementation guide, plans, the v1 editor design spec (`docs/editor-design.md`), the design system (`docs/design-system.md`), and the continuous script surface spike (`docs/screenplay-surface.md`)
prototypes/    — Static HTML design prototypes, not built or tested (`prototypes/editor-desktop/` is the visual reference for `docs/editor-design.md`)
```

## Entry Points

| What | Command | File |
|------|---------|------|
| Full dev | `cargo xtask dev` | `xtask/src/main.rs` |
| Lint, tests, and project tsc (short output) | `cargo xtask lint` | `xtask/src/main.rs` |
| Isolated browser smoke (random frontend port) | `cargo xtask e2e` (or `cargo xtask e2e --port 3107`); prints only failures and a summary line, full log in `target/xtask-e2e/e2e.log`; `--verbose` prints full output | `xtask/src/e2e.rs`, `e2e/playwright.config.ts` |
| API server only | `cargo run -p quillai_api --bin api` | `crates/api/src/main.rs` |
| AI model catalog | `cargo xtask dev --ai-models models.json` (also `e2e` and API `--ai-models`; env `QUILLAI_AI_MODELS_FILE`) | `crates/api/src/ai/catalog.rs` |
| Frontend dev only | `PORT=3001 bun --hot app/src/index.tsx` | `app/src/index.tsx` |

API defaults to `127.0.0.1:2908` and SQLite file `sqlite:quillai.db` (created if missing). `--db-url` and `QUILLAI_API_DB_URL` override that URL. `cargo xtask dev` passes an absolute `--db-url` for `<workspace root>/quillai.db` (`sqlite:<path>?mode=rwc`) unless `--db-url` or `QUILLAI_API_DB_URL` is set; those values are forwarded unchanged. Frontend defaults to port `3001` (configurable via `PORT` env var).
Set the frontend URL's `apiBaseUrl` query parameter to configure both AI and manuscript requests, e.g. `http://localhost:3001/?apiBaseUrl=http%3A%2F%2F127.0.0.1%3A49152`. The default is `http://127.0.0.1:2908`. No browser-side `process.env` or rebuild is needed. Only loopback hosts (`127.0.0.1`, `localhost`, `[::1]`) over HTTP(S) with any port are accepted; malformed or non-loopback values fall back to the default without throwing (`app/src/api_config.ts`).
The override does not change CORS: the API defaults to allowing `http://localhost:3001` / `http://127.0.0.1:3001`. Repeat `--cors-origin <origin>` on the API CLI to replace this allow-list. E2E supplies its chosen frontend origin; `cargo xtask dev` defaults stay unchanged.

## Source Index

### Rust — API (`crates/api/src/`)

| File | Purpose |
|------|---------|
| `main.rs` | Server init, SQLite pool setup, runs migrations before serving, calls `app()` from lib.rs |
| `database.rs` | Shared embedded migration runner for startup and integration tests; baselines legacy databases that have `users` but no `_sqlx_migrations` |
| `chapter_revisions.rs` | Pure Delta rebuild and transactional revision store over leaf `pieces`/`piece_revisions` with optimistic head checks and configurable snapshots |
| `lib.rs` | Router composition — merges resource routes via `Router::merge()` |
| `cli.rs` | CLI args via clap (host, port, db-url, log-level) |
| `error.rs` | Error enum (`Sqlx`, `InvalidLogLevel`, `NotFound`, `Validation`), manual `IntoResponse` → JSON `{ error, code }`. See [error handling context](.claude/context/error-handling.md) |
| `prelude.rs` | Re-exports Error and Result |
| `ai/` | Startup JSON model catalog with per-action defaults and legacy `QUILLAI_AI_MODEL` fallback (`catalog.rs`), rich request parsing, prompt building, OpenAI-compatible SSE streaming with per-model `include_usage` (default true), and NDJSON usage before done. False omits `stream_options` and usage events; null upstream errors are ignored. Edit uses its own prompt rules, streams only text inside `<replacement>` tags (`replacement.rs`; missing/unclosed tag is a stream error). See [AI streaming context](.claude/context/ai-streaming.md) |
| `users/` | Users CRUD module — models, handlers, route wiring. See [users CRUD context](.claude/context/users-crud.md) |
| `settings/` | Singleton author guidance models, UTF-8 byte validation, GET/PUT handlers, and shared server-side AI loading. |
| `manuscripts/series/` | Series CRUD models, validation, handlers, and route wiring; HTTP coverage in `crates/api/tests/series_test.rs` |
| `manuscripts/books/` | Series-scoped book CRUD models, validation, handlers, and route wiring; HTTP coverage in `crates/api/tests/books_test.rs` |
| `manuscripts/lore/` | Series lore façade over universe lore (`store.rs` holds universe-keyed CRUD), field validation, JSON-array storage, and boolean pin mapping; HTTP coverage in `crates/api/tests/lore_test.rs` |
| `manuscripts/chapters/` | Book-scoped chapter CRUD and revision sync over the `legacy_chapters` view, validated Delta input, rebuilt content, metadata validation, and ordering; HTTP coverage in `crates/api/tests/chapters_test.rs` |
| `manuscripts/leaf_sync.rs` | Shared leaf Delta history and server-priority submit (snapshot every 100) for chapter and piece delta routes |
| `manuscripts/universes/`, `containers/`, `pieces/` | Story model routes (camelCase JSON, lore stays snake_case with `universe_id`); `mod.rs` holds JSON input parsing, PUT key rejection, and trigger RAISE → 422 mapping; HTTP coverage in `crates/api/tests/story_routes_test.rs` |
| `manuscripts/pieces/moves.rs` | Placement, detach, split, and attach inside one universe (`docs/story-model.md` §3): one UPDATE per move keeps piece id, revisions, and lore; triggers enforce universe, format, and membership (422); HTTP coverage in `crates/api/tests/piece_moves_test.rs` |
| `manuscripts/pieces/scenes.rs` | Script group surface routes (`docs/screenplay-surface.md` decisions 1–2): one GET loads every scene with content and headRev, one poll returns each requested scene's missed revisions or a per-scene conflict; prose groups and leaves are 422; HTTP coverage in `crates/api/tests/script_group_test.rs` |
| `eval/`, `bin/eval.rs` | Eval set CLI (`cargo run -p quillai_api --bin eval -- export\|check`): exports one book via the chapter API to a gitignored `eval-fixtures/` fixture (refuses paths git does not ignore) and parses typed case files (Continue offset or Edit range plus instruction, Quill editor indices with embeds counted as 1). See [eval](docs/eval.md) |

### Rust — Logging (`crates/log/src/`)

| File | Purpose |
|------|---------|
| `lib.rs` | LogLevel enum, LogConfig struct, tracing subscriber init. Formats: Pretty, Compact, Json |

### Rust — Build (`xtask/src/`)

| File | Purpose |
|------|---------|
| `main.rs` | `cargo xtask dev` runner: starts API and frontend concurrently, prefixes logs, and stops both on Ctrl-C. Dev opens `<workspace root>/quillai.db` unless `--db-url` or `QUILLAI_API_DB_URL` is set. `cargo xtask lint` runs fmt, clippy, tests, dependency and file-size checks, the project compiler `app/node_modules/typescript/bin/tsc --noEmit -p app`, and the `ui` design-system check, with short output (logs in `target/xtask-lint/`) |
| `dev_database.rs` | Pure dev API database arguments: forward `--db-url` / `QUILLAI_API_DB_URL` unchanged, otherwise absolute `<workspace>/quillai.db` with `mode=rwc` |
| `check_ui.rs` | Lint check `ui`: pure scanner over tracked `app/src` files for raw `<button>`/`<input>`/`<textarea>`/`<select>` and Tailwind arbitrary brackets in `.tsx` outside `app/src/ui/`, and hex colours or `font-size:` in `.css` outside `app/src/ui/tokens.css`; compares per-file, per-rule counts with `app/src/ui/ALLOWLIST` (`path count rule`) and fails above or below the count (ratchet). See [design system](docs/design-system.md) |
| `e2e.rs` | `cargo xtask e2e`: temporary SQLite file, random loopback API and frontend ports, HTTP readiness, managed Playwright runner, process-group and database cleanup. Catalog precedence: `--ai-models` > `QUILLAI_AI_MODELS_FILE` > workspace-root `e2e/fixtures/ai-models.json`; only the default fixture starts managed Bun TypeScript `e2e/fake-ai-provider.ts`, sets API `QUILLAI_AI_BASE_URL` to its random loopback `/v1` URL with a fake key, and supplies Playwright `QUILLAI_E2E_FAKE_AI_URL`. The fixture immediately rejects and counts unarmed requests; only armed specs (`real-route-stream.spec.ts`, `continue-tie-order.spec.ts` with `e2e-continue`; `edit-replacement.spec.ts` with `e2e-edit`; `screenplay-ai.spec.ts` with both) reach it, each arming a one-use model/document identity for real `/ai/generate`, with request/fragment/finish controls and actual upstream-close observation. Set the frontend port with `--port 3107` before Playwright arguments. Remaining arguments pass directly, e.g. `cargo xtask e2e --headed --trace on`. The runner sets API `--cors-origin` and Playwright `QUILLAI_E2E_APP_URL` for the chosen frontend. E2E and `cargo xtask dev` run the frontend from `app/` so `app/bunfig.toml` loads `bun-plugin-tailwind`; `e2e/e2e-styles.spec.ts` checks a token-styled Button at 1100px. With `NO_COLOR` present or non-terminal stdout, prefixes are plain and all children receive `NO_COLOR=1`, `FORCE_COLOR=0`, and `CARGO_TERM_COLOR=never`. |

### Browser Smoke (`e2e/`)
`ui-components.spec.ts` serves `e2e/fixtures/ui-components/` with `Bun.serve`, asserts every design-system primitive and key attributes, and captures full-page screenshots at 1100px and 1440px in `target/guz-874/<browser>/` and `target/guz-884/<browser>/`.

`edit-replacement.spec.ts` arms one-use `e2e-edit` identities at 1100px: tags split across fragments with commentary around them leave only the replacement in the editor and server; missing or unclosed tags show the error and restore the selection; a whole-paragraph selection with its line ending keeps the paragraph break (replacement `Revised paragraph` or `Revised paragraph\n`, editor and server GET), and an unclosed tag restores that paragraph.

`piece-move-lore.spec.ts` at 1100px seeds one universe with lore and two collections, records the Continue lore list for a piece, places the piece in the other collection through `POST /pieces/{id}/placement`, reloads, and checks the same lore list, text, head revision, and universe lore.

`screenplay-scroll.spec.ts` at 800px and 1100px opens a two-scene screenplay as one surface with both break titles; checks per-scene saves, remote edits keeping the caret, paste, drop, drag and IME next to a break (Chromium real CDP IME, Firefox/WebKit synthetic composition events), a server delete of a scene's trailing newline, delete+reinsert of a scene's last newline, save failure recovery, reload and Resume.

`screenplay-cross-scene.spec.ts` at 1100px opens a script surface from a scene and from the group in the manuscript tree (caret in that scene or the first scene), steps arrows over the break, selects across it, blocks Delete/Backspace/typing/Enter/paste over a cross-break selection and Backspace/Delete that would merge scenes with an Alert, keeps Enter at a scene end in that scene, checks each scene's server GET, and keeps prose pieces opening alone; Alert screenshots at 1100px and 1440px in `target/guz-984/<browser>/`.

`screenplay-notice.spec.ts` at 1100px and 1440px opens a two-scene script with a 40-line first scene: a tree click on a scene of the open script moves the caret there with no navigation and highlights that scene; a cut event and a drop on the break line across a break are refused before they apply (no edit, undo still restores the last typed text); the break Alert shows its message once on the page, sits in the sheet's top padding below the header, and never overlaps the scene's last line or shifts the sheet. Screenshots in `target/guz-988/<browser>/`.

`screenplay-new-scene.spec.ts` at 1100px and 1440px: New scene with the caret in scene 1 adds a scene between two (surface, tree, server order, reload), puts the caret in it and typing saves only to it; a forced create failure and a forced failure of the first or second renumber PUT each show "New scene not added" and a reload keeps the exact scene order and text on the surface, in the tree and on the server; a scene added from another tab shows "Scenes changed; reload" without rebuilding the surface. Screenshots in `target/guz-986/<browser>/`.

`screenplay-format.spec.ts` at 1100px and 1440px: on a two-scene script, a scene heading set from the Block type select and character/dialogue lines made with Tab and Enter save as `script-block` attributes on that scene's server GET and survive reload; the Download Fountain link yields the expected `.fountain` text; typing `# `, `- ` and `**x**` creates no header, list or bold; a format on the break line shows the break Alert and is never saved. At 1100px, remote same-scene insertion, Continue Finish, and Edit strictly inside a typed heading preserve block types on the surface and group scenes GET after reload; action lines have no `script-block` attribute. Edit including the heading newline keeps that line ending: `Revised` stays a scene heading and `ANNA` a character on the surface, after reload and on the group scenes GET. Screenshots in `target/guz-944/<browser>/`.

`screenplay-ai.spec.ts` at 1100px arms one-use real-provider identities on a two-scene script surface: the Continue lore window lists scene-1 lore with the caret in scene 2; a scene with a held save rejects Continue; Edit over a break shows a reason and disables Apply, Edit inside scene 2 saves only scene 2; Continue at scene 1's end sends no group poll while it runs, saves only scene 1 on Finish and then shows a remote scene-2 edit; Stop at scene 2's end leaves both scenes' server GET and headRev unchanged.

`screenplay-fdx.spec.ts` at 1100px sets every block type on a two-scene script with the Block type select, downloads `Hall script.fdx`, parses it with `DOMParser` (no `parsererror`) and checks paragraph Types and Text in scene order, including an escaped `&`; Chromium saves the sample to `target/guz-992/sample.fdx`.

`persistent-dev-database.spec.ts` at 1100px starts `target/debug/api` in a temporary directory with no `--db-url` and no `QUILLAI_API_DB_URL`, creates a chapter, types further text, kills that process, starts it again in the same directory, and checks the editor and the chapter GET still show the text. It does not use the sandbox database and does not open `<workspace>/quillai.db`.

`continue-tie-order.spec.ts` also arms one-use real-provider identities at 1100px: server-priority ties can place remote paragraph-start typing in the AI paragraph (documented limitation); Finish pins `REMOTE AI first tail` / `Later paragraph`, Stop keeps `REMOTE Later paragraph`, with exact two-tab/server convergence and no API errors.
`stream-remote-edit.spec.ts` checks exact two-tab/server convergence after Finish or Stop during desktop Continue at 1100px, preserving author and remote edits at distinct insertion points, retaining AI text only on Finish, and rejecting API errors; AI transport is mocked, not the real generation API.

`emoji-cursor-remote.spec.ts` checks caret/selection surrogate boundaries during remote insert/delete, subsequent typing, HTTP errors, and exact two-tab/server convergence at 1100px, including adjacent 😀😃 and a deliberately requested mid-surrogate caret.

`story-tree.spec.ts` at 1100px and 1440px seeds a book-series novel chapter and a loose piece in one universe; the tree lists every universe (other universes collapsed first, the open one expanded last) with its containers and loose pieces; book series show their façade novels and other roots (`manuscripts/story_tree.ts`), the piece opens by `pieceId` (lore from the piece's universe via `/universes/{id}/lore`), syncs through `/pieces/{id}/deltas`, survives reload, and the chapter still opens.

`create-form-fit.spec.ts` at 1100px and 1440px checks every field and button of the container and loose piece create forms sits inside the tree area without manual scrolling (the open form scrolls itself into view; Label and Format sit side by side), create controls align with sibling rows, and the no-scope lore wording; screenshots in `target/guz-971/<browser>/`.

`landing.spec.ts` at 1100px and 1440px opens `/` with no target: three seeded universes show as collapsed cards (narrower than half the page) and an opened universe card spans the grid row; an edited leaf (not one only opened) becomes the Resume card with its universe · container path and Open, and its universe starts open; the empty state shows when nothing is stored; a stored leaf deleted on the server shows "no longer in the library" with Forget instead of opening the editor; a chapter-scope (series/book/chapter) Resume shows its universe · series · book path and Open reopens it; the editor's Library button saves and returns; a loose piece created from landing opens and stays listed after reload. Screenshots in `target/guz-978/<browser>/`.

`reload-in-flight.spec.ts` checks server-authoritative reload, full unsaved recovery export, and resumed saves at 1100px.

`offline-stale-base.spec.ts` holds one tab's delta submission offline while the other commits three edits, then verifies HTTP 200 for the stale base and exact convergence of both tabs and server with all in-flight and pending edits at 1100px.

`model-picker.spec.ts` also runs in the explicit allowlist. It mocks the model catalog, checks persisted per-action choices in Continue/Edit requests, and checks omitted `model` after catalog failure. Screenshots at 1100px and 1440px go to `target/guz-680/<browser>/`.

`playwright.config.ts` loads the app's `@playwright/test` dependency and runs every allowlisted spec in Chromium, Firefox and WebKit projects by default. Install the browsers once with `cd app && bun --bun x playwright install chromium firefox webkit`. Use `cargo xtask e2e --project chromium` as the quick gate; a full three-browser run takes about 10 minutes. The chapter smoke seeds one chapter through the API, opens two pages with the sandbox `apiBaseUrl` and chapter IDs, edits from each page, checks remote convergence and server revisions/content, then reloads both pages. The guidance smoke verifies locked System, explicit save, escaped preview, reload, and clearing. The mocked Continue stream checks concurrent author edits, selection preservation, exact rollback, local tint preference, and completion. A `usage` event before `done` does not end the run, and `finishReason: length` still arms Continue with thread. Desktop edge cases check Settings dismissal and live tint during streaming, plus transport failure and premature EOF rollback with author inserts, at 1100px and 1440px. Continue window coverage checks bracket geometry and unchanged transmitted slices, single-marker thread history, off-screen flag navigation, persisted visibility, and the 1100px breakpoint; screenshots at 1100px and 1440px go to `target/guz-757/<browser>/`. Lore coverage creates, edits, matches, reloads, and deletes series lore, verifies author-only notes never enter Continue/Edit requests, and captures the panel/inspector at 1100px and 1440px in `target/guz-671/<browser>/`. Named screenshot and artifact directories include the browser project (`chromium`, `firefox` or `webkit`). Save recovery forces a chapter delta POST failure, waits for the red Not saved pill, releases the route, and checks the client resumes so server content matches text typed before and during the failure at 1100px. Other artifacts go to project-specific test directories under `target/xtask-e2e/results/`. The directory is outside `app/`, so `cargo xtask lint`'s `bun test` does not discover browser tests. See [browser sandbox context](.claude/context/ai-editor.md#browser-smoke-sandbox).

### Frontend (`app/src/`)

| File | Purpose |
|------|---------|
| `index.tsx` | Bun.serve entry — routes, static assets, HMR. Port via `PORT` env var |
| `index.html`, `logo.svg`, `tab_title.ts` | HTML shell with QuillAI fallback title and palette quill favicon; pure chapter-aware tab title applied by Editor, covered by Bun and `e2e/tab-title.spec.ts` |
| `App.tsx` | Root React component — renders the landing page when the URL has no `pieceId`, `seriesId`/`bookId`/`chapterId` or `editor` parameter, otherwise the editor, inside an ErrorBoundary. `?editor=1` opens the editor with its old first-chapter fallback. See [error boundary context](.claude/context/error-boundary.md) |
| `landing/Landing.tsx`, `landing/routes.ts`, `landing/resume.ts`, `landing/landing.css` | Landing page: a Resume card for the last leaf this browser edited (browser-side only, `localStorage` key `quillai.landing.lastEdited.v1` with scope and title, written on author/AI document changes, not on open) and the `ManuscriptTree` library (same create forms) as a grid of universe cards, collapsed by default (`collapsedExcept` prop) except the Resume leaf's universe. `resume.ts` resolves the stored scope against the API to the current title and universe · container path; a 404 shows "no longer in the library" with Forget (clears the key) instead of opening the editor. `landing.css` styles the card grid with `@reference "../index.css"` and `@apply` token utilities; an open universe card spans the full row. `routes.ts` holds pure landing/editor URL helpers (keeps `apiBaseUrl`) and the stored-scope parser. The editor's left-rail Library button saves (flush) and returns to landing. |
| `components/ErrorBoundary.tsx` | React error boundary with ADT state, crash isolation per component. See [error boundary context](.claude/context/error-boundary.md) |
| `ui/StickyNote.tsx` | Shared native textarea (tones yellow, pink, green; `overhang` widens 4px each side; `readOnly` removes resize) with yellow Story Context, Continue instruction, Thread continuation prompt, and author guidance styling; pink remains the rail Edit note. Forwards native props and ref. Enter and Apply share selection/readiness checks; Shift+Enter adds a newline and Esc cancels. Guidance saves explicitly. |
| `ui/Button.tsx`, `IconButton.tsx`, `ListButton.tsx`, `TextField.tsx`, `Toggle.tsx`, `Field.tsx`, `Heading.tsx`, `Alert.tsx`, `cx.ts` | Design-system primitives, each exporting a pure `*ClassName` function and forwarding native props and ref; unit tests in `ui/primitives.test.tsx`. Component page `e2e/fixtures/ui-components/` is not shipped; `e2e/ui-components.spec.ts` renders every variant and screenshots to `target/guz-874/<browser>/` and `target/guz-884/<browser>/`. See [design system](docs/design-system.md) |
| `frontend.tsx` | React root setup, CSS import, StrictMode |
| `ui/tokens.css` | Design tokens: palette, `--placeholder`, seven named shadows, font stacks, and Tailwind 4 `@theme inline` mappings for colours, shadows, six type sizes, radii 0/1/2/4/7/10/999 and 4px spacing; Tailwind default colours, shadows, text sizes and radii are reset. Imported by `index.css`. See [design system](docs/design-system.md) |
| `ui/ALLOWLIST` | `cargo xtask lint` `ui` check debt: `path count rule` lines; lower or remove a line when you fix its violations |
| `index.css` | Tailwind + `ui/tokens.css` import, editor shell layout and responsive styling. Right drawer uses 12px labels/eyebrows and 13px body/controls/placeholders, retaining 12px mono prompt text. Desktop drawers span 100dvh with contained internal scrolling; the right drawer heading stays pinned above a thin divider. Neither drawer has a Close button. Manuscript and Context rail toggles stay unmuted with ink text on green while open. |
| `i18n/messages.ts` | Pure `en`/`es` UI message tables and `formatMessage` (`Intl` plurals, interpolation, numbers, dates, currency). English is the key source; Spanish is written to match it. UI language is separate from series language. See [i18n](docs/i18n.md). |
| `md_editor/types.ts` | TypeScript types: Delta, Range, EmitterSource, event callbacks |
| `md_editor/ai_contract.ts` | Pure AI request contract with optional model ID pass-through (absent when omitted), visible-only matched lore assembly, and browser-safe API URL helper. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/model_catalog.ts` | Pure public catalog parsing and per-action model resolution. The right drawer's Advanced AI controls offer persisted Continue/Edit selects from GET /ai/models; missing/unknown IDs use server action defaults. Failed/empty catalogs disable selects and omit model from requests. |
| `md_editor/cost.ts` | Pure actual-usage cost ADT: request-time model/pricing snapshot, latest usage only, exact bigint rational USD or explicit unknown. `inspector_cost.ts` adds separate input-only pending estimates, output rates, display rounding, and request-scoped actual usage for the inspector. See [cost policy](.claude/context/ai-editor.md#actual-usage-cost-policy). |
| `md_editor/prompt_sections.ts` | Pure server-parity prompt sections from accepted AI requests; labeled UTF-8 byte token estimates and summed totals. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/context_inspector.ts` | Pure live Continue and Edit context view-model; each preview uses its action's instruction. Continue shows "with thread" for effective history. Injected lore reports keyword/pin reasons, levels, standalone token estimates, and budget skips using the shared matcher. The right drawer also holds Story Context, Continue instruction, AI Thread/Reset Thread, and advanced AI controls. Desktop Settings holds Window Size, Whole document, Show Continue window, and Tint AI text; window controls remain in the drawer below 1100px. It replaces the separate Preview Context UI. System, saved guidance, and derived Lore are read-only; other content and speaker turns support edits/reset. Inline below 1100px. |
| `md_editor/editor_panels.ts`, `md_editor/components/ManuscriptTree.tsx` | Pure single-drawer state and Ctrl/Cmd bracket shortcut mapping; universe → container → piece outline (book series novels via the façade) with save-before-switch chapter navigation and a single-chapter GET through the manuscript client. Failed saves keep the current chapter. Active chapters scroll only the outline on load, selection, and creation without moving focus (`tree_scroll.ts`, `tree-active-scroll.spec.ts`, 1100px/1440px). `manuscripts/create_draft.ts` handles inline + Universe/Container/Piece/Loose piece/Chapter/Scene drafts, membership label options (book series novel/novella, TV series episode/TV script, collection any root label), label-driven shape and format defaults (screenplay/TV script/episode start as script), and append positions; façade novels add chapters through the chapter route. `ListButton`/`TextField`/`Field`/`Button` reuse the design system. `create-work.spec.ts` creates a loose piece, a book-series novel, a collection novel and a script TV episode, then checks reload and server GETs at 1100px/1440px (drawer screenshots in `target/guz-941/<browser>/`). `manuscript-create.spec.ts` covers creation, cancellation, save-before-switch and reload persistence at 1100px/1440px; screenshots in `target/guz-891/<browser>/`. Manuscript and Context rail toggles expose `aria-expanded`; a second click or Escape closes and restores editor selection/focus (except Escape within a create draft cancels only that draft). Manuscript rail browser coverage at 1100px and 1440px captures open/closed states in `target/guz-798/`. |
| `../fixtures/prompt/parity.json` (repo root) | Shared prompt section ids, tags, text, and history fixtures replayed by `crates/api/tests/prompt_parity_test.rs` and `app/src/md_editor/prompt_parity.test.ts`; provider model names pass through unchanged. |
| `md_editor/ai_stream.ts` | Pure incremental NDJSON parser for AI stream events, including `usage`. The editor shell ignores that event. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/ai_state.ts` | AI run-state ADT and transition helpers. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/window.ts` | Pure context-window calculation for transmitted document slices. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/lore_match.ts` | Pure name/alias/keyword matcher; carries every level unchanged and exposes no hidden notes. Whole words use NFC, Unicode simple case folding (no ß/SS expansion), and edges without letters, marks, numbers, connector punctuation, emoji modifiers, or ZWJ (including emoji/CJK terms). Budget counts name + visible text UTF-16 units: pins always included and charged first, then input-order matches, skipping optional entries that do not fit; pins may exceed the cap and then suppress all optional matches. |
| `md_editor/components/ContinueWindow.tsx` | Desktop live Continue marker from exact calculated window start to end, including context after the caret and full length in Whole document mode. DOM geometry follows author edits and concurrent streams; the top-viewport scroll flag reports integer viewport heights or just above below one screen. Settings persists Show Continue window alongside Tint AI text; neither marker changes the transmitted slice or recenters the caret. |
| `md_editor/delta_bridge.ts` | Quill delta normalization at the request and stream boundaries. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/editor_ai.ts` | Pure selection-memory, shared Enter/Apply availability, rollback, and status helpers. `editReplacementLength` (embeds count 1) leaves a selection's trailing `\n` out of the Edit range, so Edit replaces only the text before it and that line ending keeps its attributes; line endings inside a multi-line selection are still replaced. Header save labels omit the repeated title and keep save/recovery failures prominent. See [AI editor context](.claude/context/ai-editor.md) |
| `manuscripts/contract.ts`, `manuscripts/client.ts` | Typed manuscript hierarchy and revision API client with response parsing and structured errors. |
| `settings/client.ts` | Typed author guidance GET/PUT client with response parsing and structured errors; inspector loads saved server text on mount and saves explicitly. |
| `md_editor/chapter_sync.ts` | Pure revision sync core: one in-flight Delta, composed pending edits, server-priority transforms, and acknowledgement reconciliation. |
| `md_editor/script_layout.ts` | Pure script surface map, no Quill: `composeScenes` puts one `\n` break line (`scene-break: <sceneId>`, length 1) before every scene in caller order and gives a scene without a trailing `\n` one; `splitChange` returns per-scene Deltas plus the new layout, or `crossesBreak` when a change deletes, formats, or inserts at a break, deletes a scene's last `\n`, inserts past the end, or carries the break attribute; `placeRemote` shifts a scene change by the scene start; `toScenePosition`/`fromScenePosition` map offsets (null when unmapped). Seeded property test replays with `QUILLAI_LAYOUT_SEED=982 bun test --cwd app src/md_editor/script_layout.property.test.ts`. See [script surface](docs/screenplay-surface.md) |
| `md_editor/use_script_sync.ts`, `script_poll.ts`, `scene_break.ts`, `script_keys.ts` | Script group surface (opened by `App.tsx` when `scriptGroupId` in `landing/routes.ts` finds a script group or scene): one `createChapterSession` per scene through an adapter client, one `GET /pieces/{groupId}/scenes` on load, one batched scenes poll per tick, `placeRemote` for remote edits, read-only `SceneBreak` title lines never saved, edits across a break reverted with a notice, per-scene save status and recovery; markdown shortcuts off. Paste and cut over a cross-break selection and a drop on a break line are refused in capture before Quill applies them (Quill already prevents in-editor drag starts and text drops), so undo history survives; the late revert stays for anything else. `script_keys.ts` (pure) maps keys to pass/move/block: ArrowLeft/Right step over a break, Backspace at a scene start, Delete at a scene end and editing keys over a cross-break selection are blocked; a caret landing on a break moves into the scene it came toward. Blocked keys, cross-break paste/cut/drop and reverted edits show an `Alert` pill until the next accepted edit; the header status does not repeat it. At 1100px and up the pill sits absolute under the top bar inside the sheet's top padding, so it neither shifts the sheet nor covers prose. AI: Continue and Edit start only when every scene session is clean, the group poll pauses while AI runs, Continue anchors at the caret scene's trailing `\n`, and `editRejection` disables Edit (Apply plus a reason in the pink note) for a selection that crosses a break. The opening `pieceId` scene (or the first scene) gets the caret. In the manuscript tree a scene of the open script moves the caret into that scene in place (`focusScene`), and the tree highlights the scene holding the caret (`treeScope`); the top bar also holds a Block type select (`TextField as="select"`, reflects the caret line; refuses the break line and cross-break selections) and a Download Fountain link (`fountain()` over the layout's current scene Deltas, file `<group title>.fountain`); in `script_keys.ts` a space after a list prefix (`- `, `1.`) stops Quill's list autofill, and Tab, Shift-Tab and plain Enter are handled in the capture `keydown` after the break checks; the top bar's New scene button (`addScene`) creates an empty scene after the caret's scene (one `PUT /pieces/{id}` position per later scene from last to first, then `POST /pieces/{id}/children` at the caret scene's position + 1), flushes and reloads the surface on that scene; any failure shows a "New scene not added" Alert with no navigation and the order intact (at most a position gap); the poll's `children` differing from the surface's scene order shows a persistent "Scenes changed; reload" Alert (no rebuild); another script's scene or a script group (summary button, including the open one) opens the surface by full navigation after a flush (`leave(target)`); prose still switches in place. See [script surface](docs/screenplay-surface.md) |
| `md_editor/script_blocks.ts`, `md_editor/fountain.ts`, `md_editor/fdx.ts` | Pure script block formats and export: `script-block` line attribute on a line's `\n` for scene-heading, character, dialogue, parenthetical and transition (action = no attribute, so unformatted and AI text read as action); Tab/Shift-Tab cycle order, Enter successor (`splitLineDelta` is one Delta that keeps the line's type and gives the new line its successor), `scriptLines` and `toFountain` (uppercases headings/characters/transitions, forces `.` `@` `>` `!` only when needed, joins character/parenthetical/dialogue lines, skips empty lines and embeds; unit test `fountain.test.ts`). `toFdx` writes the same non-empty lines as a Final Draft `.fdx` (`<FinalDraft DocumentType="Script" Version="5">`, one `<Paragraph Type>` per line, `&<>"` escaped, XML-illegal control characters dropped, a parenthetical bracketed when it has no brackets; unit test `fdx.test.ts`); the top bar's Download FDX link sits beside Download Fountain (`<title or "script">.fdx`). The `ql-script-block-*` ClassAttributor is registered in `scene_break.ts`; its styles in `index.css` (monospace, uppercase, indents) only match script lines. |
| `md_editor/selection_boundary.ts` | Pure UTF-16 endpoint correction: moves an index inside a surrogate pair to its start; remote sync applies it to both transformed selection ends. |
| `md_editor/chapter_sync.property.test.ts` | Seeded two-client/server convergence below the browser: 12 fixed seeds, 400 codepoint-safe edits per run, random send/commit/remote/ack interleavings. Replay with `QUILLAI_SYNC_SEED=729 bun test --cwd app src/md_editor/chapter_sync.property.test.ts`. |
| `md_editor/chapter_session.ts`, `md_editor/use_chapter_sync.ts` | Chapter selection/bootstrap, read-only selected chapter title, serialized save/poll shell, and per-tab unsaved recovery; failed POST recovery waits for the next existing one-second background poll, and flush cannot bypass that wait. Clean Continue reserves the session before awaiting an active poll, excludes later ticks, and retains Stop and unsaved rejection (`e2e/continue-poll.spec.ts`). Server chapter content/head_rev is authoritative on reload. |
| `md_editor/editor_persistence.ts` | Pure parse/serialize of local AI settings and thread under `quillai.editor.state.v1`, including separate `editInstruction`/`continueInstruction` and `threadState.settings.continueModel`/`editModel`. Legacy `instruction` loads into Continue only; Edit starts empty. The editor writes an empty document placeholder and never restores chapter content from this key. Invalid data falls back to defaults. |
| `../fixtures/delta/` | Language-neutral Delta fixtures generated by `quill-delta` (`cd app && bun run fixtures:delta`), replayed by `src/delta_fixtures/delta_fixtures.test.ts`; also replayed by `crates/delta/tests/fixtures.rs` (`cargo test -p quillai_delta`). |
| `md_editor/markdown.ts` | Pure markdown analysis functions — `analyzeMarkdownContext`, `isPatternComplete`. See [markdown parsing context](.claude/context/markdown-parsing.md) |
| `md_editor/prose.ts` | Pure passive-voice detector — `analyzeProse` returns UTF-16 `Hint` spans from a be-verb through a following participle. |
| `md_editor/components/Editor.tsx` | Quill Continue/Edit shell with separate pink-note Edit and drawer Continue instructions, used only by their matching preview/request. Desktop Continue (>=1100px) permits concurrent typing, grows below the original caret paragraph, preserves the logical caret/selection without recentering, and rolls back only surviving AI text on Stop/cancel/failure. `continue_stream.ts` transforms its anchor, append end and exact ownership through author edits; `continue_tint.ts` renders local-only tint with a completion fade and persisted on/off control in the bottom-gear Settings popover alongside Window Size and Whole document. Edit and narrow-screen Continue retain locked-editor behavior. Edit deletes and restores only the `editReplacementLength` range; when it keeps a line ending it holds back a trailing `\n` of the streamed replacement and drops it at the end, so no blank line is added. Writing controls live in the right drawer; Export local chapter lives in the left drawer. The flowing header centers quiet save status above the sheet; save/recovery failure shows a floating red "Not saved" pill without shifting the sheet or covering prose. The full reason remains in its tooltip and screen-reader description. See [AI editor context](.claude/context/ai-editor.md) and [markdown parsing context](.claude/context/markdown-parsing.md) |
| `api_tester/components/APITester.tsx` | HTTP method selector + URL input + loading state + structured error parsing. See [error handling context](.claude/context/error-handling.md) |
| `api_tester/components/__tests__/APITester.test.ts` | Unit tests for `buttonLabel()` pure function |

The editor loads universe lore (`/universes/{id}/lore`) through `manuscripts/lore_client.ts` and `use_lore.ts`: a piece uses its scope's `universeId`, a chapter resolves its series via `GET /containers/{seriesId}`; pieces open in any universe, with or without a book series (`e2e/lore-scope.spec.ts`, 1100px). The left manuscript drawer's `LorePanel.tsx` lists, creates, edits, and deletes author lore, including levels and pins. Desktop reserves min(320px, 40dvh) for all manuscript tree states with internal scrolling; lore scrolls separately and Export local chapter stays fixed below it (`tree-load-shift.spec.ts`, screenshots in `target/guz-820/`). `lore_panel.ts` contains pure draft, term, and list helpers. Universe changes clear stale entries; failed loads offer retry. A caller-supplied `loreEntries` list remains available for isolated tests. Lore stays derived/read-only so request-local text overrides cannot remove its non-reveal marker. Hidden notes remain author data, not AI context. Desktop `LoreWindow.tsx` uses the inspector's shared lore model for visual-only underlines, read-only hover cards, and the budgeted Continue-only "Lore in window" list. Missing `CSS.highlights` or `Highlight` disables only underline/hover coloring; DOM-range hover cards, list navigation, Alt+L, and Escape remain functional. Tests simulate missing APIs and run in the Chromium, Firefox and WebKit projects. It never changes Quill formats or saved content; browser proof at 1100px and 1440px goes to `target/guz-759/<browser>/`. See [lore prompt contract](.claude/context/ai-streaming.md#injected-lore).

### Database

`migrations/20250407204144_init.up.sql` creates `users` (id, name, email). `migrations/20261019000000_series_books_chapters.up.sql` adds `series` → `books` → `chapters` with integer IDs, titles, positions, and cascading parent foreign keys. Chapters restrict kind to `chapter`, `prologue`, or `epilogue`; content stores valid JSON text with an empty Delta (`{"ops":[]}`) default. Parent/position indexes support ordered child queries. The down migration drops children first and preserves users.

Startup applies embedded migrations before serving and explicitly enables SQLite foreign keys. Integration tests share `crates/api/tests/common/mod.rs`, which creates isolated, single-connection in-memory pools and runs the same migrations. `crates/api/tests/book_schema_test.rs` covers cascades, orphan rejection, kinds, position ordering, Delta storage, and rollback/reapply.

`migrations/20261019000001_chapter_revisions.up.sql` adds retained `chapter_revisions` (chapter FK cascade, unique chapter/revision, Delta JSON, creation timestamp) and chapter `head_rev`/`snapshot_rev` markers. `chapters.content` is the snapshot Delta at `snapshot_rev` (existing content starts at revision zero). `RevisionStore` takes a nonzero snapshot interval, replays revisions strictly above the snapshot in revision order, and atomically appends against an expected head. HTTP sync snapshots every 100 revisions, retains all revisions, transforms concurrent submissions with server priority, and retries optimistic head conflicts. GET detail, GET list with `?content=true`, and PUT responses rebuild snapshot plus revisions within the same transaction as the chapter metadata read/write. Default GET list selects only chapter headers without reading content or revisions. Sync accepts a base/since only when every subsequent revision through the head is retained; the earliest retained revision minus one is the floor for contiguous history, independent of snapshot_rev. Missing required revisions, negative bases, and bases above the head return 409. `crates/api/tests/chapter_revisions_test.rs` covers rebuild, snapshots, stale heads, rollback, and constraints; `chapters_test.rs` covers HTTP sync, concurrency, convergence, input rejection, and parent scoping.

`migrations/20261019000002_author_guidance.up.sql` adds a singleton `author_guidance` table (id constrained to 1, text capped at 65,536 UTF-8 bytes). Missing row and empty text both mean no guidance. PUT upserts the one row; the down migration drops only this table.

`migrations/20261020000000_lore_entries.up.sql` adds series-scoped `lore_entries` with cascading deletion and a series index: kind (`character`, `place`, `term`, `note`), nonblank name, separate `keywords` and `aliases` JSON-array text columns (each default `[]`, preserving search terms versus alternate names), visible text and hidden notes (default empty), pinned (0/1, default 0), and AI-request level (`include`, `known_dont_reveal`, `leave_out`, default `include`). The down migration drops only `lore_entries`; `crates/api/tests/lore_schema_test.rs` covers storage, defaults, constraints, cascades, and rollback/reapply.

`migrations/20261025000000_story_model.up.sql` (`docs/story-model.md` §1) replaces `series`/`books`/`chapters`/`chapter_revisions` with `universes`, `containers` (book_series/tv_series/collection), `pieces` (leaf/group, label, format, `legacy_book_id`/`legacy_chapter_id`), and `piece_revisions`; `lore_entries` becomes universe-scoped. Triggers enforce parent, container, and membership rules and assign legacy ids from `legacy_ids` on insert. Views `legacy_series`, `legacy_books`, `legacy_chapters` back the old routes as a permanent façade. The down migration aborts unless every row fits series/books/chapters. `crates/api/tests/story_model_migration_test.rs` covers up/down round trips; old schema tests undo to 20261020000000 first.

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
- Old `/series`, `/books`, `/chapters`, deltas, and lore routes are a façade over the story model with unchanged JSON: series are `book_series` containers, books are prose `novel` groups (id = `legacy_book_id`), chapters are their prologue/chapter/epilogue leaves (id = `legacy_chapter_id`), and series lore is the container's universe lore (shared by series in one universe). `POST /series` creates a universe plus container; `DELETE /series/{id}` deletes the universe too when it has no containers or pieces left
- `GET/POST /universes`, `GET/PUT/DELETE /universes/{id}` — title/position, series rules (201, 404, 422); DELETE cascades containers, pieces, lore
- `GET/POST /universes/{id}/lore`, `GET/PUT/DELETE /universes/{id}/lore/{lore_id}` — lore fields as series lore, JSON `universe_id`
- `GET/POST /universes/{id}/containers`, `GET/PUT/DELETE /containers/{id}` — `{id, universeId, title, position, kind}`; `kind` or `universeId` on PUT is 422
- `GET /universes/{id}/pieces?container=none` — loose roots; other or missing `container` is 422. `POST /universes/{id}/pieces` — root in storage `{title, position, label, shape, format?}`; format defaults to script for screenplay/tv_script/episode, else prose; a leaf starts with an empty Delta
- `GET/POST /containers/{id}/pieces` — root pieces; book_series allows novel/novella, tv_series tv_script/episode (422 otherwise)
- `POST /pieces/{id}/children` — new empty leaf under a group with the group's format; label defaults to chapter (prose) or scene (script); non-group is 422
- `GET/PUT/DELETE /pieces/{id}` — leaf adds `content`/`headRev`, group adds `children`; PUT changes title/position/label/format (group format cascades to children); `universeId`, `containerId`, `parentId`, `shape`, `content`, a child format change, or a membership break is 422
- `POST/GET /pieces/{id}/deltas` — leaves only (group 404); same JSON and 409 rules as chapter deltas
- `GET /pieces/{groupId}/scenes` — a script group opened in one GET: the group's piece fields plus `scenes: [{...piece, content, headRev}]`, every child in `position, id` order, each rebuilt like a leaf `GET /pieces/{id}` in one transaction (200, 404 missing, 422 for a leaf or a prose group)
- `GET /pieces/{groupId}/scenes/deltas?since=<sceneId>:<rev>,...` — one poll for a script group: `{children: [id...], scenes: {"<id>": {revisions: [{rev, delta}]} | {conflict: "future" | "unavailable"}}}`. `children` is the current order; `scenes` holds only the requested ids (empty `since=` asks for none). Per-scene leaf history rules; a scene with future/negative or missing history gets `conflict` and the others still return (200). A scene not in the group, a repeated id, a malformed or missing `since` is 422; missing group 404, leaf or prose group 422. Posts stay on `/pieces/{id}/deltas`
- `POST /pieces/{id}/placement` — `{containerId: number|null, position}`; a root moves to that container or storage (null); a child accepts only null (detach: clears `parentId`). Returns the piece detail (200, 404, or 422 for another universe, a missing container, a membership break, or a child with a container)
- `POST /pieces/{id}/split` — `{label?}`; a root leaf becomes the position-0 child of a new group that copies its universe, container, title, position, label, and format; child label defaults to chapter (prose) or scene (script). Returns the group detail with children (201, 404, or 422 for a group, a child, or an unknown label)
- `POST /pieces/{groupId}/attach` — `{pieceId, position}`; a root leaf in the same universe and format becomes a child of that group, clearing `containerId` in the same UPDATE. Returns the group detail (200, 404 for a missing group, or 422 for a non-group target, a missing/group/child source, another universe, or another format)
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
- Every ticket fixing or changing browser-visible behaviour ships with an E2E regression spec.

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
- Workers run `cargo xtask lint` plus only the specs they add or change, in Chromium (`cargo xtask e2e e2e/<spec> --project chromium`; `--project` is variadic, so spec paths go first); the full three-browser suite is the owner's check, run out of band.
- MSRV is Rust 1.95.0 or higher, set as `rust-version` in the workspace `Cargo.toml` and inherited by every member crate.
- Workspace crates under `crates/`. New crates go there.
- `cargo xtask` for build automation — extend `xtask/src/main.rs` for new tasks.
- Tracked `.rs` files under `crates/` and `xtask/` must stay at or below 1000 lines (enforced by `cargo xtask lint`).
- SQLx with compile-time checked queries (`.sqlx/` cache). `.cargo/config.toml` sets `SQLX_OFFLINE=true` so builds use that cache; a shell `SQLX_OFFLINE=false` overrides it. See [SQLx offline context](.claude/context/sqlx-offline.md).
- Errors via `thiserror` with manual `IntoResponse` for JSON responses. See [error handling context](.claude/context/error-handling.md).

### Frontend Conventions

- Read [docs/design-system.md](docs/design-system.md) before adding or changing UI. Use `app/src/ui/` primitives and `app/src/ui/tokens.css` tokens; do not add raw controls, hex colours, font sizes or Tailwind arbitrary values elsewhere (`cargo xtask lint` check `ui` enforces this).
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
