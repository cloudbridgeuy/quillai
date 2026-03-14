---
shaping: true
---

# Audit Fix Plan — Priority 4 & 5

Implements slices V1–V4 from the codebase audit shaping (FE-5, RUST-8, RUST-6, FE-10).

## Applicable Patterns

| Pattern | Applies to | How |
|---------|-----------|-----|
| **Functional Core / Imperative Shell** | V1, V3, V4 | Error classification is pure; boundary component is shell. Handler is shell; router factory is pure construction. Markdown functions are pure core; Editor component is shell. |
| **Algebraic Data Types** | V1 | Error boundary state modeled as discriminated union: `{ status: 'idle' } \| { status: 'error', error } \| { status: 'recovering', error }` |
| **Make Impossible States Impossible** | V1 | ADT state replaces `{ hasError: boolean, error: Error \| null }` — no invalid combinations |
| **Type-Driven Development** | V1, V3, V4 | Types designed first, compiler/checker guides implementation |
| **Parse Don't Validate** | V4 | Raw text + cursor position parsed into typed `MarkdownMatch` at boundary |
| **Composition over Inheritance** | V1, V3 | ErrorBoundary composes error catching + fallback rendering. Handler composes pool + query + response. |
| **CQRS** | — | Not applicable at current scale |
| **Data-Oriented Design** | — | Not applicable — no hot loops |

---

## Parallelism Analysis

All four slices are **fully independent** — they touch different files, different languages, different layers. They can all run in parallel.

| Slice | Files touched | Language | Dependencies |
|-------|--------------|----------|--------------|
| V1 | `app/src/components/ErrorBoundary.tsx`, `app/src/App.tsx` | TypeScript/React | None |
| V2 | `crates/api/Cargo.toml`, `.env`, `.sqlx/` | Rust/Config | None |
| V3 | `crates/api/src/main.rs`, `crates/api/Cargo.toml`, `crates/api/tests/` | Rust | V2 should run first (offline feature needed for tests) |
| V4 | `app/src/md_editor/markdown.ts`, `app/src/md_editor/components/Editor.tsx`, `app/src/md_editor/markdown.test.ts` | TypeScript | None |

**Dependency note:** V3 depends on V2 completing first (the `"offline"` feature in Cargo.toml is shared). V1 and V4 are fully independent of everything.

**Groups:**
- **Group A** (parallel): V1, V4
- **Group B** (sequential): V2 → V3

All three groups can run concurrently.

---

## V1: Error Boundary Catches Crashes

**Patterns:** FC/IS, ADTs, Make Impossible States Impossible, Type-Driven Development, Composition

### Tasks

#### V1-T1: Create `ErrorBoundary` component

**File:** `app/src/components/ErrorBoundary.tsx` (new)

1. Define the state ADT (Make Impossible States Impossible):
   ```ts
   type ErrorBoundaryState =
     | { status: 'idle' }
     | { status: 'error'; error: Error; errorInfo: React.ErrorInfo };
   ```
   No `{ hasError: true, error: null }` possible.

2. Define props type (Type-Driven Development):
   ```ts
   type ErrorBoundaryProps = {
     children: React.ReactNode;
     fallback?: (props: { error: Error; retry: () => void }) => React.ReactNode;
   };
   ```

3. Implement class component (Imperative Shell — React requires class for error boundaries):
   - `componentDidCatch(error, errorInfo)` — sets state to `{ status: 'error', error, errorInfo }`
   - `render()` — exhaustive switch on `state.status`:
     - `'idle'` → render `children`
     - `'error'` → render fallback UI (error message + "Try again" button)
   - `resetErrorBoundary()` — sets state back to `{ status: 'idle' }`, called by retry button

4. Default fallback (Functional Core — pure render function):
   ```tsx
   function DefaultFallback({ error, retry }: { error: Error; retry: () => void }) {
     return (
       <div className="error-boundary-fallback">
         <p>Something went wrong: {error.message}</p>
         <button onClick={retry}>Try again</button>
       </div>
     );
   }
   ```

#### V1-T2: Wrap components in `App.tsx`

**File:** `app/src/App.tsx`

1. Import `ErrorBoundary`
2. Wrap `<Editor />` in `<ErrorBoundary>`
3. Wrap `<APITester />` in `<ErrorBoundary>`
4. Each gets its own boundary — a crash in one doesn't affect the other (Composition)

#### V1-T3: Verify

- Run `bun build` to check for type errors
- Manual check: the app renders without errors

---

## V2: SQLx Compile-Time Verification

**Patterns:** None directly — this is infrastructure/config work.

### Tasks

#### V2-T1: Add `offline` feature to SQLx

**File:** `crates/api/Cargo.toml`

