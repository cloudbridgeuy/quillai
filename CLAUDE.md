# QuillAI

AI-assisted fiction editor. Rust/Axum API backend + React/Bun frontend with Quill.js.

For agent onboarding, start with `primer.md`, then use this file as the source index.

## Workspace Layout

Cargo workspace with three members:

```
crates/api/    — Axum HTTP server, SQLite, CLI config
crates/log/    — Structured logging library (tracing-based)
xtask/         — Development process runner (`cargo xtask dev`)
app/           — React frontend (Bun runtime, Quill.js editor)
migrations/    — SQLx migrations (SQLite)
docs/          — QuillJS implementation guide
```

## Entry Points

| What | Command | File |
|------|---------|------|
| Full dev | `cargo xtask dev` | `xtask/src/main.rs` |
| API server only | `cargo run -p quillai_api --bin api` | `crates/api/src/main.rs` |
| Frontend dev only | `PORT=3001 bun --hot app/src/index.tsx` | `app/src/index.tsx` |

API defaults to `127.0.0.1:2908`. Frontend defaults to port `3001` (configurable via `PORT` env var).
Current AI browser calls assume the API is on `2908` and the frontend is on `3001`.

## Source Index

### Rust — API (`crates/api/src/`)

| File | Purpose |
|------|---------|
| `main.rs` | Server init, SQLite pool setup, calls `app()` from lib.rs |
| `lib.rs` | Router composition — merges resource routes via `Router::merge()` |
| `cli.rs` | CLI args via clap (host, port, db-url, log-level) |
| `error.rs` | Error enum (`Sqlx`, `InvalidLogLevel`, `NotFound`, `Validation`), manual `IntoResponse` → JSON `{ error, code }`. See [error handling context](.claude/context/error-handling.md) |
| `prelude.rs` | Re-exports Error and Result |
| `ai/` | Rich AI request parsing, prompt building, OpenAI-compatible SSE streaming, and NDJSON event output. See [AI streaming context](.claude/context/ai-streaming.md) |
| `users/` | Users CRUD module — models, handlers, route wiring. See [users CRUD context](.claude/context/users-crud.md) |

### Rust — Logging (`crates/log/src/`)

| File | Purpose |
|------|---------|
| `lib.rs` | LogLevel enum, LogConfig struct, tracing subscriber init. Formats: Pretty, Compact, Json |

### Rust — Build (`xtask/src/`)

| File | Purpose |
|------|---------|
| `main.rs` | `cargo xtask dev` runner: starts API and frontend concurrently, prefixes logs, and stops both on Ctrl-C |

### Frontend (`app/src/`)

