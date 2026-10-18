# AI Editor — Quill Shell, Helpers, and Browser URL Constraint

## Overview

The AI editor lives under `app/src/md_editor/`. The browser UI uses Quill as the document source of truth, pure helper modules for request and stream logic, and `components/Editor.tsx` as the imperative shell that coordinates selection capture, `fetch`, stream application, cancellation, and rollback.

## Architecture

The frontend split is deliberate.

**Functional Core**:
- `ai_contract.ts` — request types, stream-event types, URL helper, selection rebasing.
- `ai_stream.ts` — incremental NDJSON parser plus final remainder handling.
- `ai_state.ts` — AI run-state ADT and transition helpers.
- `window.ts` — context-window calculation.
- `delta_bridge.ts` — Quill delta normalization at the contract boundary.
- `editor_ai.ts` — pure selection-memory, rollback, and status-presentation helpers.
- `editor_persistence.ts` — pure parse/serialize of the localStorage editor snapshot (document, story context, instruction, window settings, thread state); invalid data falls back to defaults.

**Imperative Shell**:
- `components/Editor.tsx` — owns Quill, DOM refs, browser `fetch`, `AbortController`, streamed delta application, and UI state wiring.

## Browser URL Constraint

The browser-side Continue and Edit trigger path currently uses a fixed local API URL:

- `http://127.0.0.1:2908`

The current local development CORS setup also assumes the frontend is served from port `3001`.

This is intentional. The browser shell no longer reads `process.env` directly, because that caused the QA failure `process is not defined` before `fetch` started. The URL helper in `ai_contract.ts` is now pure and returns the default URL with no browser-side override parameter.

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

There is still no direct component-level test for the full Quill streaming shell, so browser QA remains important for Continue/Edit trigger regressions.
