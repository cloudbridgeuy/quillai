# Users CRUD API

## Architecture

The users module follows **Functional Core / Imperative Shell**:

- **Functional Core** (`models.rs`): Pure structs (`User`, `CreateUser`, `UpdateUser`) and validation functions. No I/O, fully unit-testable.
- **Imperative Shell** (`handlers.rs`): Thin Axum handlers that orchestrate I/O (SQLx queries). No business logic — just extract input, call validate, do I/O, format output.
- **Composition** (`mod.rs`): `routes()` factory function composes a `Router` via method chaining, merged into the app via `Router::merge()`.

## Module Structure

```
crates/api/src/users/
├── mod.rs        — Route wiring: routes(pool) -> Router
├── handlers.rs   — Axum handlers (imperative shell)
└── models.rs     — Domain types + validation (functional core)
```

## Routes

| Method | Path | Handler | Status | Description |
|--------|------|---------|--------|-------------|
| GET | `/users` | `list_users` | 200 | Returns all users as JSON array |
| POST | `/users` | `create_user` | 201 | Creates user, returns created user with id |
| GET | `/users/{id}` | `get_user` | 200/404 | Returns single user or NotFound |
| PUT | `/users/{id}` | `update_user` | 200/404/422 | Partial update via `unwrap_or(existing)` |
| DELETE | `/users/{id}` | `delete_user` | 204/404 | Deletes user, checks rows_affected |

Note: Axum 0.8 uses `{id}` syntax for path parameters (not `:id`).

## Validation Pattern

Validation is implemented as pure functions on the request types (Parse Don't Validate):

```rust
impl CreateUser {
    pub fn validate(&self) -> Result<(), Error> { ... }
}
```

Shared validation helpers (`validate_name`, `validate_email`) are private free functions in `models.rs`, called by both `CreateUser::validate()` and `UpdateUser::validate()`.

Rules:
- Name: must not be empty or whitespace-only (`trim().is_empty()`)
- Email: must contain `@`

## Error Mapping

| Error | Status | Code |
|-------|--------|------|
| `Error::NotFound` | 404 | `NOT_FOUND` |
| `Error::Validation(msg)` | 422 | `VALIDATION_ERROR` |
| `Error::Sqlx(_)` | 500 | `SQLX` |

All errors return JSON: `{ "error": "<message>", "code": "<CODE>" }`.

## Testing

- **Unit tests** (`models.rs`): 9 tests covering all validation paths for CreateUser and UpdateUser
- **Integration tests** (`tests/handler_test.rs`): 16 tests covering all CRUD operations, validation errors, not-found cases

Test helpers in `handler_test.rs`:
- `test_pool()` (from `tests/common/mod.rs`) — single-connection in-memory SQLite with foreign keys, migrated via `quillai_api::database::migrate`
- `json_request(method, uri, body)` — builds JSON POST/PUT requests
- `get_request(uri)` / `delete_request(uri)` — builds bodyless requests
- `json_body(res)` — extracts and parses response as JSON
- `create_user(pool, name, email)` — creates a user and returns its id

## Adding a New Resource

Follow the same pattern:
1. Create `crates/api/src/<resource>/models.rs` with types and validation
2. Create `crates/api/src/<resource>/handlers.rs` with thin handlers
3. Create `crates/api/src/<resource>/mod.rs` with `routes()` factory
4. Add `pub mod <resource>;` to `lib.rs`
5. Merge routes: `Router::new().merge(users::routes(pool.clone())).merge(<resource>::routes(pool))`
6. Regenerate SQLx cache: `cargo sqlx prepare --workspace`
