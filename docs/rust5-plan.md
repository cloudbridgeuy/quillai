---
shaping: done
---

# RUST-5 Implementation Plan — Users CRUD API

## Overview

Replace the stub `GET /` handler (`SELECT 'hello world from sqlite'`) with a full CRUD API
for the `users` table. Establish module patterns reusable for future resources.

## Pattern Applicability

| Pattern | Applies? | How |
|---------|:--------:|-----|
| Functional Core / Imperative Shell | **Yes** (limited) | Validation functions are the pure core (unit-testable). Handlers are the imperative shell (thin, orchestrate I/O). Per FC/IS "When NOT to Apply": simple CRUD with no business rules doesn't need a service layer. |
| Type-Driven Development | **Yes** | Define `User`, `CreateUser`, `UpdateUser` as proper structs first. Serde derives enforce serialization contracts. Function signatures document what goes in/out. |
| Algebraic Data Types | **Yes** | `Error` enum is a sum type with exhaustive matching. Each variant carries context relevant to that failure mode. |
| Parse Don't Validate | **Yes** | `CreateUser::validate()` and `UpdateUser::validate()` parse at the HTTP boundary. Once validated, handlers trust the data. No re-checking downstream. |
| Make Impossible States Impossible | **Partial** | `UpdateUser` uses `Option<String>` for fields — these are truly independent optionals (the pattern says this is correct). Error enum prevents impossible error combinations. |
| Composition over Inheritance | **Yes** (natural) | Rust has no inheritance. Router composed via `Router::merge()`. Error handling via trait impls (`IntoResponse`, `From`). |
| CQRS | **No** | Pattern explicitly says "When NOT to Apply: Simple CRUD applications." Read and write models are the same shape. |
| Data-Oriented Design | **No** | Pattern explicitly says "When NOT to Apply: Business logic with few entities." This is a web API, not a batch processor. |

---

## File Map

New files:
- `crates/api/src/users/mod.rs` — route wiring (`pub fn routes(pool) -> Router`)
- `crates/api/src/users/handlers.rs` — Axum handlers (imperative shell)
- `crates/api/src/users/models.rs` — domain types + validation (functional core)

Modified files:
- `crates/api/src/lib.rs` — replace stub handler, compose users routes
- `crates/api/src/error.rs` — add `NotFound` and `Validation` variants
- `crates/api/tests/handler_test.rs` — replace old `GET /` tests with users CRUD tests
- `.sqlx/` — remove old cache entry, add new query cache entries

---

## Phases

### Phase 1 — Foundation (parallel tasks)

Two independent tasks that can run concurrently.

#### Task 1A: Error Variants

**File:** `crates/api/src/error.rs`

Add two new variants to the `Error` enum:

```rust
#[derive(thiserror::Error)]
pub enum Error {
    #[error("sqlx error")]
    Sqlx(#[from] sqlx::Error),
    #[error("invalid log level")]
    InvalidLogLevel(#[from] quillai_log::Error),
    #[error("not found")]
    NotFound,
    #[error("validation error: {0}")]
    Validation(String),
}
```

Update `status_code()` — exhaustive match, no wildcards (ADT pattern):
- `NotFound` → `StatusCode::NOT_FOUND` (404)
- `Validation(_)` → `StatusCode::UNPROCESSABLE_ENTITY` (422)

Update `error_code()` — exhaustive match:
- `NotFound` → `"NOT_FOUND"`
- `Validation(_)` → `"VALIDATION_ERROR"`

Update existing unit tests and add new ones:
- `not_found_error_code_and_status` — assert 404 + "NOT_FOUND"
- `validation_error_code_and_status` — assert 422 + "VALIDATION_ERROR"
- `validation_error_message` — assert the message is included in display

**Pattern notes:**
- ADTs: exhaustive `match` on all variants, no `_ =>` wildcard
- FC/IS: `status_code()` and `error_code()` are pure functions — no I/O

---

#### Task 1B: Domain Models + Validation

**File:** `crates/api/src/users/models.rs` (new)

Define three structs:

```rust
use serde::{Deserialize, Serialize};

/// Database row representation of a user.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

/// Request body for creating a user.
#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
}

/// Request body for updating a user. All fields optional — only provided fields are updated.
#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub name: Option<String>,
    pub email: Option<String>,
}
```

Validation as pure functions (functional core — unit-testable, no I/O):

```rust
use crate::error::Error;

impl CreateUser {
    pub fn validate(&self) -> Result<(), Error> {
        if self.name.trim().is_empty() {
            return Err(Error::Validation("name must not be empty".into()));
        }
        if !self.email.contains('@') {
            return Err(Error::Validation("email must contain @".into()));
        }
        Ok(())
    }
}

impl UpdateUser {
    pub fn validate(&self) -> Result<(), Error> {
        if let Some(name) = &self.name {
            if name.trim().is_empty() {
                return Err(Error::Validation("name must not be empty".into()));
            }
        }
        if let Some(email) = &self.email {
            if !email.contains('@') {
                return Err(Error::Validation("email must contain @".into()));
            }
        }
        Ok(())
    }
}
```

