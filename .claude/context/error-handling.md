# Error Handling — API ↔ Frontend Contract

## JSON Error Shape

Errors mapped through `crates/api/src/error.rs` return a standardized JSON body:

```json
{ "error": "<human-readable message>", "code": "<SCREAMING_SNAKE error code>" }
```

HTTP status code is set via the response status, not embedded in the body.

Axum extractor rejections not mapped by a handler can return plain text.

## Backend (`crates/api/src/error.rs`)

The `Error` enum uses `thiserror` for `Display`/`From` derives. HTTP mapping is manual (no `axum_thiserror`):

- **Pure methods** (functional core):
  - `status_code(&self) -> StatusCode` — exhaustive match, one arm per variant
  - `error_code(&self) -> &'static str` — exhaustive match, SCREAMING_SNAKE strings
- **`IntoResponse` impl** (imperative shell): composes `status_code()` + `Json({ error, code })`

### Adding a new error variant

1. Add variant to `Error` enum with `#[error("...")]` and optional `#[from]`
2. Add arm to `status_code()` — pick appropriate `StatusCode`
3. Add arm to `error_code()` — pick a SCREAMING_SNAKE string
4. Add a unit test for the new variant

The compiler enforces exhaustive matching, so missing arms won't compile.

## Frontend (`app/src/manuscripts/contract.ts`)

`parseApiError()` requires string `error` and `code` fields. The manuscript client preserves the message, code, and HTTP status in `ManuscriptApiError`. Malformed or non-JSON error responses produce `INVALID_RESPONSE`. The lore and settings clients also use the shared parser.

## Tests

| Layer | Command | What's tested |
|-------|---------|---------------|
| Backend | `cargo test -p quillai_api` | `status_code()` and `error_code()` per variant |
| Frontend | `cd app && bun test src/manuscripts/client.test.ts` | HTTP error message, code, status, and malformed responses |
