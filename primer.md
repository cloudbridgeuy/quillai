# QuillAI Primer

This file is the fast-start map for agents working in this repository. Read it first, then use `CLAUDE.md` and the relevant `.claude/context/*.md` files for subsystem details.

## What This Repo Is

QuillAI is not one monolith. It is a small workspace with two main runtime surfaces:

- `crates/api/`: the Rust/Axum JSON API, backed by SQLite through SQLx.
- `app/`: the Bun-served React frontend, including the Quill-based editor.

Support code lives in:

- `crates/log/`: shared logging setup.
- `xtask/`: developer workflow commands.
- `migrations/`: database schema changes.
- `.sqlx/`: SQLx query cache for compile-time query verification.

The current product code is concentrated in `crates/api/src/ai/`, `crates/api/src/users/`, `app/src/md_editor/`, and `app/src/components/ErrorBoundary.tsx`. Some top-level files still look like starter scaffolding. Follow active feature folders and context docs more than generic starter text or dependency lists.

## Start Here

When you need context, read in this order:

1. `primer.md`
2. `CLAUDE.md`
3. The closest subsystem doc under `.claude/context/`
4. The concrete feature code itself

Use the closest existing example before inventing a new structure:

- New Rust CRUD resource: `crates/api/src/users/`
- Rust AI request and stream boundary: `crates/api/src/ai/`
- Pure frontend logic: `app/src/md_editor/ai_contract.ts`, `app/src/md_editor/ai_stream.ts`, `app/src/md_editor/window.ts`
- Frontend fetch and stream shell: `app/src/md_editor/components/Editor.tsx`
- UI crash isolation: `app/src/components/ErrorBoundary.tsx`

## The Main Mental Model

The codebase prefers Functional Core / Imperative Shell.

- Put pure logic in small, testable functions and data types.
- Keep handlers, components, and server entry points thin.
- Parse raw input at boundaries, then pass typed values inward.
- Use explicit enums or discriminated unions when they make state transitions clearer.

In practice, that means:

- `crates/api/src/main.rs` starts the server, wires logging, creates the DB pool, and does not hold business rules.
- `crates/api/src/lib.rs` composes routers and stays tiny.
- `crates/api/src/ai/models.rs` holds AI request types and boundary validation.
- `crates/api/src/ai/provider.rs` owns OpenAI-compatible streaming HTTP I/O.
- `crates/api/src/users/models.rs` holds request types and validation.
- `crates/api/src/users/handlers.rs` performs SQL I/O and HTTP mapping.
- `app/src/md_editor/markdown.ts` holds pure markdown analysis.
- `app/src/md_editor/components/Editor.tsx` owns Quill, fetch, cancellation, and stream application.

If a function touches Quill, fetch, SQL, HTTP request extraction, environment variables, or DOM nodes, it is probably shell code. If it transforms strings, Deltas, request bodies, error shapes, or domain data, it probably belongs in the core.

## How To Navigate By Intent

If the task is about...

- API routes, JSON contracts, validation, SQL queries: start in `crates/api/src/`
- Database shape: start in `migrations/`, then check `.sqlx/`
- Frontend UI composition: start in `app/src/App.tsx` and the feature folder
- Editor behavior or markdown transforms: start in `app/src/md_editor/`
- Bun static serving or frontend dev routes: start in `app/src/index.tsx`
- Shared logging behavior: start in `crates/log/src/lib.rs`
- Dev commands or local workflow: start in `xtask/src/main.rs`

Important distinction:

- `crates/api/src/main.rs` is the Rust API process.
- `app/src/index.tsx` is the Bun frontend server.

Current AI editor caveat:

- The browser-side Continue/Edit trigger path currently assumes the Rust API is reachable at `http://127.0.0.1:2908`.

Do not add Rust API functionality to the Bun hello routes, and do not treat the Bun hello routes as the model for backend feature work.

## Where New Backend Functionality Goes

For a new API resource, follow the `users` pattern unless there is a strong reason not to.

