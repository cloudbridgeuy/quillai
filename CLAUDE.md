# QuillAI

AI-assisted fiction editor. Rust/Axum API backend + React/Bun frontend with Quill.js.

For agent onboarding, start with `primer.md`, then use this file as the source index.

## Workspace Layout

Cargo workspace with four members:

```
crates/api/    — Axum HTTP server, SQLite, CLI config
crates/log/    — Structured logging library (tracing-based)
crates/delta/  — Quill Delta core types, JSON serde, push/normalize, compose/transform/transform_position/invert/diff (UTF-16 lengths)
xtask/         — Development process runner (`cargo xtask dev`)
app/           — React frontend (Bun runtime, Quill.js editor)
migrations/    — SQLx migrations (SQLite)
docs/          — QuillJS implementation guide, plans, the v1 editor design spec (`docs/editor-design.md`), the design system (`docs/design-system.md`), and the continuous script surface spike (`docs/screenplay-surface.md`)
prototypes/    — Static HTML design prototypes, not built or tested (`prototypes/editor-desktop/` is the visual reference for `docs/editor-design.md`)
```

## Entry Points

| What | Command | File |
|------|---------|------|
| Full dev | `cargo xtask dev` | `xtask/src/main.rs` |
| Dev AI settings | Copy `quillai.env.example` to ignored `quillai.env`; API only, CLI flags > shell env > file | `xtask/src/dev_env.rs` |
| Lint, tests, and project tsc (short output) | `cargo xtask lint` | `xtask/src/main.rs` |
| [Pre-commit lint hook](docs/agent/testing.md) | `cargo xtask install-hooks` | `xtask/src/hooks.rs` |
| [Nightly browser tests](docs/agent/testing.md) | `cargo xtask nightly` | `xtask/src/nightly.rs` |
| [Nightly launchd job](docs/agent/testing.md) | `cargo xtask install-nightly [--uninstall]` | `xtask/src/nightly_launchd.rs` |
| Isolated browser smoke (random frontend port) | `cargo xtask e2e` (or `cargo xtask e2e --port 3107`); prints only failures and a summary line, full log in `target/xtask-e2e/e2e.log`; `--verbose` prints full output | `xtask/src/e2e.rs`, `e2e/playwright.config.ts` |
| Live AI smoke (opt-in) | `cargo xtask ai-smoke [--model ID] [--ai-models FILE]` | `xtask/src/ai_smoke.rs` |
| API server only | `cargo run -p quillai_api --bin api` | `crates/api/src/main.rs` |
| AI model catalog | `cargo xtask dev --ai-models models.json` (also `e2e` and API `--ai-models`; env `QUILLAI_AI_MODELS_FILE`) | `crates/api/src/ai/catalog.rs` |
| Frontend dev only | `PORT=3001 bun --hot app/src/index.tsx` | `app/src/index.tsx` |

API defaults to `127.0.0.1:2908` and SQLite file `sqlite:quillai.db` (created if missing). `--db-url` and `QUILLAI_API_DB_URL` override that URL. `cargo xtask dev` passes an absolute `--db-url` for `<workspace root>/quillai.db` (`sqlite:<path>?mode=rwc`) unless `--db-url` or `QUILLAI_API_DB_URL` is set; those values are forwarded unchanged. Frontend defaults to port `3001` (configurable via `PORT` env var).
Set the frontend URL's `apiBaseUrl` query parameter to configure both AI and manuscript requests, e.g. `http://localhost:3001/?apiBaseUrl=http%3A%2F%2F127.0.0.1%3A49152`. The default is `http://127.0.0.1:2908`. No browser-side `process.env` or rebuild is needed. Only loopback hosts (`127.0.0.1`, `localhost`, `[::1]`) over HTTP(S) with any port are accepted; malformed or non-loopback values fall back to the default without throwing (`app/src/api_config.ts`).
The override does not change CORS: the API defaults to allowing `http://localhost:3001` / `http://127.0.0.1:3001`. Repeat `--cors-origin <origin>` on the API CLI to replace this allow-list. E2E supplies its chosen frontend origin; `cargo xtask dev` defaults stay unchanged.

## Source Index

### Rust — API (`crates/api/src/`)

- `main.rs`, `database.rs`, `chapter_revisions.rs`, `lib.rs`, `cli.rs`, `error.rs`, `prelude.rs`
- `ai/`, `users/`, `settings/`, `eval/`, `bin/eval.rs`
- `manuscripts/series/`, `books/`, `lore/`, `chapters/`, `leaf_sync.rs`, `universes/`, `containers/`, `pieces/`

### Rust — Logging (`crates/log/src/`)

- `lib.rs`

### Rust — Build (`xtask/src/`)

- `main.rs`, `dev_database.rs`, `check_ui.rs`, `check_i18n.rs`, `check_claude_md.rs`, `e2e.rs`, `ai_smoke.rs`, `hooks.rs`, `nightly.rs`, `nightly_launchd.rs`