1. Change sqlx features from `["runtime-tokio", "sqlite"]` to `["runtime-tokio", "sqlite", "offline"]`

#### V2-T2: Create `.env` for `DATABASE_URL`

**File:** `.env` (new, project root)

1. Add `DATABASE_URL=sqlite:quillai.db`
2. This is needed by `cargo sqlx prepare` to connect and cache queries

#### V2-T3: Populate `.sqlx/` cache

1. Run `cargo sqlx database create` — creates `quillai.db`
2. Run `cargo sqlx migrate run` — applies the `users` migration
3. Run `cargo sqlx prepare --workspace` — generates `.sqlx/*.json` files
4. The `.sqlx/` directory should now contain JSON files describing query metadata

#### V2-T4: Verify

- Set `SQLX_OFFLINE=true` and run `cargo check -p quillai_api` — should pass without a live DB
- Run `cargo build -p quillai_api` — should succeed

**Note:** `cargo sqlx` CLI must be installed. If not available, install with `cargo install sqlx-cli --features sqlite`.

---

## V3: API Handler Integration Tests

**Patterns:** FC/IS, Type-Driven Development, Composition

### Tasks

#### V3-T1: Extract router factory (Functional Core)

**File:** `crates/api/src/main.rs`

1. Extract router creation into a pure factory function:
   ```rust
   pub fn app(pool: SqlitePool) -> Router {
       Router::new().route("/", get(handler)).with_state(pool)
   }
   ```
   This is the "functional core" — given a pool, it returns a deterministic router. No I/O.

2. Update `main()` to call `app(pool)` instead of inline construction.

3. Keep `handler` private — it's an implementation detail of the router. Tests exercise the router, not the handler directly.

#### V3-T2: Add dev dependencies

**File:** `crates/api/Cargo.toml`

1. Add `[dev-dependencies]` section:
   ```toml
   [dev-dependencies]
   tower = { version = "0.5", features = ["util"] }
   http = "1"
   tokio = { workspace = true, features = ["macros"] }
   ```
   - `tower` for `ServiceExt::oneshot`
   - `http` for `Request::builder`
   - `tokio` with `macros` for `#[tokio::test]`

#### V3-T3: Write integration tests

**File:** `crates/api/tests/handler_test.rs` (new)

1. Helper to create test pool (Imperative Shell — I/O setup):
   ```rust
   async fn test_pool() -> SqlitePool {
       let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
       // No migrations needed — handler queries static string, not users table
       pool
   }
   ```

2. Test: `GET /` returns 200 with HTML containing "hello world":
   ```rust
   #[tokio::test]
   async fn get_root_returns_hello_world() {
       let pool = test_pool().await;
       let app = quillai_api::app(pool);
       let req = Request::builder().uri("/").body(Body::empty()).unwrap();
       let res = app.oneshot(req).await.unwrap();
       assert_eq!(res.status(), StatusCode::OK);
       // Read body and assert contains "hello world"
   }
   ```

3. Test: unknown route returns 404:
   ```rust
   #[tokio::test]
   async fn unknown_route_returns_404() {
       let pool = test_pool().await;
       let app = quillai_api::app(pool);
       let req = Request::builder().uri("/nonexistent").body(Body::empty()).unwrap();
       let res = app.oneshot(req).await.unwrap();
       assert_eq!(res.status(), StatusCode::NOT_FOUND);
   }
   ```

#### V3-T4: Make `app()` accessible from tests

**File:** `crates/api/src/main.rs`

1. The `app` function needs to be `pub` and the crate needs a `lib.rs` or the binary needs to re-export.
2. Create `crates/api/src/lib.rs` (new) that re-exports `app`:
   ```rust
   mod error;
   mod prelude;

   pub use app::app;
   ```
   Actually simpler: move `app()` and `handler()` into `lib.rs`, keep `main.rs` as thin shell that calls `lib.rs`.

   **`crates/api/src/lib.rs`** (new — Functional Core):
   - `pub mod error;`
   - `pub mod prelude;`
   - `pub fn app(pool: SqlitePool) -> Router` — router factory
   - `async fn handler(...)` — private handler

   **`crates/api/src/main.rs`** (Imperative Shell):
   - CLI parsing
   - Logger init
   - DB pool creation
   - Calls `quillai_api::app(pool)`
   - Listener binding + serve

#### V3-T5: Verify

- Run `cargo test -p quillai_api` — both tests pass
- Run `cargo clippy -p quillai_api` — no warnings

---

## V4: Editor Markdown Unit Tests

**Patterns:** FC/IS, Parse Don't Validate, Type-Driven Development

### Tasks

#### V4-T1: Define types for markdown analysis (Type-Driven Development)

**File:** `app/src/md_editor/markdown.ts` (new)

