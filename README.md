# QuillAI

Markdown-aware rich text editor. Rust/Axum API backend + React/Bun frontend with Quill.js.

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

### Frontend

| Env Variable | Default |
|-------------|---------|
| `PORT` | `3001` |

## API Routes

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/users` | List all users |
| `POST` | `/users` | Create user |
| `GET` | `/users/{id}` | Get user by ID |
| `PUT` | `/users/{id}` | Partial update user |
| `DELETE` | `/users/{id}` | Delete user |

## Running Tests

Rust tests:

```sh
cargo test
```

Frontend tests:

```sh
bun test --cwd app
```
