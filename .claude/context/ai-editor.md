# AI Editor — Quill Shell, Helpers, and Browser URL Constraint

## Overview

The AI editor lives under `app/src/md_editor/`. Quill holds the visible document; the manuscript API holds the saved chapter. Pure helpers handle revision transforms and AI request/stream logic. `components/Editor.tsx` coordinates selection capture, AI streaming, cancellation, and rollback.

## Architecture

The frontend split is deliberate.

**Functional Core**:
- `ai_contract.ts` — request types, stream-event types, URL helper, selection rebasing.
- `ai_stream.ts` — incremental NDJSON parser plus final remainder handling.
- `ai_state.ts` — AI run-state ADT and transition helpers.
- `window.ts` — context-window calculation.
- `delta_bridge.ts` — Quill delta normalization at the contract boundary.
- `editor_ai.ts` — pure selection-memory, rollback, and status-presentation helpers.
- `chapter_sync.ts` — server document/revision, one in-flight Delta, composed pending Delta, server-priority remote transforms, and acknowledgement reconciliation.
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

AI and manuscript requests share `app/src/api_config.ts`. Its pure `resolveApiBaseUrl(override)` defaults to `http://127.0.0.1:2908` and accepts an absolute HTTP(S) URL, including random ports. It trims whitespace and trailing slashes, and rejects credentials, queries, and fragments.

Set the frontend URL's `apiBaseUrl` query parameter to override the API URL, e.g. `http://localhost:3001/?apiBaseUrl=http%3A%2F%2F127.0.0.1%3A49152`. The browser shell passes this setting to the pure resolver. Chapter selection preserves the parameter. Reload after changing it. This uses the browser's native URL support with Bun-served bundles in development and production: no config endpoint, injected script, rebuild, or browser-side `process.env` (which previously caused `process is not defined`). The E2E sandbox can select any API port with its initial navigation URL.

The current backend CORS setup still assumes the frontend is served from port `3001`; API port configuration does not change that restriction.

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
- `ai_stream.test.ts`
- `ai_state.test.ts`
- `window.test.ts`
- `delta_bridge.test.ts`
- `editor_ai.test.ts`
- `chapter_sync.test.ts`
- `chapter_session.test.ts`

There is still no direct component-level test for the full Quill streaming shell, so browser QA remains important for Continue/Edit trigger regressions.
