# AI Editor — Quill Shell, Helpers, and Browser URL Constraint

## Overview

The AI editor lives under `app/src/md_editor/`. Quill holds the visible document; the manuscript API holds the saved chapter. Pure helpers handle revision transforms and AI request/stream logic. `components/Editor.tsx` coordinates selection capture, AI streaming, cancellation, and rollback.

## Architecture

The frontend split is deliberate.

**Functional Core**:
- `ai_contract.ts` — request types, stream-event types, URL helper, selection rebasing.
- `context_inspector.ts` — pure prompt-section view-model with summed estimated tokens and ordered history turns; the editable live panel selects Continue/Resume or Edit and uses the same request builder as sending. It refreshes for Quill/settings/thread changes, handles unavailable previews, and hides at 800px. Only edited ids enter `sectionOverrides`; Reset restores assembly per section. Each action owns separate overrides, cleared after its accepted send or any chapter load/remote reset, never persisted. History is edited per speaker turn, with explicit user/assistant add controls. Editing alone sends no request.
- `prompt_sections.ts` — pure server-parity assembly from accepted requests. `editableText` is raw section content; `text` is provider-rendered content. Text overrides pass through normal escaping and fixed tags/action/selection metadata; empty text keeps its section frame. History overrides are ordered `{role, text}` turns, preserving roles and message boundaries without tagged framing. Both normal and edited history escape `&`, `<`, and `>` before provider delivery and token estimation; normal history is trimmed first, while edited history preserves whitespace. `promptHistory` exposes raw turns for editing; `assemblePromptHistory` exposes escaped provider messages. History display concatenates escaped turn text; current-turn sections join without separators. Token estimates are `ceil(UTF-8 bytes / 4)` per section, summed for totals, excluding chat framing, not tokenizer counts.
- `ai_stream.ts` — incremental NDJSON parser plus final remainder handling.
- `ai_state.ts` — AI run-state ADT and transition helpers.
- `window.ts` — context-window calculation.
- `delta_bridge.ts` — Quill delta normalization at the contract boundary.
- `editor_ai.ts` — pure selection-memory, rollback, and status-presentation helpers.
- `chapter_sync.ts` — server document/revision, one in-flight Delta, composed pending Delta, server-priority remote transforms, and acknowledgement reconciliation.
- `document_diff.ts` — sync document differences skip fast-diff's semantic cleanup because it can shift an edit into a surrogate pair; equal-text Delta diff then reconciles attributes and embeds.
- `editor_persistence.ts` — pure parse/serialize of localStorage AI controls and thread settings; the editor writes an empty document placeholder, never restores chapter content from this key, and resets the AI thread when a chapter loads or remote content changes.

**Imperative Shell**:
- `components/Editor.tsx` — owns Quill, DOM refs, browser `fetch`, `AbortController`, streamed delta application, and UI state wiring.
- `chapter_session.ts` — hierarchy selection/bootstrap and serialized POST/GET orchestration.
- `use_chapter_sync.ts` — Quill synchronization, one-second polling, save status, unload warning, and per-tab sessionStorage recovery/export.

## Chapter Loading And Saving

- A URL with `seriesId`, `bookId`, and `chapterId` opens that scoped server chapter. All three IDs must be positive integers.
- Without IDs, selection walks the ordered series/books hierarchy and opens its first chapter. The selected scope is written to the URL, so reload opens the same server chapter with its current content and `head_rev`.
- An empty hierarchy offers **Create chapter**. Only this explicit action creates missing default series/book/chapter resources through the manuscript API. Concurrent bootstrap actions can create separate defaults; no backend uniqueness is assumed.
- All local document changes enter the same sync path: typing, formatting, undo/redo, markdown conversion, AI selection deletion, streamed fragments, and cancellation/failure restoration. Markdown's silent replacement is captured by comparing the final Quill document with the sync core's visible document.
- Quill's document normalization, including the required terminal newline, enters that same local path after chapter load or a remote update.
- Each one-second cycle permits at most one POST `{baseRev, delta}`, then GETs revisions since the last acknowledged server revision. Polling never overlaps a submission or another poll. New edits compose into pending work while network I/O is active.
- The POST response identifies the client's revision in the returned history. Earlier remote revisions transform both in-flight and pending work with server collision priority. The own revision acknowledges rather than reapplies the local edit. Later remote revisions transform remaining pending work.
- AI starts only when local work is saved and no network cycle is active. Polling/sending pauses during the AI run to keep stream anchors and rollback ranges stable. The complete resulting edit enters the next save cycle.
- An uncertain POST failure stops saving without resubmission: the API has no idempotency key. Protocol/history/HTTP errors also stop saving and make the editor read-only. GET network failures retain the acknowledgement and retry history only. Local work remains available through **Export local chapter**.
- Unsaved visible documents and chapter scope are copied to per-tab sessionStorage. Reload archives any previous unsaved copy and loads the server chapter, never silently replays uncertain local changes. **Export recovery** downloads archived copies. An unload warning protects unsaved work; unavailable/full recovery storage is shown in save status. Closing the tab ends sessionStorage retention, so export before closing.

