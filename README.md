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
xtask/         — Development process runner (`cargo xtask dev`)
app/           — React frontend (Bun runtime, Quill.js editor)
migrations/    — SQLx migrations (SQLite)
docs/          — QuillJS implementation guide
```

## Quick Start

Start the API server and frontend dev server together:

```sh
cargo xtask dev
```

Use custom ports when needed:

```sh
cargo xtask dev --api-port 2908 --app-port 3001
```

Choose a stronger AI model for local testing:

```sh
QUILLAI_AI_API_KEY=your-key cargo xtask dev --ai-model gpt-4.1
```

Use another OpenAI-compatible provider when needed:

```sh
QUILLAI_AI_API_KEY=your-key cargo xtask dev \
  --ai-model provider-model-name \
  --ai-base-url https://provider.example/v1
```

`xtask dev` installs frontend dependencies when `app/node_modules` is missing, runs the Rust API and Bun frontend concurrently, prefixes each log line with the current time and service name, and stops both processes on `Ctrl-C`. `QUILLAI_AI_API_KEY` stays an environment variable so secrets are not added to `xtask` flags.

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

AI provider env vars and `xtask dev` overrides:

| Setting | Env Variable | `xtask dev` Flag | Required | Description |
|---------|--------------|------------------|----------|-------------|
| API key | `QUILLAI_AI_API_KEY` | — | Yes | OpenAI-compatible API key used by the Rust API |
| Model | `QUILLAI_AI_MODEL` | `--ai-model` | Yes | Model name passed to `chat/completions` |
| Base URL | `QUILLAI_AI_BASE_URL` | `--ai-base-url` | No | Override base URL for OpenAI-compatible hosts; defaults to `https://api.openai.com/v1` |

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
