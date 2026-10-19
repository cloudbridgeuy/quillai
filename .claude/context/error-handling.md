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

## Frontend (`app/src/api_tester/components/APITester.tsx`)

`formatResponseBody()` is a pure function that parses the response:

- **Success**: pretty-printed JSON
- **API error** (has `error` + `code` fields): `Error [CODE]: message`
- **Fallback** (non-JSON or unexpected shape): `{status} {statusText}`

The fetch call catches `res.json()` failures so non-JSON responses degrade gracefully.

## Tests

| Layer | Command | What's tested |
|-------|---------|---------------|
| Backend | `cargo test -p quillai_api` | `status_code()` and `error_code()` per variant |
| Frontend | `bun test app/src/api_tester/` | `formatResponseBody()` for all branches |
