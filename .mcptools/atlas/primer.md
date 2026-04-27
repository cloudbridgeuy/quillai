Here is a concise mental model of the **QuillAI Primer** project:

### 1. Purpose
A lightweight workspace providing a **Quill-based markdown editor** with a **Rust/SQLite JSON API** and a **Bun/React frontend**.

### 2. Architecture
- **Backend**: Rust (Axum + SQLx) serving a JSON API, backed by SQLite.
- **Frontend**: Bun (React) serving the SPA, including the Quill editor.
- **Shared**: Logging (`crates/log`), Migrations (`migrations/`), Dev tools (`xtask/`).
- **Pattern**: **Functional Core / Imperative Shell**
  - **Core**: Pure logic (transforms, domain types, validation).
  - **Shell**: I/O, HTTP, SQL, DOM, environment, event handling.
  - **Boundary**: Raw input parsed at boundaries, typed values passed inward.

### 3. Key Modules
| Module | Responsibility |
| :--- | :--- |
| `crates/api/src/main.rs` | Server startup, DB pool, logging wiring. |
| `crates/api/src/lib.rs` | Router composition. |
| `crates/api/src/<resource>/models.rs` | Request types, validation. |
| `crates/api/src/<resource>/handlers.rs` | SQL I/O, HTTP mapping. |
| `crates/api/src/<resource>/mod.rs` | Route wiring. |
| `app/src/md_editor/markdown.ts` | Pure markdown/Delta analysis. |
| `app/src/md_editor/components/Editor.tsx` | Quill DOM effects, event handling. |
| `app/src/api_tester/components/APITester.tsx` | Fetch shell + pure formatter. |
| `app/src/components/ErrorBoundary.tsx` | React error isolation. |

### 4. Conventions & Placement Rules
- **New Backend Resource**: Follow the `users` pattern (Migration → Models → Handlers → Mod → Lib → Tests).
- **New Frontend Feature**: Use feature folders (`md_editor`, `api_tester`) before creating top-level `utils`.
- **Validation**: Next to request types, not inline in handlers.
- **Errors**: Preserve JSON contract `{ "error": string, "code": string }`.
- **Entry Points**: Keep `main.rs` and `index.tsx` thin (compose, don't decide).

### 5. Verification Checklist
- **Backend**: `cargo test -p quillai_api`, `cargo clippy`, `SQLX_OFFLINE=true cargo check`.
- **Frontend**: `bun test --cwd app`.
- **SQLx Cache**: Update `.sqlx/` after query/migration changes.

### 6. Common Traps
- Treating `app/src/index.tsx` as the backend.
- Putting validation directly in handlers.
- Creating generic `utils` too early.
- Copying starter scaffolding instead of active feature patterns.