1. Define the return type of `analyzeMarkdownContext` (Parse Don't Validate — raw text parsed into typed structure):
   ```ts
   type MarkdownFormat = { bold?: true; italic?: true };

   type MarkdownPattern = {
     regex: RegExp;
     format: MarkdownFormat;
     type: 'bold-italic' | 'bold' | 'italic';
     markersLength: number;
   };

   type MarkdownMatch = {
     match: RegExpMatchArray;
     pattern: MarkdownPattern;
     start: number;
     end: number;
     content: string;
   };
   ```

2. These types are the contract — function signatures follow from them.

#### V4-T2: Extract pure functions (Functional Core)

**File:** `app/src/md_editor/markdown.ts`

1. Extract `analyzeMarkdownContext(text: string, cursorPos: number): MarkdownMatch | null`
   - Pure function: string in, typed match out
   - No Quill dependency, no DOM access
   - This is "parse don't validate" — raw text is parsed into a `MarkdownMatch` or rejected as `null`

2. Extract `isPatternComplete(text: string, cursorPos: number): boolean`
   - Pure function: calls `analyzeMarkdownContext` internally
   - No side effects

3. Export both functions.

#### V4-T3: Update `Editor.tsx` to import extracted functions (Imperative Shell)

**File:** `app/src/md_editor/components/Editor.tsx`

1. Import `analyzeMarkdownContext` and `isPatternComplete` from `../markdown`
2. Remove the nested function definitions from `processMarkdownSyntax`
3. `processMarkdownSyntax` remains in Editor.tsx — it's the imperative shell (reads Quill state, calls pure functions, applies Quill mutations)
4. Update calls: `analyzeMarkdownContext` no longer receives `lineStart` context implicitly — pass the line text and relative cursor position explicitly

#### V4-T4: Write unit tests (Pure function tests)

**File:** `app/src/md_editor/markdown.test.ts` (new)

Follow existing test conventions from `APITester.test.ts` (import from `bun:test`, named imports).

Tests for `analyzeMarkdownContext`:
1. `*foo*` at cursor → returns italic match with content "foo"
2. `**foo**` at cursor → returns bold match with content "foo"
3. `***foo***` at cursor → returns bold-italic match with content "foo"
4. `*foo` (incomplete) → returns null
5. `**foo*` (mismatched) → returns italic match for inner `*foo*` portion, or null depending on cursor position
6. Empty string → returns null
7. No asterisks → returns null
8. Content with spaces: `*foo bar*` → returns italic match with content "foo bar"

Tests for `isPatternComplete`:
1. `*foo*` → true (complete italic)
2. `**foo**` → true (complete bold)
3. `***foo***` → true (complete bold-italic)
4. `*foo*` followed by `*` → false (might become `**foo**`)
5. `**foo**` preceded by `*` → false (might be `***foo***`)
6. `*foo` → false (incomplete)

#### V4-T5: Verify

- Run `bun test app/src/md_editor/markdown.test.ts` — all tests pass
- Run `bun build app/src/index.html` — no type errors, app still works

---

## Execution Order

```
                    ┌─── V1 (Error Boundary) ───┐
                    │                            │
Start ──────────────┼─── V4 (Markdown Tests) ───┤──── Done
                    │                            │
                    └─── V2 (SQLx Cache) ────────┤
                              │                  │
                              └── V3 (API Tests) ┘
```

**Group A** — V1 and V4 run in parallel (independent)
**Group B** — V2 then V3 sequentially (V3 needs V2's Cargo.toml change)

All groups run concurrently.

---

## Verification Checklist

After all slices complete:

- [ ] `cargo check --workspace` passes
- [ ] `cargo clippy --workspace` has no warnings
- [ ] `cargo test --workspace` passes (V3 handler tests + existing log tests)
- [ ] `bun test` passes in `app/` (V4 markdown tests + existing APITester tests)
- [ ] App runs: `cargo xtask dev app` starts without errors
- [ ] Error boundary works: throwing in Editor/APITester shows fallback, retry recovers
- [ ] `SQLX_OFFLINE=true cargo check -p quillai_api` passes

## README Updates

No README updates needed — these are internal quality improvements (error handling, testing, build config) that don't change the project's public interface or setup instructions.

## Audit Doc Update

After all slices pass verification, update `docs/codebase-audit-shaping.md`:
- Mark FE-5 as **DONE** (Error Boundary added)
- Mark RUST-8 as **DONE** (SQLx offline cache populated)
- Mark RUST-6 as **DONE** (API handler integration tests added)
- Mark FE-10 as partially done (Editor markdown tests added; component tests deferred)
- Update Fit Check table: R3 → ✅, R4 → ⚠️ (partial — xtask still untested)
