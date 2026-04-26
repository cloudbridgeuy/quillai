# quillai_app

React and Quill frontend for QuillAI.

## Install

```sh
bun install
```

## Run

Development server:

```sh
bun dev
```

Production server:

```sh
bun start
```

## Environment

| Env var | Default | Description |
|---------|---------|-------------|
| `PORT` | `3001` | Bun frontend port |

## AI Editor Behavior

The editor supports two AI actions:

- `Continue` streams short text at the current caret position.
- `Edit` rewrites the current selection inline.

Both actions send:

- a windowed document slice as Quill Delta plus plain text
- an optional selected slice as Quill Delta plus plain text
- story context
- a per-request instruction prompt

The frontend parses NDJSON events from `/ai/generate`, applies streamed delta fragments directly to Quill, locks typing while a run is active, and exposes cancel. Cancel keeps text that already landed. Failed edits restore the original text.

At the moment, the browser-side Continue and Edit trigger path is local-only. It always calls the Rust API at `http://127.0.0.1:2908`, and the API's local CORS allowlist expects the frontend to run on port `3001`.

## Verification

Run the frontend tests:

```sh
bun test
```

Run a TypeScript check:

```sh
bunx tsc --noEmit
```

Build the production bundle:

```sh
bun run build
```
