# QuillAI

AI-assisted fiction editor. Rust/Axum API backend + React/Bun frontend with Quill.js.

## Prerequisites

- Rust (stable)
- Bun (v1.0+)
- SQLite (no separate install needed — bundled at build time)

## Workspace Layout

```
crates/api/    — Axum HTTP server, SQLite, CLI config
crates/log/    — Structured logging library (tracing-based)
xtask/         — Build task runner (cargo xtask api|dev)
app/           — React frontend (Bun runtime, Quill.js editor)
migrations/    — SQLx migrations (SQLite)
docs/          — QuillJS implementation guide
```

## Quick Start

Install frontend dependencies:

```sh
bun install --cwd app
```

Start the API server:

```sh
cargo xtask api
```

Start the frontend dev server:

```sh
cargo xtask dev app
```

The frontend expects the Rust API at `http://127.0.0.1:2908`.
The current browser-side AI trigger path is local-only: Continue and Edit always call that default API URL, and the API's local CORS allowlist expects the frontend to run on port `3001`.

## Configuration

### API Server

| Flag | Env Variable | Default |
|------|-------------|---------|
| `--host` | — | `127.0.0.1` |
| `--port` | — | `2908` |
| `--db-url` | `QUILLAI_API_DB_URL` | `sqlite://` |
| `--db-max-connections` | `QUILLAI_API_DB_MAX_CONNECTIONS` | `1` |
| `--db-acquire-timeout` | `QUILLAI_API_DB_ACQUIRE_TIMEOUT` | `3` |
| `--log-level` | `QUILLAI_API_LOG_LEVEL` | `warn` |

AI provider env vars:

| Env Variable | Required | Description |
|-------------|----------|-------------|
| `QUILLAI_AI_API_KEY` | Yes | OpenAI-compatible API key used by the Rust API |
| `QUILLAI_AI_MODEL` | Yes | Model name passed to `chat/completions` |
| `QUILLAI_AI_BASE_URL` | No | Override base URL for OpenAI-compatible hosts; defaults to `https://api.openai.com/v1` |

### Frontend

| Env Variable | Default |
|-------------|---------|
| `PORT` | `3001` |

## AI Editor

The frontend editor now exposes two AI actions:

- `Continue` streams a short continuation at the current caret position.
- `Edit` rewrites the current selection inline.

Both actions accept:

- a shared story-context box for standing notes
- a per-request instruction prompt
- a configurable context window size, or a whole-document toggle

While a run is active, the editor becomes read-only and shows a cancel button. Cancel keeps text that already landed. Failed edits restore the original text.

## API Routes

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/users` | List all users |
| `POST` | `/users` | Create user |
| `GET` | `/users/{id}` | Get user by ID |
| `PUT` | `/users/{id}` | Partial update user |
| `DELETE` | `/users/{id}` | Delete user |
| `POST` | `/ai/generate` | Stream AI delta fragments as NDJSON |

## Running Tests

Rust tests:

```sh
cargo test
```

Frontend tests:

```sh
bun test --cwd app
```

TypeScript check:

```sh
sh -lc 'cd app && bun x tsc --noEmit'
```

Frontend production build:

```sh
bun run --cwd app build
```