| File | Purpose |
|------|---------|
| `index.tsx` | Bun.serve entry — routes, static assets, HMR. Port via `PORT` env var |
| `index.html` | HTML shell, loads frontend.tsx |
| `App.tsx` | Root React component — renders the editor inside an ErrorBoundary. See [error boundary context](.claude/context/error-boundary.md) |
| `components/ErrorBoundary.tsx` | React error boundary with ADT state, crash isolation per component. See [error boundary context](.claude/context/error-boundary.md) |
| `frontend.tsx` | React root setup, CSS import, StrictMode |
| `index.css` | Tailwind + editor shell layout, dark theme, and responsive styling |
| `md_editor/types.ts` | TypeScript types: Delta, Range, EmitterSource, event callbacks |
| `md_editor/ai_contract.ts` | Pure AI request contract and browser-safe API URL helper. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/ai_stream.ts` | Pure incremental NDJSON parser for AI stream events. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/ai_state.ts` | AI run-state ADT and transition helpers. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/window.ts` | Pure context-window calculation for transmitted document slices. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/delta_bridge.ts` | Quill delta normalization at the request and stream boundaries. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/editor_ai.ts` | Pure selection-memory, rollback, and status helpers for the editor shell. See [AI editor context](.claude/context/ai-editor.md) |
| `md_editor/editor_persistence.ts` | Pure parse/serialize of editor state (document, AI settings, thread) stored in `localStorage` under `quillai.editor.state.v1`. Falls back to defaults on invalid data. |
| `md_editor/markdown.ts` | Pure markdown analysis functions — `analyzeMarkdownContext`, `isPatternComplete`. See [markdown parsing context](.claude/context/markdown-parsing.md) |
| `md_editor/components/Editor.tsx` | Quill editor shell for Continue/Edit streaming, cancellation, rollback, and markdown formatting. See [AI editor context](.claude/context/ai-editor.md) and [markdown parsing context](.claude/context/markdown-parsing.md) |
| `api_tester/components/APITester.tsx` | HTTP method selector + URL input + loading state + structured error parsing. See [error handling context](.claude/context/error-handling.md) |
| `api_tester/components/__tests__/APITester.test.ts` | Unit tests for `buttonLabel()` pure function |

### Database

Single migration: `migrations/20250407204144_init.up.sql` — creates `users` table (id, name, email).

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Axum 0.8, Tokio 1.44, SQLx (SQLite), Sea-ORM |
| Logging | tracing + tracing-subscriber |
| CLI | clap 4.5 |
| Frontend | React 19, TypeScript, Quill.js 2.0.3 |
| Styling | Tailwind CSS 4 |
| Runtime | Bun (server + bundler + test runner) |
| Build | Cargo workspace + xtask |

## API Routes

**Rust backend** (`crates/api/src/users/`):
- `GET /users` — List all users (JSON array)
- `POST /users` — Create user (201 + JSON)
- `GET /users/{id}` — Get user by ID (JSON or 404)
- `PUT /users/{id}` — Partial update user (JSON, 404, or 422)
- `DELETE /users/{id}` — Delete user (204 or 404)
- `POST /ai/generate` — Stream AI delta fragments as NDJSON. See [AI streaming context](.claude/context/ai-streaming.md)

**Bun frontend** (`app/src/index.tsx`):
- `GET /assets/*` — static file serving
- `GET /api/hello` — JSON hello
- `PUT /api/hello` — JSON hello (PUT)
- `GET /api/hello/:name` — parameterized JSON hello
- `GET /*` — SPA fallback to index.html

## Coding Patterns

Reference patterns at `~/.claude/patterns/`:

| Pattern | File | Core Idea |
|---------|------|-----------|
| Functional Core - Imperative Shell | `functional-core-imperative-shell.md` | Pure logic separated from I/O at every level |
| Type-Driven Development | `type-driven-development.md` | Design types first, let compiler guide implementation |
| ADTs | `algebraic-data-types.md` | Sum types (enum) + product types (struct) to model domains |
| Make Impossible States Impossible | `make-impossible-states-impossible.md` | Only valid states representable in the type system |
| Parse Don't Validate | `parse-dont-validate.md` | Convert raw input to typed values at boundaries |
| CQRS | `cqrs.md` | Separate write models from read models |
| Composition over Inheritance | `composition-over-inheritance.md` | Trait-based components, factory functions |
| Data-Oriented Design | `data-oriented-design.md` | SoA layout, batch processing for cache efficiency |
| Relationships | `RELATIONSHIPS.md` | Visual graph of how patterns interconnect |

## Conventions

Default to using Bun instead of Node.js.

- Use `bun <file>` instead of `node <file>` or `ts-node <file>`
- Use `bun test` instead of `jest` or `vitest`
- Use `bun build <file.html|file.ts|file.css>` instead of `webpack` or `esbuild`
- Use `bun install` instead of `npm install` or `yarn install` or `pnpm install`
- Use `bun run <script>` instead of `npm run <script>` or `yarn run <script>` or `pnpm run <script>`
- Bun automatically loads .env, so don't use dotenv.

### Bun APIs

- `Bun.serve()` supports WebSockets, HTTPS, and routes. Don't use `express`.
- `bun:sqlite` for SQLite. Don't use `better-sqlite3`.
- `Bun.redis` for Redis. Don't use `ioredis`.
- `Bun.sql` for Postgres. Don't use `pg` or `postgres.js`.
- `WebSocket` is built-in. Don't use `ws`.
- Prefer `Bun.file` over `node:fs`'s readFile/writeFile
- Bun.$\`ls\` instead of execa.

### Rust Conventions

- Workspace crates under `crates/`. New crates go there.
- `cargo xtask` for build automation — extend `xtask/src/main.rs` for new tasks.
- SQLx with compile-time checked queries (`.sqlx/` cache). See [SQLx offline context](.claude/context/sqlx-offline.md).
- Errors via `thiserror` with manual `IntoResponse` for JSON responses. See [error handling context](.claude/context/error-handling.md).

### Frontend Conventions

- HTML imports with `Bun.serve()`. Don't use `vite`.
- HTML files import .tsx/.jsx/.js directly — Bun transpiles and bundles.
- `<link>` tags point to stylesheets — Bun's CSS bundler handles them.
- Tailwind via `bun-plugin-tailwind`.
- For Bun API docs: `node_modules/bun-types/docs/**.md`.

## Context Docs

- `.claude/context/ai-streaming.md` — AI request validation, prompt rules, provider env vars, and NDJSON stream behavior
- `.claude/context/ai-editor.md` — Quill AI editor shell, helper modules, and the current browser URL constraint
- `.claude/context/markdown-parsing.md` — markdown shortcut logic inside the editor shell
- `.claude/context/error-handling.md` — JSON error contract across backend and frontend
- `.claude/context/error-boundary.md` — React crash isolation pattern
- `.claude/context/sqlx-offline.md` — SQLx offline workflow
- `.claude/context/users-crud.md` — users CRUD resource template