Unit tests for validation (pure function tests — FC/IS pattern):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // CreateUser validation
    #[test]
    fn create_user_valid() { ... }
    #[test]
    fn create_user_empty_name_fails() { ... }
    #[test]
    fn create_user_whitespace_name_fails() { ... }
    #[test]
    fn create_user_missing_at_in_email_fails() { ... }

    // UpdateUser validation
    #[test]
    fn update_user_valid_name_only() { ... }
    #[test]
    fn update_user_valid_email_only() { ... }
    #[test]
    fn update_user_empty_name_fails() { ... }
    #[test]
    fn update_user_invalid_email_fails() { ... }
    #[test]
    fn update_user_all_none_is_valid() { ... }
}
```

**Pattern notes:**
- Type-Driven: types defined first, signatures document contracts
- Parse Don't Validate: validate at boundary, downstream trusts the data
- FC/IS: validate() is pure — deterministic, no I/O, unit-testable
- Make Impossible States Impossible: `UpdateUser` uses `Option` for truly independent optional fields (correct per pattern guidance)

---

### Phase 2 — V1 Handlers + Wiring (depends on Phase 1)

Single task. Cannot parallelize — touches `lib.rs`, `mod.rs`, `handlers.rs`, and tests that depend on all of the above.

#### Task 2: Create + List handlers, route wiring, remove stub

**File:** `crates/api/src/users/mod.rs` (new)

```rust
pub mod handlers;
pub mod models;

use axum::{routing::{get, post}, Router};
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/users", get(handlers::list_users).post(handlers::create_user))
        .with_state(pool)
}
```

**File:** `crates/api/src/users/handlers.rs` (new)

Handlers are the imperative shell — thin, no business logic, just orchestrate:

```rust
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use sqlx::sqlite::SqlitePool;

use crate::error::Error;
use super::models::{CreateUser, User};

pub async fn list_users(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<User>>, Error> {
    let users = sqlx::query_as!(User, "SELECT id, name, email FROM users")
        .fetch_all(&pool)
        .await?;
    Ok(Json(users))
}

pub async fn create_user(
    State(pool): State<SqlitePool>,
    Json(input): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), Error> {
    input.validate()?;
    let result = sqlx::query!(
        "INSERT INTO users (name, email) VALUES (?, ?)",
        input.name,
        input.email,
    )
    .execute(&pool)
    .await?;

    let user = User {
        id: result.last_insert_rowid(),
        name: input.name,
        email: input.email,
    };
    Ok((StatusCode::CREATED, Json(user)))
}
```

**File:** `crates/api/src/lib.rs` (modify)

Replace the stub handler and compose the users routes:

```rust
pub mod error;
pub mod prelude;
pub mod users;

use axum::Router;
use sqlx::sqlite::SqlitePool;

pub fn app(pool: SqlitePool) -> Router {
    Router::new().merge(users::routes(pool))
}
```

Remove:
- The old `handler()` function
- The old `use` imports for `Html`, `get`, `State`, `routing`, `query_scalar!`

**File:** `crates/api/tests/handler_test.rs` (modify)

Replace old tests. The test helper `test_pool()` needs to run migrations so the `users` table exists:

```rust
async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("failed to create in-memory SQLite pool");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("failed to run migrations");
    pool
}
```

Note: This requires adding the `migrate` feature to sqlx in dev-dependencies, or it's already in the main dependencies. Check that `sqlx::migrate!()` macro can find the migrations directory (it defaults to `./migrations` relative to the workspace root).

New integration tests:
- `list_users_empty` — GET /users returns 200 + empty array
- `create_user_returns_201` — POST /users with valid body returns 201 + user JSON with id
- `create_user_invalid_name_returns_422` — POST /users with empty name returns 422 + `{ error, code }`
- `create_user_invalid_email_returns_422` — POST /users with bad email returns 422
- `create_then_list` — POST a user, then GET /users, verify the created user appears
- `unknown_route_returns_404` — keep this test (still valid)

Remove:
- `get_root_returns_hello_world` (the stub is gone)

**Pattern notes:**
- FC/IS: handlers are the shell — read input, call validate (pure), do I/O (SQLx), format output
- Composition: `Router::merge()` composes route trees; `users::routes()` is a factory function
- Parse Don't Validate: `Json<CreateUser>` parses JSON at the boundary; `validate()` checks invariants; handler trusts the result

---

### Phase 3 — V2 + V3 (sequential, depends on Phase 2)

Two sequential tasks. They both modify `handlers.rs` and `mod.rs`, so they cannot run in parallel on the same worktree.

#### Task 3: Get by ID + Delete (V2)

**File:** `crates/api/src/users/handlers.rs` (add)

```rust
use axum::extract::Path;

