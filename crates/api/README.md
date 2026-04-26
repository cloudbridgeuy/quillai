# quillai_api

Axum HTTP server for QuillAI. Serves the users CRUD API and the AI streaming endpoint over SQLite plus an OpenAI-compatible provider.

## Running

With hot reload via systemfd:

```sh
cargo xtask api
```

Direct:

```sh
cargo run -p quillai_api
```

## CLI Flags

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--host` | — | `127.0.0.1` | Bind address |
| `--port` | — | `2908` | Listen port |
| `--db-url` | `QUILLAI_API_DB_URL` | `sqlite://` | SQLite connection URL |
| `--db-max-connections` | `QUILLAI_API_DB_MAX_CONNECTIONS` | `1` | Pool size |
| `--db-acquire-timeout` | `QUILLAI_API_DB_ACQUIRE_TIMEOUT` | `3` | Acquire timeout (seconds) |
| `--log-level` | `QUILLAI_API_LOG_LEVEL` | `warn` | Log level |

## AI Provider Configuration

| Env var | Required | Description |
|---------|----------|-------------|
| `QUILLAI_AI_API_KEY` | Yes | OpenAI-compatible API key |
| `QUILLAI_AI_MODEL` | Yes | Model name for `chat/completions` |
| `QUILLAI_AI_BASE_URL` | No | Base URL override, default `https://api.openai.com/v1` |

The current implementation uses a direct OpenAI-compatible HTTP client. `rig` was spiked for this slice, but the final implementation kept direct `reqwest` streaming because it provided more explicit control over base URLs, SSE parsing, and cancellation behavior.

## Routes

| Method | Path | Description |
|--------|------|-------------|
| GET | `/users` | List all users |
| POST | `/users` | Create user (`{ "name": "...", "email": "..." }`) |
| GET | `/users/{id}` | Get user by ID |
| PUT | `/users/{id}` | Partial update user (name and/or email) |
| DELETE | `/users/{id}` | Delete user (204 on success) |
| POST | `/ai/generate` | Stream AI delta fragments as NDJSON |

### `POST /ai/generate`

Request body:

```json
{
  "action": "generate",
  "document": {
    "delta": { "ops": [{ "insert": "Opening line." }] },
    "text": "Opening line."
  },
  "selection": null,
  "storyContext": "fantasy court intrigue",
  "instruction": "continue the scene"
}
```

`edit` requests send a non-null `selection` object:

```json
{
  "delta": { "ops": [{ "insert": "line" }] },
  "text": "line",
  "range": { "index": 8, "length": 4 }
}
```

Success responses stream `application/x-ndjson` events:

```json
{"type":"delta","delta":{"ops":[{"insert":"New text"}]}}
{"type":"done"}
```

Pre-stream failures still use the standard JSON error shape:

```json
{ "error": "...", "code": "..." }
```

## Tests

```sh
cargo test -p quillai_api
```

Compile check:

```sh
SQLX_OFFLINE=true cargo check -p quillai_api
```

Integration tests in `tests/` using axum test helpers with in-memory SQLite.
