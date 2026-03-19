# quillai_api

Axum HTTP server for QuillAI. Serves a CRUD API for the `users` table over SQLite.

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

## Routes

| Method | Path | Description |
|--------|------|-------------|
| GET | `/users` | List all users |
| POST | `/users` | Create user (`{ "name": "...", "email": "..." }`) |
| GET | `/users/{id}` | Get user by ID |
| PUT | `/users/{id}` | Partial update user (name and/or email) |
| DELETE | `/users/{id}` | Delete user (204 on success) |

## Tests

```sh
cargo test -p quillai_api
```

Integration tests in `tests/` using axum test helpers with in-memory SQLite.