pub async fn get_user(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<User>, Error> {
    let user = sqlx::query_as!(User, "SELECT id, name, email FROM users WHERE id = ?", id)
        .fetch_optional(&pool)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(Json(user))
}

pub async fn delete_user(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, Error> {
    let result = sqlx::query!("DELETE FROM users WHERE id = ?", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
```

**File:** `crates/api/src/users/mod.rs` (modify)

Add routes:
```rust
.route("/users/:id", get(handlers::get_user).delete(handlers::delete_user))
```

**File:** `crates/api/tests/handler_test.rs` (add)

Integration tests:
- `get_user_existing` — create a user, GET /users/:id → 200 + correct JSON
- `get_user_not_found` — GET /users/999 → 404 + `{ error: "not found", code: "NOT_FOUND" }`
- `delete_user_existing` — create a user, DELETE /users/:id → 204
- `delete_user_not_found` — DELETE /users/999 → 404
- `delete_then_get_returns_404` — create, delete, GET → 404

---

#### Task 4: Update (V3)

Note: `UpdateUser` model and `validate()` are already created in Task 1B (Phase 1).

**File:** `crates/api/src/users/handlers.rs` (add)

```rust
pub async fn update_user(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateUser>,
) -> Result<Json<User>, Error> {
    input.validate()?;

    // Fetch existing user first
    let existing = sqlx::query_as!(User, "SELECT id, name, email FROM users WHERE id = ?", id)
        .fetch_optional(&pool)
        .await?
        .ok_or(Error::NotFound)?;

    let name = input.name.unwrap_or(existing.name);
    let email = input.email.unwrap_or(existing.email);

    sqlx::query!(
        "UPDATE users SET name = ?, email = ? WHERE id = ?",
        name,
        email,
        id,
    )
    .execute(&pool)
    .await?;

    let user = User { id, name, email };
    Ok(Json(user))
}
```

**File:** `crates/api/src/users/mod.rs` (modify)

Add route:
```rust
.route("/users/:id", get(handlers::get_user).put(handlers::update_user).delete(handlers::delete_user))
```

**File:** `crates/api/tests/handler_test.rs` (add)

Integration tests:
- `update_user_name_only` — create user, PUT /users/:id with `{ "name": "new" }` → 200 + email unchanged
- `update_user_email_only` — create user, PUT /users/:id with `{ "email": "new@x.com" }` → 200 + name unchanged
- `update_user_both_fields` — PUT with both → 200 + both changed
- `update_user_invalid_data_returns_422` — PUT with `{ "name": "" }` → 422
- `update_user_not_found` — PUT /users/999 → 404

---

### Phase 4 — Finalize (depends on Phase 3)

Two parallel tasks.

#### Task 5: SQLx Offline Cache

- Delete old cache file: `.sqlx/query-90507242b5452858635cbed493946aa219a886878bd28f65737091faa47a57d1.json`
- Run `cargo sqlx prepare --workspace` to regenerate cache for new queries
- Verify: `SQLX_OFFLINE=true cargo check` passes

Requires a running SQLite database with migrations applied. If `DATABASE_URL` isn't set:
```bash
DATABASE_URL="sqlite:quillai.db" sqlx database create
DATABASE_URL="sqlite:quillai.db" sqlx migrate run
DATABASE_URL="sqlite:quillai.db" cargo sqlx prepare --workspace
```

#### Task 6: Verification + Doc Update

Run full verification:
```bash
cargo clippy --workspace -- -D warnings
cargo test --workspace
SQLX_OFFLINE=true cargo check
```

**File:** `docs/codebase-audit-shaping.md` (modify)

Mark RUST-5 as DONE in the findings table:
```
| ~~**RUST-5**~~ | ~~**API handler is a stub** — `GET /` runs `SELECT 'hello world'` instead of querying `users` table~~ | ~~Medium~~ | ~~R0~~ | **DONE** — replaced stub with full CRUD (list, get, create, update, delete) for `users` table |
```

---

## Execution Summary

```
Phase 1 (parallel):   Task 1A (error variants)  ║  Task 1B (models + validation)
                            ╲                      ╱
Phase 2 (sequential):        Task 2 (V1: create + list + wiring + tests)
                                    │
Phase 3 (sequential):        Task 3 (V2: get + delete)
                                    │
                             Task 4 (V3: update)
                                    │
Phase 4 (parallel):   Task 5 (SQLx cache)  ║  Task 6 (verify + docs)
```

## Dependencies

```
Task 1A ──┐
           ├──▶ Task 2 ──▶ Task 3 ──▶ Task 4 ──┬──▶ Task 5
Task 1B ──┘                                     └──▶ Task 6
```
