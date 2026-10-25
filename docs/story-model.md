# Flexible story model

Design spike at `ed5ae04`. Docs only. It replaces the fixed series → book → chapter chain with works that can stand alone, optional containers, and a text row that owns revisions.

Current chain, all required: `series` (`migrations/20261019000000_series_books_chapters.up.sql:1`), `books.series_id NOT NULL` (`:9`), `chapters.book_id NOT NULL` (`:18`). A book cannot exist outside a series. That is the gap behind [GUZ-892](https://linear.app/guzmonne/issue/GUZ-892/standalone-stories-outside-books-and-series) (Backlog, child of [GUZ-885](https://linear.app/guzmonne/issue/GUZ-885/e12-navigation-landing-and-manuscript-management)).

## Recommendation

Agree with the spine of the starting proposal. Refine the parts that made episodes both works and sections, and that hung revisions on chapters only.

- A **work** is the unit a writer names. Kind: `piece`, `short_story`, `novella`, `novel`, `screenplay`, `tv_script`. `container_id` is null when it stands alone. This is GUZ-892. Do not implement GUZ-892 by making `books.series_id` nullable.
- A **container** is optional and holds many works. Kind: `book_series`, `tv_series`, `collection`. One work has at most one container. Deleting a container deletes its works, matching `ON DELETE CASCADE` today (`migrations/20261019000000_series_books_chapters.up.sql:9`).
- **Format** is a field on the work: `prose` or `script`. `screenplay` is script. `tv_script` defaults to script and may be prose. Every other kind is prose.
- **Sections** exist only on some works. Novel and novella: `prologue`, `chapter`, `epilogue`. Script screenplay and script episode: `scene`. Piece, short story, and prose episode: no sections; the work holds one text.
- A **text** owns `content`, `head_rev`, `snapshot_rev`, and the revision rows. Moving a work between containers does not touch the text. Absorbing a standalone piece into a sectioned work re-parents that same text onto a new section, so revisions stay addressable.
- **Lore** has one owner: the container if the work is in one, otherwise the work. Matching and the AI request body stay as they are. The editor changes which list it passes in.
- Migration maps today's rows without reinterpretation: each series becomes a `book_series`, each book a prose `novel`, each chapter a section with the same role. Ids are kept.

Rejected from the starting proposal: episodes as sections of a TV work while a TV series also holds episode-works. An episode is a `tv_script` work inside a `tv_series`. A scene is a section of a script (Fountain headings inside that text are formatting, §6). Nesting a work inside a work is rejected too. Moving a piece into a work absorbs it onto a new section.

## 1. Types and schema

Rust and TypeScript share these sums. Invalid pairs are not representable in the types; SQLite triggers repeat the same rules so a raw write cannot bypass them.

```text
ContainerKind = book_series | tv_series | collection
WorkKind      = piece | short_story | novella | novel | screenplay | tv_script
TextFormat    = prose | script
SectionRole   = prologue | chapter | epilogue | scene

Container { id, title, position, kind }
Work      { id, title, position, kind, format, container_id: Option<Id> }
Section   { id, work_id, title, position, role }
Text      { id, content, head_rev, snapshot_rev, parent: Work(Id) | Section(Id) }
Revision  { text_id, rev, delta, created_at }          // rev starts at 1
LoreOwner = Container(Id) | Work(Id)
```

Format and structure:

| Kind | Format | Text parent |
| --- | --- | --- |
| piece, short_story | prose | the work; no sections |
| novella, novel | prose | one text per section; roles prologue, chapter, epilogue |
| screenplay | script | one text per section; role scene |
| tv_script | script (default) or prose | script: scene sections; prose: one text on the work, no sections |

Container membership: `book_series` holds novel and novella; `tv_series` holds `tv_script`; `collection` holds piece, short story, novella, and screenplay. Any kind may stand alone. A novel is not a collection member (open question 1). A piece does not sit beside novels in a book series; it is absorbed as a chapter or placed in a collection.

`position` stays a nonnegative integer and is not unique (`crates/api/src/manuscripts/series/models.rs:31`). Lists stay `ORDER BY position, id` (`series/handlers.rs:14`, `books/handlers.rs:22`, `chapters/handlers.rs:49`). Title stays nonblank (`series/models.rs:24`).

### Tables

New migration after `migrations/20261020000000_lore_entries.up.sql`. `users` and `author_guidance` stay. Rebuild, because SQLite cannot add the XOR checks or drop `NOT NULL` on `books.series_id` in place.

`containers` — `id INTEGER PRIMARY KEY`, `title TEXT NOT NULL`, `position INTEGER NOT NULL`, `kind TEXT NOT NULL` checked against `ContainerKind`.

`works` — `id`, `title`, `position`, `kind`, `format TEXT NOT NULL`, `container_id INTEGER NULL REFERENCES containers(id) ON DELETE CASCADE`. Index `(container_id, position)`.

`sections` — `id`, `work_id INTEGER NOT NULL REFERENCES works(id) ON DELETE CASCADE`, `title`, `position`, `role`. Index `(work_id, position)`.

`texts` — `id`, `work_id NULL REFERENCES works(id) ON DELETE CASCADE`, `section_id NULL REFERENCES sections(id) ON DELETE CASCADE`, `content TEXT NOT NULL DEFAULT '{"ops":[]}' CHECK (json_valid(content))`, `head_rev` and `snapshot_rev` with the same checks as `migrations/20261019000001_chapter_revisions.up.sql:1`. Check exactly one parent. `UNIQUE(work_id)`, `UNIQUE(section_id)`. SQLite UNIQUE treats NULL as distinct, so those uniques only cap the non-null parent; the check is what forbids both parents and neither.

`text_revisions` — same columns as `chapter_revisions` (`:4`) with `text_id` instead of `chapter_id`, `UNIQUE(text_id, rev)`, `ON DELETE CASCADE`.

`lore_entries` — rebuild with `container_id NULL REFERENCES containers(id) ON DELETE CASCADE` and `work_id NULL REFERENCES works(id) ON DELETE CASCADE`, exactly one set. Other columns, checks, and defaults stay (`migrations/20261020000000_lore_entries.up.sql:1`). Index each owner column. Order stays `name, id` (`lore/handlers.rs:34`).

Triggers:

- Format matches the table above.
- Section `role` matches the parent work. Body-kinds reject any section. Section-kinds reject a text whose parent is the work. A work with a body text rejects a section.
- Lore `work_id` is allowed only when that work's `container_id` is null.

### Migration from current rows

Copy inside one transaction. Preserve ids.

| From | To |
| --- | --- |
| `series` | `containers` with the same id, title, position, kind `book_series` |
| `books` | `works` with the same id, `container_id = series_id`, kind `novel`, format `prose` |
| `chapters` | `sections` with the same id, `work_id = book_id`, same title, position, and role (`chapter`, `prologue`, or `epilogue`, the check at `chapters/models.rs:154`) |
| each chapter | one `texts` row, new id, `section_id` = chapter id, `content` / `head_rev` / `snapshot_rev` copied |
| `chapter_revisions` | `text_revisions` with `text_id` = the new text id, same `rev`, `delta`, `created_at` |
| `lore_entries.series_id` | `lore_entries.container_id`, `work_id` null, same id and fields |

There is no body text on a migrated novel; empty books stay works with zero sections (`ManuscriptTree.tsx:62` already shows "No chapters yet."). Update `sqlite_sequence` for the explicit ids. Then drop `chapter_revisions`, `chapters`, `books`, `series`, and the old `lore_entries`.

No data loss: every Delta, revision, position, title, kind, and lore field is copied. Meaning is unchanged because every existing series is a book series of novels. The migration does not guess novella, short story, or script.

### Down migration

The older down scripts still expect the pre-migration schema. `20261019000001_chapter_revisions.down.sql` drops `chapter_revisions` and alters `chapters`. `20261020000000_lore_entries.down.sql` drops `lore_entries`. This migration's down must restore that shape before those files run.

Abort first, with no writes, when any row cannot be represented: a container other than `book_series`; a work with `container_id` null, kind other than `novel`, or format other than `prose`; a section role outside `prologue` / `chapter` / `epilogue`; a text whose parent is a work; lore with `work_id` set. SQLite has no top-level `RAISE`. Use a guard table and a `BEFORE INSERT` trigger that `RAISE(ABORT, …)` when such a row exists, then insert one guard row. A refused down leaves the new schema in place.

When the guard passes: recreate `series`, `books`, `chapters` (including `content`, `head_rev`, `snapshot_rev`), `chapter_revisions`, and `lore_entries` with `series_id`, preserving ids, then drop the new tables.

## 2. API

New handlers are thin over the tables. Old handlers stay mounted (`crates/api/src/manuscripts/mod.rs:10`) and read the same tables. No dual-write.

### New routes

| Method and path | Body / result |
| --- | --- |
| `GET/POST /containers`, `GET/PUT/DELETE /containers/{id}` | `kind` on create; same title and position rules as series |
| `GET/POST /containers/{id}/works` | works in that container |
| `GET /works?container=none` or `?container={id}` | standalone works, or one container's works |
| `POST /works` | standalone work. Body-kind also inserts an empty text. Section-kind does not insert a section |
| `GET/PUT/DELETE /works/{id}` | kind and format changes use the rules in §3 |
| `POST /works/{id}/placement` | `{ "containerId": number \| null, "position": number }` |
| `POST /works/{id}/sections` | create section and its empty text |
| `GET/PUT/DELETE /works/{id}/sections/{sectionId}` | metadata only. `content` on PUT is 422, as `UpdateChapter` rejects `content` (`chapters/models.rs:170`) |
| `POST /works/{id}/sections/absorb` | `{ "workId", "position", "role" }` |
| `GET /texts/{id}` | rebuilt head Delta and `head_rev` |
| `POST /texts/{id}/deltas`, `GET /texts/{id}/deltas?since=` | same JSON as today: `{ baseRev, delta }` → `{ rev, delta }`, and `[{ rev, delta }]` |

Lore, only the legal owner:

- `GET/POST /containers/{id}/lore` and `GET/PUT/DELETE /containers/{id}/lore/{loreId}`
- `GET/POST /works/{id}/lore` and the item route. 422 when the work has a container.

Parent scoping stays as strict as `sync.rs:25`: a section route 404s when the section is not in that work, and a text route 404s when the id is unknown. Validation failures stay 422. 409 stays reserved for revision history (`IncompleteHistory` in `chapter_revisions.rs:17`, future or gapped `since`).

Delta sync moves from `chapter_id` to `text_id` and otherwise does not change. `RevisionStore::append` still takes the expected head, inserts `rev = head + 1`, transforms with server priority (`sync.rs:58`), and snapshots every 100 revisions (`sync.rs:55`, `chapter_revisions.rs:104`). `rebuild` is still snapshot composed with later deltas (`chapter_revisions.rs:38`). The client sync core already speaks `baseRev` and deltas (`app/src/md_editor/chapter_sync.ts:41`); the session (`chapter_session.ts:47`) is what learns a `textId`. The browser URL keeps `seriesId`, `bookId`, and `chapterId` (`chapter_session.ts:25`) as a compatibility load: those three resolve the section, then its text. A standalone work or a scene uses `textId`.

`docs/eval.md` exports `GET /series/{id}/books/{book_id}/chapters?content=true`. It keeps working through the façade for migrated novels. Point it at works and texts only when the old routes are removed.

### Compatibility façade

Old JSON stays the fields current parsers read (`app/src/manuscripts/contract.ts:1`). Extra fields are not added.

| Current route | Reads and writes |
| --- | --- |
| `/series` (`series/mod.rs:10`) | containers with `kind = book_series` only. Other kinds are absent. `GET /series/{id}` is 404 for them. `POST` creates a `book_series` |
| `/series/{id}/books` (`books/mod.rs:10`) | works in that container with kind `novel` and format `prose`. `series_id` in JSON is `container_id`. `POST` creates that pair. A novella in the series is hidden |
| `/series/{id}/books/{book_id}/chapters` (`chapters/mod.rs:11`) | sections of that novel. `book_id` is `work_id`. Content is the section's text, rebuilt the same way as `Chapter` (`chapters/models.rs:28`) |
| `…/chapters/{chapter_id}/deltas` (`chapters/mod.rs:15`) | look up `texts.section_id = chapter_id`, then the text sync above. Response shape unchanged |
| `/series/{id}/lore` (`lore/mod.rs:10`) | lore with that `container_id`. Work-owned lore is not on this route |

Standalone works have no series path, so the old tree cannot see them. That is intentional: GUZ-892 shows up on the new routes.

Remove the façade in the ticket after the landing page, not in the schema ticket. Until then, existing HTTP tests (`series_test`, `books_test`, `chapters_test`, `lore_test`, `chapter_revisions_test`) and `e2e/chapter-sync.spec.ts` are the compatibility proof. New specs have to be added to `testMatch` in `e2e/playwright.config.ts:5`.

## 3. Move operations

Positions are not renumbered. The caller sends the new `position` (≥ 0). Order among ties remains `position, id`.

**Placement** (`POST /works/{id}/placement`) changes `container_id` and `position` only.

- Target must allow the work's kind (§1). 422 otherwise.
- Into a container: in the same transaction, `UPDATE` lore from `work_id` to that `container_id` and clear `work_id`. Names are not unique today, so nothing is merged.
- From container A to container B: the work has no private lore (the trigger). A's lore stays on A. The work then sees B's lore.
- Out to standalone: `container_id` becomes null. Lore stays on the container. The work arrives with an empty lore list. A later "take a copy" is open question 2, not part of this call.
- Texts, sections, and `text_revisions` are not rewritten. A client posting deltas to that `textId` keeps its base revision. Two tabs converge the same way as today.

**Absorb** (`POST /works/{target}/sections/absorb`) is how a piece enters a work.

- Source is standalone (`container_id` null) and sectionless (one body text): piece, short story, or prose episode. 422 if it has a container or any section. Detach first; shared container lore is not copied.
- Target is sectioned: novel, novella, or a script screenplay / script episode. 422 for a piece, short story, or prose episode as the target.
- `role` is legal for the target.
- Transaction: insert the section at `position` on the target; set the source text's parent to that section (`section_id`, `work_id` null) before deleting the source, so `ON DELETE CASCADE` does not drop the text; reassign source lore to the target's container if it has one, otherwise to the target work; delete the source work.
- The text id, `head_rev`, and every revision row stay. The new section id is new. An editor open on the source retargets its route to the new section or to `textId`; the sync key does not change.

**Kind and format change** (`PUT /works/{id}`), same transaction:

- Inside one structure, metadata only: `piece` ↔ `short_story`; `novel` ↔ `novella`; `tv_script` stays script.
- Body to sectioned (for example short story → novel, or piece → screenplay): insert one section (`chapter`, or `scene` for a script), re-parent the body text onto it.
- Sectioned to body (novel → short story, script episode → prose episode): allowed only when there is exactly one section. Re-parent that text onto the work and delete the section. Several sections would mean merging revision chains; 422.
- `screenplay` does not become prose. A prose `tv_script` does not become a screenplay in one step; it can become a script `tv_script` only through the body-to-sectioned rule (one new scene).

Delete of a work deletes its sections, texts, revisions, and work-owned lore. Container-owned lore is not deleted with one work.

## 4. Lore and the AI request

`matchLoreEntries` (`app/src/md_editor/lore_match.ts:16`) takes a string and a list. It does not know series or works. Pins, keyword edges, NFC, the UTF-16 budget, and skip-when-over-budget stay. `buildAiRequest` still drops `leave_out` and sends `{ name, kind, visible_text, level }` (`ai_contract.ts:102`, `ai_contract.ts:122`). The server accepts that object and rejects unknown fields (`crates/api/src/ai/lore.rs:8`) and prints `<lore>` from the request array (`crates/api/src/ai/prompt.rs:28`). It does not load lore from SQLite. Hidden notes stay off the request (`lore_client.ts:6` holds them; `AiLoreEntry` does not).

What changes is the list. Today `useLore(chapterSync.scope?.seriesId)` (`Editor.tsx:338`) and `LorePanel` (`Editor.tsx:1245`) always load `/series/{id}/lore`. `use_lore.ts:10` clears entries when `seriesId` changes.

After the change, the open text resolves one owner: the work's `container_id`, or the work id when that is null. The panel and `loreEntries` load that list only. A container move swaps the list in one step, the same way a series change does now. Do not union container lore with work lore; the check constraint makes a union a bug. `LoreEntry.series_id` (`lore_client.ts:6`) becomes the owner discriminant (`container_id` or `work_id`). Request JSON is unchanged, so Continue and Edit prompts do not gain a scope field.

## 5. UI

The manuscript drawer is still series, then books, then chapters, and only a chapter is selectable (`ManuscriptTree.tsx:7`, `ManuscriptTree.tsx:54`). Empty copy is "No series yet." Opening a chapter still saves first (`ManuscriptTree.tsx:53`, `docs/editor-design.md:82`). Failed save keeps the current text. Those rules stay.

New tree, one list ordered by `position, id`, keys prefixed so a work id cannot collide with a container id:

- Each container is a disclosure. Label the kind in the summary (book series, TV series, collection). Children are its works.
- A sectioned work is a disclosure of sections. Empty copy: "No chapters yet." or "No scenes yet."
- A sectionless work is a `ListButton` that opens its text. A section `ListButton` opens that section's text.
- Standalone works are roots beside containers, not a fake series. That is the GUZ-892 tree. "No series yet." becomes empty only when there are no containers and no works.

Selection highlight follows the open text. `editor-design.md:82` still applies: one GET of that text, and a failed save does not switch.

[GUZ-891](https://linear.app/guzmonne/issue/GUZ-891/create-chapters-books-and-series-from-the-app) is In Progress on the current model (Linear, 2026-10-25). `docs/e2e-coverage.md:46` still lists it as Backlog. This worktree has one create control, the empty-state "Create chapter" button, which bootstraps "My Series" / "My Book" / "Chapter One" (`Editor.tsx:1257`, `chapter_session.ts:12`). Let GUZ-891 land against the façade. Do not block it on this schema, and do not add a second standalone path that GUZ-892 would then throw away. The story-model create ticket replaces those actions.

Create, using `Button` and `ListButton` (`docs/design-system.md:7`; there is no `Select` primitive yet, so kind is a short button list until one exists under `app/src/ui/`):

- New container: title and kind.
- New work: title and kind, optional container when membership allows. Body-kind opens its text. Section-kind opens the work and shows the empty section state.
- New section: on a sectioned work, title and legal role.
- Placement and absorb: actions on a work row, calling §3. Absorb is offered only for a standalone sectionless work.

[GUZ-895](https://linear.app/guzmonne/issue/GUZ-895/rename-series-books-and-chapters-optional-chapter-numbers) (Backlog) is still title and position on container, work, and section. Optional chapter numbers stay a display of section `position` for role `chapter`. This schema does not add a number column.

[GUZ-894](https://linear.app/guzmonne/issue/GUZ-894/landing-page) (Backlog): the app currently opens the editor and, with no chapter, offers bootstrap (`chapter_session.ts:44` throws `NoEditorChapter` only when the URL has no ids and no chapter exists). The landing page lists the same roots as the tree (containers and standalone works) and recent texts by `text_revisions.created_at`. Resume opens that text. Create on the landing page is the same API as the tree, so creating a piece does not require the editor to be open (the second sentence of GUZ-892). Usage tips are out of this spike. The left-drawer lore and export behavior in `docs/editor-design.md:109` follows the open text's lore owner; export is still the local text, one text at a time.

## 6. Script formatting (later phase)

Schema phase stores `format = script` and scene sections. It does not teach the editor screenplay layout.

A script text needs block types: scene heading, action, character, dialogue, parenthetical, transition. Fountain synopsis, notes, lyrics, and centered lines wait. FDX export is the same blocks as XML paragraphs. Import waits behind export.

Quill is constructed with the snow theme and `history.userOnly` only (`Editor.tsx:204`). Markdown shortcuts apply bold and italic (`markdown.ts:3`, `markdown.ts:21`). Store each screenplay block as a Quill block attribute on the newline. `quill-delta` and `crates/delta` already compose and transform attributes. Do not add an op kind, an embed, or a Delta-crate change. The parity rule in `CLAUDE.md` stays. `buildAiRequest` rejects object inserts (`ai_contract.ts:154`); attributes on text inserts still pass.

Export walks the work's scenes in `position, id` order and writes one Fountain or FDX document. A scene heading attribute inside a scene and the scene section in the tree are different: the section is the revision boundary used by absorb; the heading is formatting. Markdown shortcuts turn off when `format` is script, so `*` and `_` are not eaten.

This phase does not change sync. Attributes ride the same text deltas.

## 7. Tickets, risks, open questions

Ordered. Browser-visible tickets include an E2E spec and a `testMatch` entry. No Linear writes in this spike. GUZ-892 is closed by tickets 4, 5, and 7 together, not by a nullable `series_id`.

| # | Ticket | Proof |
| --- | --- | --- |
| 1 | Migration up/down, id preservation, guard refuses a standalone work | `crates/api/tests/story_model_migration_test.rs` (not browser-visible) |
| 2 | New read/create routes and the façade on the new tables. Old JSON unchanged | Existing `crates/api/tests/*_test.rs` plus `e2e/chapter-sync.spec.ts` (two tabs, reload, server content). No new visible behavior |
| 3 | Placement, absorb, kind change. Text id and revisions stable; lore follows §3 | Rust HTTP tests. Not browser-visible until ticket 4 |
| 4 | Tree lists containers, standalone works, and sections; opening a piece loads its text | `e2e/story-tree.spec.ts`. API-seed a series novel and a standalone piece; both visible; open the piece; reload; server text matches. Chapters of the novel still open |
| 5 | In-editor create for container, work, and section, replacing the GUZ-891 controls once they exist | `e2e/create-work.spec.ts`. Create a piece with no series, a novel in a new book series, and a TV series with a script episode; reload shows all three |
| 6 | Lore panel and Continue/Edit use the owner from §4 | `e2e/lore-scope.spec.ts`. Piece lore is in the Continue body and hidden notes are absent; after placement into a collection, the entry is on the collection, the piece has no private list, and another work in the collection sees it |
| 7 | Landing (GUZ-894) on these roots, including create without opening the editor | `e2e/landing.spec.ts`. Fresh app shows the landing list, not the sheet; resume opens the text; create piece from the landing page |
| 8 | Script block formats, markdown shortcuts off, Fountain and FDX export | `e2e/screenplay-format.spec.ts`. Scene heading and dialogue survive reload; downloaded Fountain contains those lines. Depends on tickets 1–5 |
| 9 | Remove `/series`, `/books`, `/chapters`, and series lore routes; point `docs/eval.md` at works and texts | Rust tests that the old paths are 404. Browser covered by tickets 4–7 still passing on the new routes |

GUZ-895 stays its own ticket and can ship against the façade (rename series, book, chapter) before ticket 9.

### Risks

- GUZ-891 will add create UI on series/book/chapter. Ticket 5 has to absorb whatever it merges. The façade is what makes that order safe.
- Two route sets can drift. Old handlers must be queries on the new tables, not a second write model. Ticket 2's existing tests are the check.
- A down migration on a database that already has a piece, a script, or work-owned lore aborts on purpose. Do not "fix" that by deleting those rows.
- Preserved ids keep old editor URLs working for migrated novels only. A piece never had `seriesId` / `bookId` / `chapterId`.
- Absorb deletes the source work after re-parenting the text. The order in §3 is load-bearing: `texts.work_id` is cleared before the work row is deleted, in the same transaction, or cascade deletes the revisions.
- An open tab whose route is the source work must retarget after absorb. Delta POST to the text id still succeeds. Ticket 4's spec should absorb through the API while a tab is open and show the tab on the new section after refresh.
- Eval export (`docs/eval.md:8`) breaks at ticket 9 if the CLI is not updated in that same ticket.
- Screenplay embeds would break the text-only AI slice and the Delta parity rule. Ticket 8 is attributes only.
- New Rust files have to stay at or below 1000 lines (`CLAUDE.md`). New route modules, not more cases in `chapters/handlers.rs`.
- `Editor.tsx:1257` is a raw `<button>`, already outside the primitive set. New create controls do not add another one (`docs/design-system.md:7`).

### Open questions

1. Can a collection contain a novel? This spike says no: novels and novellas go in a book series; a collection holds pieces, short stories, novellas, and screenplays. A novel that should sit with short stories would need a yes.
2. Is "take a copy of container lore with me" required when a work leaves a container? This spike leaves shared lore behind and does not copy it on the way out or on absorb of a piece that was already in a container (absorb of a contained piece is 422; detach first).
3. Confirm scene sections plus concatenated Fountain/FDX, against one text per screenplay. Sections are what let absorb keep a revision chain. One text matches a single Fountain file more closely and cannot absorb a piece without merging histories.
4. Remove the old routes in ticket 9, after landing, or keep them while `docs/eval.md` and any external client still call them?
5. Are `piece` and `short_story` both real kinds? This spike keeps both as labels on the same shape (one prose text, collection or standalone). One kind would still meet "unattached pieces" and "short stories" only if the UI used two names for it.