### Browser Smoke (`e2e/`)

- See [E2E specs](docs/agent/e2e-specs.md).

### Frontend (`app/src/`)

- `index.tsx`, `index.html`, `logo.svg`, `tab_title.ts`, `App.tsx`, `frontend.tsx`, `index.css`, `api_config.ts`
- `landing/`, `components/`, `ui/`, `i18n/`, `manuscripts/`, `settings/`, `md_editor/`
- `fixtures/prompt/`, `fixtures/delta/`

### Database

- `migrations/`, `crates/api/src/database.rs`, `crates/api/tests/`

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
- Every ticket fixing or changing browser-visible behaviour ships with an E2E regression spec. Write the spec but do not run it inside the ticket.

### Bun APIs

- `Bun.serve()` supports WebSockets, HTTPS, and routes. Don't use `express`.
- `bun:sqlite` for SQLite. Don't use `better-sqlite3`.
- `Bun.redis` for Redis. Don't use `ioredis`.
- `Bun.sql` for Postgres. Don't use `pg` or `postgres.js`.
- `WebSocket` is built-in. Don't use `ws`.
- Prefer `Bun.file` over `node:fs`'s readFile/writeFile
- Bun.$\`ls\` instead of execa.

### Rust Conventions

- Delta parity: see [Delta parity](docs/agent/delta-parity.md).
- Workers run `cargo xtask lint` and never `cargo xtask e2e`. Red/green uses a unit or integration test. Browser specs run nightly (GUZ-1206); see [testing policy](docs/agent/testing.md).
- MSRV is Rust 1.95.0 or higher, set as `rust-version` in the workspace `Cargo.toml` and inherited by every member crate.
- Workspace crates under `crates/`. New crates go there.
- `cargo xtask` for build automation — extend `xtask/src/main.rs` for new tasks.
- Tracked `.rs` files under `crates/` and `xtask/` must stay at or below 1000 lines (enforced by `cargo xtask lint`).
- SQLx with compile-time checked queries (`.sqlx/` cache). `.cargo/config.toml` sets `SQLX_OFFLINE=true` so builds use that cache; a shell `SQLX_OFFLINE=false` overrides it. See [SQLx offline context](.claude/context/sqlx-offline.md).
- Errors via `thiserror` with manual `IntoResponse` for JSON responses. See [error handling context](.claude/context/error-handling.md).

### Frontend Conventions

- Read [docs/design-system.md](docs/design-system.md) before adding or changing UI. Use `app/src/ui/` primitives and `app/src/ui/tokens.css` tokens; do not add raw controls, hex colours, font sizes or Tailwind arbitrary values elsewhere (`cargo xtask lint` check `ui` enforces this).
- UI system first: a new or changed `app/src/ui/` primitive gets its story before any screen uses it ([rule](docs/design-system.md)).
- HTML imports with `Bun.serve()`. Don't use `vite`.
- HTML files import .tsx/.jsx/.js directly — Bun transpiles and bundles.
- `<link>` tags point to stylesheets — Bun's CSS bundler handles them.
- Tailwind via `bun-plugin-tailwind`.
- For Bun API docs: `node_modules/bun-types/docs/**.md`.

## Context Docs

`.claude` is gitignored; the docs below stay tracked, but a new context doc must be added with `git add -f`.

- `.claude/context/ai-streaming.md` — AI request validation, prompt rules, provider env vars, and NDJSON stream behavior
- `.claude/context/ai-editor.md` — Quill AI editor shell, helper modules, and the current browser URL constraint
- `.claude/context/markdown-parsing.md` — markdown shortcut logic inside the editor shell
- `.claude/context/error-handling.md` — JSON error contract across backend and frontend
- `.claude/context/error-boundary.md` — React crash isolation pattern
- `.claude/context/sqlx-offline.md` — SQLx offline workflow
- `.claude/context/users-crud.md` — users CRUD resource template

## Agent docs

CLAUDE.md is an index capped at 16,000 bytes (`cargo xtask lint` check). Put ticket-level detail (file behaviour, spec coverage, routes, schema) in the matching docs/agent/ file, not here.

- [Sync push design](docs/sync-push.md) — SSE change nudges, existing delta reads, and slow fallback (GUZ-905).

| File | Lookup purpose |
|------|----------------|
| [source-index.md](docs/agent/source-index.md) | Source file behaviour |
| [testing.md](docs/agent/testing.md) | Test layers, red/green, and commit gate |
| [e2e-specs.md](docs/agent/e2e-specs.md) | Browser smoke coverage |
| [api-routes.md](docs/agent/api-routes.md) | API routes |
| [database.md](docs/agent/database.md) | Database schema and migrations |
| [delta-parity.md](docs/agent/delta-parity.md) | Delta parity rules |