## Browser URL Constraint

AI and manuscript requests share `app/src/api_config.ts`. Its pure `parseApiBaseUrl(override)` returns a typed result (`default`, `override`, or `rejected` with a reason) and never throws; `resolveApiBaseUrl(override)` returns its URL. The default is `http://127.0.0.1:2908`. An override must be an absolute HTTP(S) URL whose host is loopback (`127.0.0.1`, `localhost`, or `[::1]`), on any port; whitespace and trailing slashes are trimmed. Malformed values, other schemes, non-loopback hosts (including `127.0.0.1.evil.example` and userinfo tricks such as `http://127.0.0.1@evil.example`), credentials, queries, and fragments all fall back to the default, so chapter text cannot be sent off-machine.

Set the frontend URL's `apiBaseUrl` query parameter to override the API URL, e.g. `http://localhost:3001/?apiBaseUrl=http%3A%2F%2F127.0.0.1%3A49152`. `getApiBaseUrl()` in the same module reads `window.location.search` when called (`buildAiGenerateUrl()` per AI request, `createManuscriptClient()` when the client is created; returning the default when no `window` exists) and passes the value to the pure resolver; the editor shell and clients do not pass it in. Chapter selection preserves the parameter. Reload after changing it. This uses the browser's native URL support with Bun-served bundles in development and production: no config endpoint, injected script, rebuild, or browser-side `process.env` (which previously caused `process is not defined`). The E2E sandbox can select any API port with its initial navigation URL.

The current backend CORS setup still assumes the frontend is served from port `3001`; API port configuration does not change that restriction.

## Browser Smoke Sandbox

Run `cargo xtask e2e` from the workspace. On first use, install Chromium with `cd app && bun --bun x playwright install chromium`. The runner installs app dependencies if Playwright is missing, builds the API, creates a temporary on-disk SQLite database, and starts the API on a random loopback port. Frontend port 3001 is mandatory for existing CORS; an occupied port fails before startup. No CORS configuration changes are made.

The runner waits for HTTP 200 from API `/series` and frontend `/`, then runs `bun --bun x playwright test` from `app/` with the config in root `e2e/`. Additional flags pass through unchanged: `cargo xtask e2e --headed --trace on`. Tests supply `apiBaseUrl` and chapter IDs on both page URLs. The chapter smoke checks sequential edits, convergence, saved revisions/content, and reload. Emoji coverage adds concurrent append/replacement/pending edits and six seeded runs of 50 rapid, codepoint-safe Quill clipboard pastes per tab with delayed network I/O; both tabs must converge with all inserted codepoints retained and no HTTP or console errors. AI remains outside this browser coverage. Browser tests live outside `app/` to keep `bun test` separate. `e2e/package.json` marks this directory as ESM for Bun's Playwright loader; imports reuse the dependency installed under `app/node_modules/`. Failure screenshots and traces are under `target/xtask-e2e/results/`.

Services, build/install commands, and Playwright run in managed process groups. Normal exit, startup/test failure, and Ctrl-C terminate those groups before removing the database directory and SQLite sidecars. The random API port is released before API bind; a competing bind fails startup rather than silently reusing another service. Port 3001 is reserved during setup and released immediately before frontend startup.

## Continue And Edit Flow

`Continue`:
- resolves the current caret or remembered caret
- builds a windowed request
- starts `fetch`
- applies streamed text-only delta fragments at the insertion anchor

`Edit`:
- resolves the current non-empty selection or remembered pre-blur selection
- deletes the selected text when the stream starts
- inserts streamed text at the original anchor
- restores or rolls back the original text depending on cancellation or failure timing

## Cancellation And Failure Rules

- While a run is active, the editor becomes read-only.
- Local abort keeps already inserted text.
- Cancel before first inserted token restores the original selection.
- Failed edits roll the original text back into place.
- Failed generate runs keep any text already inserted.

## Stream Contract Expectations

The frontend expects NDJSON events with Quill-delta payloads, but the editor shell currently accepts text inserts only from streamed `delta` events. Non-text embed inserts are rejected at the shell boundary, and request slices containing embeds are rejected before `fetch` starts.

## Tests

```sh
bun test --cwd app
sh -lc 'cd app && bun x tsc --noEmit'
bun run --cwd app build
```

Pure helper coverage currently lives in:

- `ai_contract.test.ts`
- `prompt_parity.test.ts` — replays `fixtures/prompt/parity.json` with Rust `crates/api/tests/prompt_parity_test.rs`, checking section ids/tags/text, normalized history, resume/edit behavior, Unicode, and model-independent prompts against captured provider requests.
- `ai_stream.test.ts`
- `ai_state.test.ts`
- `window.test.ts`
- `delta_bridge.test.ts`
- `editor_ai.test.ts`
- `chapter_sync.test.ts`
- `chapter_session.test.ts`

There is still no direct component-level test for the full Quill streaming shell, so browser QA remains important for Continue/Edit trigger regressions.
