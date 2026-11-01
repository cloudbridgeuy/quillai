# API Routes

## API Routes

**Rust backend** (`crates/api/src/`):
- `GET /users` — List all users (JSON array)
- `POST /users` — Create user (201 + JSON)
- `GET /users/{id}` — Get user by ID (JSON or 404)
- `PUT /users/{id}` — Partial update user (JSON, 404, or 422)
- `DELETE /users/{id}` — Delete user (204 or 404)
- `GET /series` — List series ordered by position, then ID (JSON array)
- `POST /series` — Create series with nonblank title and nonnegative integer position (201 + JSON or 422)
- `GET /series/{id}` — Get series by ID (JSON or 404)
- `PUT /series/{id}` — Partial update title and/or position (JSON, 404, or 422); omitted/null fields stay unchanged
- `DELETE /series/{id}` — Delete series and cascade children (204 or 404)
- `GET /series/{id}/books` — List books ordered by position, then ID (JSON array or 404 for missing series)
- `POST /series/{id}/books` — Create book with nonblank title and nonnegative integer position; parent from path (201 + JSON, 404, or 422)
- `GET /series/{id}/books/{book_id}` — Get book scoped to series (JSON or 404)
- `PUT /series/{id}/books/{book_id}` — Partial update title and/or position, scoped to series (JSON, 404, or 422); omitted/null fields stay unchanged
- `DELETE /series/{id}/books/{book_id}` — Delete book scoped to series and cascade chapters (204 or 404)
- `GET /series/{id}/lore` — List author lore ordered by name, then ID, including hidden notes (200 JSON array or 404 for missing series)
- `POST /series/{id}/lore` — Create lore with required kind (character/place/term/note) and nonblank name; optional fields retain migration defaults (201 + JSON, 404, or standard JSON 422 for invalid fields/types). Keywords/aliases are arrays of nonblank strings; pinned is boolean; level is include/known_dont_reveal/leave_out
- `GET /series/{id}/lore/{lore_id}` — Get author lore scoped to series, including hidden notes (200 JSON or 404)
- `PUT /series/{id}/lore/{lore_id}` — Partial lore update scoped to series (200 JSON, 404, or standard JSON 422); omitted/null fields unchanged, including pinned=false as an explicit update
- `DELETE /series/{id}/lore/{lore_id}` — Soft-delete lore scoped to series; `?permanent=true` permanently deletes active or deleted lore (204 or 404)
- `GET /series/{id}/books/{book_id}/chapters` — List chapters ordered by position, then ID; default returns only id, title, position, kind, head_rev without reading content or revisions. `?content=true` returns full chapters with book_id and rebuilt head content; any other supplied content value, including false or empty, returns 422 (JSON array, 404, or 422)
- `POST /series/{id}/books/{book_id}/chapters` — Create chapter with title, position, kind (chapter/prologue/epilogue), and optional Delta content; snapshot at revision zero (201 + JSON, 404, or 422). Delta input requires an object with an ops array and valid operations; omitted content defaults to an empty Delta
- `GET /series/{id}/books/{book_id}/chapters/{chapter_id}` — Get chapter rebuilt head content and head_rev, scoped to book and series (JSON or 404)
- `PUT /series/{id}/books/{book_id}/chapters/{chapter_id}` — Partial update title, kind, and/or position; omitted/null fields unchanged; any content key rejected (JSON, 404, or 422)
- `DELETE /series/{id}/books/{book_id}/chapters/{chapter_id}` — Delete chapter scoped to book and series (204 or 404)
- `POST /series/{id}/books/{book_id}/chapters/{chapter_id}/deltas` — Submit `{baseRev, delta}`; transform against later retained revisions with server priority, append atomically, return `{rev, delta}` (200, 404, 409 unavailable/future history, or 422 malformed JSON/Delta or compose failure)
- `GET /series/{id}/books/{book_id}/chapters/{chapter_id}/deltas?since=rev` — Return missed revisions ordered ascending as `[{rev, delta}]`; current head returns `[]` (200, 404, 409 unavailable/future history, or 422 invalid query)
- `POST /series/{id}/lore/{lore_id}/restore` — Restore deleted lore (200 JSON or 404); deleted lore is excluded from list, get, and update
- Old `/series`, `/books`, `/chapters`, deltas, and lore routes are a façade over the story model with unchanged JSON: series are `book_series` containers, books are prose `novel` groups (id = `legacy_book_id`), chapters are their prologue/chapter/epilogue leaves (id = `legacy_chapter_id`), and series lore is the container's universe lore (shared by series in one universe). `POST /series` creates a universe plus container; `DELETE /series/{id}` deletes the universe too when it has no containers or pieces left
- `GET/POST /universes`, `GET/PUT/DELETE /universes/{id}` — title/position, series rules (201, 404, 422); DELETE cascades containers, pieces, lore
- `GET/POST /universes/{id}/lore`, `GET/PUT/DELETE /universes/{id}/lore/{lore_id}` — lore fields and deletion semantics as series lore, JSON `universe_id`; `POST /universes/{id}/lore/{lore_id}/restore` restores deleted lore
- `GET/POST /universes/{id}/containers`, `GET/PUT/DELETE /containers/{id}` — `{id, universeId, title, position, kind}`; `kind` or `universeId` on PUT is 422
- `GET /universes/{id}/pieces?container=none` — loose roots; other or missing `container` is 422. `POST /universes/{id}/pieces` — root in storage `{title, position, label, shape, format?}`; format defaults to script for screenplay/tv_script/episode, else prose; a leaf starts with an empty Delta
- `GET/POST /containers/{id}/pieces` — root pieces; book_series allows novel/novella, tv_series tv_script/episode (422 otherwise)
- `POST /pieces/{id}/children` — new empty leaf under a group with the group's format; label defaults to chapter (prose) or scene (script); non-group is 422
- `GET/PUT/DELETE /pieces/{id}` — leaf adds `content`/`headRev`, group adds `children`; PUT changes title/position/label/format (group format cascades to children); `universeId`, `containerId`, `parentId`, `shape`, `content`, a child format change, or a membership break is 422
- `POST/GET /pieces/{id}/deltas` — leaves only (group 404); same JSON and 409 rules as chapter deltas
- `GET /pieces/{groupId}/scenes` — a script group opened in one GET: the group's piece fields plus `scenes: [{...piece, content, headRev}]`, every child in `position, id` order, each rebuilt like a leaf `GET /pieces/{id}` in one transaction (200, 404 missing, 422 for a leaf or a prose group)
- `GET /pieces/{groupId}/scenes/deltas?since=<sceneId>:<rev>,...` — one poll for a script group: `{children: [id...], scenes: {"<id>": {revisions: [{rev, delta}]} | {conflict: "future" | "unavailable"}}}`. `children` is the current order; `scenes` holds only the requested ids (empty `since=` asks for none). Per-scene leaf history rules; a scene with future/negative or missing history gets `conflict` and the others still return (200). A scene not in the group, a repeated id, a malformed or missing `since` is 422; missing group 404, leaf or prose group 422. Posts stay on `/pieces/{id}/deltas`
- `POST /pieces/{id}/placement` — `{containerId: number|null, position}`; a root moves to that container or storage (null); a child accepts only null (detach: clears `parentId`). Returns the piece detail (200, 404, or 422 for another universe, a missing container, a membership break, or a child with a container)
- `POST /pieces/{id}/split` — `{label?}`; a root leaf becomes the position-0 child of a new group that copies its universe, container, title, position, label, and format; child label defaults to chapter (prose) or scene (script). Returns the group detail with children (201, 404, or 422 for a group, a child, or an unknown label)
- `POST /pieces/{groupId}/attach` — `{pieceId, position}`; a root leaf in the same universe and format becomes a child of that group, clearing `containerId` in the same UPDATE. Returns the group detail (200, 404 for a missing group, or 422 for a non-group target, a missing/group/child source, another universe, or another format)
- `POST /ai/generate` — Stream AI delta fragments as NDJSON; Continue removes a case-insensitive, whitespace-normalized overlap of at least 12 characters against the last 512 characters before the caret; optional `model` must exactly match a startup catalog ID (422 if invalid); omission uses the action default. See [AI streaming context](../../.claude/context/ai-streaming.md)
- `GET /ai/models` — Return startup catalog model IDs, labels, input/output prices per million tokens, and `defaults.generate` (Continue) / `defaults.edit` IDs. No provider configuration or `include_usage`; missing catalog returns the standard 500 configuration error.
- `GET /settings/author-guidance` — Return `{text: string}`; missing row returns empty text (200).
- `PUT /settings/author-guidance` — Save `{text: string}` and return saved text (200); empty clears guidance, more than 65,536 UTF-8 bytes returns 422. Invalid JSON/types return the standard 400 JSON error.

**Bun frontend** (`app/src/index.tsx`):
- `GET /assets/*` — static file serving
- `GET /api/hello` — JSON hello
- `PUT /api/hello` — JSON hello (PUT)
- `GET /api/hello/:name` — parameterized JSON hello
- `GET /*` — SPA fallback to index.html