1. Add or update a migration in `migrations/` if the schema changes.
2. Create `crates/api/src/<resource>/models.rs` for domain types and validation.
3. Create `crates/api/src/<resource>/handlers.rs` for Axum handlers and SQLx calls.
4. Create `crates/api/src/<resource>/mod.rs` for route wiring.
5. Export and merge the module in `crates/api/src/lib.rs`.
6. Add integration coverage under `crates/api/tests/`.
7. Refresh the SQLx cache with `cargo sqlx prepare --workspace` when queries change.

Placement rules:

- Validation belongs next to request types, not inline in handlers.
- Response and error mapping must preserve the existing JSON error contract: `{ "error": string, "code": string }`.
- If you add a new `Error` variant in `crates/api/src/error.rs`, update `status_code()`, `error_code()`, and tests together.
- Keep shared code local to the API crate unless it is genuinely cross-cutting.

## Where New Frontend Functionality Goes

Use feature folders first, not generic shared helpers.

1. If the behavior is editor-specific, start in `app/src/md_editor/`.
2. If the behavior is API-tester-specific, keep it in `app/src/api_tester/`.
3. If the behavior is app-wide composition, put it in `app/src/App.tsx` or a focused top-level component.
4. If the behavior is Bun server routing, static files, or SPA serving, put it in `app/src/index.tsx`.

Placement rules:

- Pure text or data transformation belongs in a standalone module like `markdown.ts`, with unit tests.
- React components should own rendering, refs, events, fetch calls, and other browser effects, but should delegate non-trivial logic to pure helpers.
- Keep feature-specific logic near the feature before creating a top-level `utils` module.
- Preserve the existing Bun toolchain. Do not introduce Vite, Express, or Node-first patterns.

For editor work, split the problem this way:

- Markdown or Delta analysis: pure module
- Quill event handling and selection updates: component shell
- Visual controls and state display: component UI

## Design Rules For New Work

- Prefer the smallest correct change.
- Extend an existing feature folder before creating a new abstraction layer.
- Reuse the repo's vocabulary: Functional Core / Imperative Shell, Parse Don't Validate, Type-Driven Development, and ADTs.
- New pure functions need unit tests.
- Wiring code gets integration tests, not elaborate unit-test mocking.
- Keep entry points boring. Startup files should compose, not decide.

When deciding where code belongs, ask:

1. Is this a boundary concern or a domain concern?
2. Does it touch I/O or only transform data?
3. Is it specific to one feature, or reused in multiple places already?

Answer those three questions before creating new files.

## Existing Context Docs Worth Reading

- `.claude/context/users-crud.md`: the backend resource template
- `.claude/context/ai-streaming.md`: the AI request contract, prompt rules, provider env vars, and NDJSON stream behavior
- `.claude/context/ai-editor.md`: the Quill AI editor shell, helper modules, and browser URL constraint
- `.claude/context/markdown-parsing.md`: the editor's best FC/IS example
- `.claude/context/error-handling.md`: API/frontend error contract
- `.claude/context/error-boundary.md`: React error isolation pattern
- `.claude/context/sqlx-offline.md`: SQLx offline workflow

Use these docs when you are changing the corresponding subsystem. They are more specific than `CLAUDE.md`.

## Verification Checklist

Run the smallest relevant checks for the area you changed:

- Backend: `cargo test -p quillai_api`
- Rust workspace quality: `cargo clippy`
- SQLx query verification: `SQLX_OFFLINE=true cargo check`
- Frontend: `bun test --cwd app`

If you changed SQL or migrations, update `.sqlx/`. If you added pure logic and no tests, the change is incomplete.

## Common Traps

- Treating `app/src/index.tsx` as the backend. It is not.
- Putting validation directly in Axum handlers or React event handlers.
- Creating generic `utils` too early.
- Adding new tooling that bypasses Bun or `cargo xtask` without a clear need.
- Copying starter scaffolding patterns instead of the active feature patterns.

When in doubt, find the closest existing feature, copy its shape, and keep the new code just as local and just as explicit.
