# Flexible story model

Design spike, revised after owner decisions. Docs only. Base `cc9010a` (this doc rebased onto `3002c89`). It replaces the fixed series → book → chapter chain.

Current chain, all required: `series` (`migrations/20261019000000_series_books_chapters.up.sql:1`), `books.series_id NOT NULL` (`:9`), `chapters.book_id NOT NULL` (`:18`). A book cannot exist outside a series. That is the gap behind [GUZ-892](https://linear.app/guzmonne/issue/GUZ-892/standalone-stories-outside-books-and-series) (Backlog, child of [GUZ-885](https://linear.app/guzmonne/issue/GUZ-885/e12-navigation-landing-and-manuscript-management)). Do not implement GUZ-892 by making `books.series_id` nullable.

## Recommendation

A **piece** is the internal unit. Every text a writer produces is a leaf piece: one Delta and one revision chain. A group piece has no text; it only orders child pieces. **Label** is a catalogue name the writer picks (`piece`, `short_story`, `novella`, `novel`, `screenplay`, `tv_script`, `episode`, `prologue`, `chapter`, `epilogue`, `scene`). The label is not a different shape. `format` (`prose` or `script`) and leaf versus group are the shape.

A **universe** owns lore. Every piece and every container is in exactly one universe. A new piece sees all of that universe's lore. Moving a piece between containers, or between a container and loose storage, inside the same universe does not change lore. Cross-universe moves are out of scope: no route, and any `universeId` change is 422.

A **container** groups root pieces. Kind: `book_series`, `tv_series`, `collection`. A collection may hold any label, including a novel (omnibus or anthology). A book series holds labels `novel` and `novella`. A TV series holds `tv_script` and `episode`. Loose storage is a root piece with `container_id` null, still in the universe. Deleting a container deletes the root pieces in it (and their children), matching `ON DELETE CASCADE` today (`migrations/20261019000000_series_books_chapters.up.sql:9`). It does not delete the universe or its lore.

`format` defaults: `screenplay`, `tv_script`, and `episode` start as `script`; the writer may set an episode or TV script to `prose`. Other labels start as `prose`. Children use the parent's format. Script groups are one leaf per scene, drawn as one continuous document (§6). Prose groups open one child at a time, as chapters do now.

Migration: each existing series becomes one universe plus one `book_series` container. Its lore moves to the universe. Each book becomes a group piece labelled `novel`, format `prose`. Each chapter becomes a leaf child labelled with its current kind. Old `/series`, `/books`, `/chapters`, delta, and lore routes stay as a façade over this model with no removal ticket.

## 1. Types and schema

Rust and TypeScript share these sums. The label is a string on the piece, not a sum that picks a table. Triggers repeat the shape rules so a raw write cannot bypass them.

```text
Universe  { id, title, position }
Container { id, universe_id, title, position, kind: book_series | tv_series | collection }

Label = piece | short_story | novella | novel | screenplay | tv_script
      | episode | prologue | chapter | epilogue | scene
Format = prose | script
Shape  = leaf | group

Piece {
  id, universe_id, title, position, label, format, shape,
  container_id: Option<Id>,   // roots only
  parent_id: Option<Id>,      // leaves under a group only
}
```

A leaf owns `content`, `head_rev`, `snapshot_rev`, and `piece_revisions`. A group owns none of those. A group is always a root (`parent_id` null). A child is always a leaf, so groups do not nest. `container_id` is null on a child; placement belongs to the root. `position` stays a nonnegative integer and is not unique (`crates/api/src/manuscripts/series/models.rs:31`). Lists stay `ORDER BY position, id` (`series/handlers.rs:14`, `books/handlers.rs:22`, `chapters/handlers.rs:49`). Title stays nonblank (`series/models.rs:24`).

How label relates to shape and format: the database does not infer either from the label. The UI offers conventions, and container membership checks the label only when placing a root:

| Usual writing | Shape the UI creates | Format default | Where it sits |
| --- | --- | --- | --- |
| scrap, short story | leaf | prose | storage, or a collection |
| novel, novella | group of leaves the writer labels `prologue`, `chapter`, `epilogue` | prose | book series or collection; a single-text novel is a leaf labelled `novel` until it is split |
| screenplay | group of leaves labelled `scene` | script | storage or a collection |
| TV episode | leaf, or a group when it has scenes or chapters | script, writer may choose prose | TV series or storage |

Relabelling does not move children, rewrite deltas, or change `format`. It re-checks container membership and 422s when a book series or TV series would no longer allow that root. A collection never fails that check. A leaf labelled `novel` may sit in a book series or a collection. A group labelled `short_story` may sit in a collection or in storage, and 422s in a book series.

### Tables

New migration after `migrations/20261020000000_lore_entries.up.sql`. `users` and `author_guidance` stay. Rebuild, because SQLite cannot retarget `lore_entries.series_id` or drop `NOT NULL` on `books.series_id` in place.

`universes` — `id INTEGER PRIMARY KEY`, `title TEXT NOT NULL`, `position INTEGER NOT NULL`.

`containers` — `id`, `universe_id INTEGER NOT NULL REFERENCES universes(id) ON DELETE CASCADE`, `title`, `position`, `kind` checked against `book_series | tv_series | collection`. Index `(universe_id, position)`.

`pieces` — `id`, `universe_id NOT NULL REFERENCES universes(id) ON DELETE CASCADE`, `container_id NULL REFERENCES containers(id) ON DELETE CASCADE`, `parent_id NULL REFERENCES pieces(id) ON DELETE CASCADE`, `title`, `position`, `label` checked against `Label`, `format` checked against `prose | script`, `shape` checked against `leaf | group`, `content TEXT NULL CHECK (content IS NULL OR json_valid(content))`, `head_rev` and `snapshot_rev` nullable with the numeric checks from `migrations/20261019000001_chapter_revisions.up.sql:1` when set, `legacy_book_id INTEGER NULL UNIQUE`, `legacy_chapter_id INTEGER NULL UNIQUE`.

Shape check on the row:

- `group`: `parent_id`, `content`, `head_rev`, and `snapshot_rev` are null. `legacy_chapter_id` is null.
- `leaf`: `content`, `head_rev`, and `snapshot_rev` are not null, `snapshot_rev <= head_rev`, and `legacy_book_id` is null.
- `parent_id` and `container_id` are not both set.

`piece_revisions` — `piece_id`, `rev`, `delta`, `created_at`, same checks as `chapter_revisions` (`:4`), `UNIQUE(piece_id, rev)`, `ON DELETE CASCADE`.

`lore_entries` — rebuild with `universe_id INTEGER NOT NULL REFERENCES universes(id) ON DELETE CASCADE` instead of `series_id`. Other columns, checks, and defaults stay (`migrations/20261020000000_lore_entries.up.sql:1`). Index `universe_id`. Order stays `name, id` (`lore/handlers.rs:34`).

Triggers:

- Parent is a group in the same universe. Child `format` equals parent `format`. No grandchildren.
- `container.universe_id` equals the piece's universe.
- Book series roots are labelled `novel` or `novella`. TV series roots are labelled `tv_script` or `episode`. Collection roots are unrestricted.
- A group has no `piece_revisions`. A leaf's revisions reference that leaf only.
- `legacy_book_id` is only on groups. `legacy_chapter_id` is only on leaves.

Book ids and chapter ids are separate sequences today and can collide, so they cannot both become `pieces.id`. The façade keeps the old numbers in the legacy columns. `pieces.id` is new and is the revision key.

### Migration from current rows

One transaction.

| From | To |
| --- | --- |
| each `series` | one `universes` row (new id, same title and position) and one `containers` row (`id` = series id, that `universe_id`, kind `book_series`, same title and position) |
| `lore_entries.series_id` | `universe_id` of that series' new universe, same lore id and fields |
| each `books` row | a group piece, new id, `legacy_book_id` = book id, label `novel`, format `prose`, container = the series container, universe = that universe |
| each `chapters` row | a leaf, new id, `legacy_chapter_id` = chapter id, `parent_id` = the book's piece, label = the chapter kind (`chapter`, `prologue`, or `epilogue`, `chapters/models.rs:154`), format `prose`, `content` / `head_rev` / `snapshot_rev` copied |
| `chapter_revisions` | `piece_revisions` on that leaf, same `rev`, `delta`, `created_at` |

Empty books stay groups with zero children (`ManuscriptTree.tsx:120` shows "No chapters yet."). Update `sqlite_sequence`. Then drop `chapter_revisions`, `chapters`, `books`, `series`, and the old `lore_entries`.

No data loss: every Delta, revision, position, title, chapter kind, and lore field is copied. Meaning is unchanged for current rows because each series is its own universe of one book series of novels. The migration does not guess novella, short story, or script.

### Down migration

Older down scripts still expect the pre-migration schema. `20261019000001_chapter_revisions.down.sql` alters `chapters`. `20261020000000_lore_entries.down.sql` drops `lore_entries`. This down restores that shape first.

Abort with no writes when a row cannot be represented: a universe with anything other than one `book_series` container and the pieces under it; a container that is not `book_series`; a loose piece; a group whose label is not `novel` or whose format is not `prose`; a leaf that is not a child, or whose label is outside `prologue` / `chapter` / `epilogue`, or whose format is not `prose`; a missing legacy id; lore whose universe is not that one universe. SQLite has no top-level `RAISE`. Use a guard table and a `BEFORE INSERT` trigger that `RAISE(ABORT, …)` when such a row exists, then insert one guard row.

When the guard passes: recreate `series` from containers (same ids), `books` from groups (`id` = `legacy_book_id`), `chapters` from leaves (`id` = `legacy_chapter_id`, content and heads from the leaf), `chapter_revisions` from `piece_revisions`, and `lore_entries.series_id` = the container id. Then drop the new tables.

## 2. API

New handlers read the new tables. Old handlers stay mounted (`crates/api/src/manuscripts/mod.rs:10`) and read the same tables. No dual-write. The old routes are not deprecated and have no removal ticket.

### New routes

| Method and path | Body / result |
| --- | --- |
| `GET/POST /universes`, `GET/PUT/DELETE /universes/{id}` | title and position, same rules as series |
| `GET/POST /universes/{id}/lore`, `GET/PUT/DELETE /universes/{id}/lore/{loreId}` | the only lore routes that write. Same fields as today |
| `GET/POST /universes/{id}/containers` | containers in that universe |
| `GET/PUT/DELETE /containers/{id}` | kind is fixed after create |
| `GET /universes/{id}/pieces?container=none` | loose storage in that universe |
| `GET/POST /containers/{id}/pieces` | root pieces. `POST` checks membership |
| `POST /universes/{id}/pieces` | a root in storage. A leaf inserts an empty Delta. A group does not insert a child |
| `GET/PUT/DELETE /pieces/{id}` | `PUT` may change title, position, label, format. Format on a group updates its children in the same transaction. `universeId` on `PUT` is 422 |
| `POST /pieces/{id}/placement` | `{ "containerId": number \| null, "position": number }`. Root only. Container must share the piece's universe |
| `POST /pieces/{id}/split` | leaf root becomes the first child of a new group. The leaf's id, Delta, and revisions stay. The group copies universe, container, position, format, and label. The child's label defaults to `chapter` (prose) or `scene` (script); the body may set it |
| `POST /pieces/{groupId}/attach` | `{ "pieceId", "position" }`. Source is a root leaf in the same universe. Sets `parent_id`, clears `container_id`. Revisions stay on `pieceId` |
| `POST /pieces/{groupId}/children` | a new empty leaf under that group |
| `GET/PUT/DELETE /pieces/{id}` for a child | metadata. `content` on `PUT` is 422, as `UpdateChapter` rejects `content` (`chapters/models.rs:170`) |
| `GET /pieces/{id}` | leaf: rebuilt head and `head_rev`. Group: children, no Delta |
| `POST/GET /pieces/{id}/deltas` | leaves only. Same JSON as today: `{ baseRev, delta }` → `{ rev, delta }`, and `[{ rev, delta }]` |

Parent scoping stays as strict as `sync.rs:25`. Validation stays 422. 409 stays reserved for revision history (`IncompleteHistory` in `chapter_revisions.rs:17`, future or gapped `since`). Attach and placement of a piece from another universe are 422, not a move.

Delta sync for a leaf is `RevisionStore::append` on `piece_id`: expected head, `rev = head + 1`, server-priority transform (`sync.rs:58`), snapshot every 100 (`sync.rs:55`, `chapter_revisions.rs:104`). `rebuild` is still snapshot composed with later deltas (`chapter_revisions.rs:38`). The client sync core already speaks `baseRev` and one document (`app/src/md_editor/chapter_sync.ts:41`). One leaf uses that core unchanged. A script group does not (§6). The browser URL keeps `seriesId`, `bookId`, and `chapterId` (`chapter_session.ts:25`) as a façade load: container, `legacy_book_id`, `legacy_chapter_id`, then the leaf. A loose piece or a scene uses `pieceId`.

`docs/eval.md` exports `GET /series/{id}/books/{book_id}/chapters?content=true` (`docs/eval.md:8`). It keeps using the façade. There is no ticket to retarget it.

### Compatibility façade

Kept indefinitely. Old JSON stays the fields current parsers read (`app/src/manuscripts/contract.ts:1`).

| Current route | Behavior |
| --- | --- |
| `/series` (`series/mod.rs:10`) | `book_series` containers only, `id` = container id. Other kinds are absent; `GET` by id is 404. `POST` creates a universe and a `book_series` in one transaction, same title and position. `DELETE` deletes the container and its pieces, then deletes the universe when it has no containers and no root pieces left, which removes that universe's lore. On today's one-series databases that matches lore disappearing with the series |
| `/series/{id}/books` (`books/mod.rs:10`) | group pieces in that container with label `novel` and format `prose`. JSON `id` is `legacy_book_id`, `series_id` is the container id. `POST` creates that group. A novella, a leaf novel, or a novel in a collection is not on this list |
| `…/chapters` (`chapters/mod.rs:11`) | leaves under that group. JSON `id` is `legacy_chapter_id`, `kind` is the label, `book_id` is `legacy_book_id`. Content is the leaf, rebuilt as `Chapter` (`chapters/models.rs:28`) |
| `…/chapters/{chapter_id}/deltas` (`chapters/mod.rs:15`) | resolve `legacy_chapter_id`, then the leaf sync above |
| `/series/{id}/lore` (`lore/mod.rs:10`) | lore for the container's universe. `POST` inserts there. Two book series in one universe share one lore list on both series routes. `POST /series` still isolates lore, because it creates a new universe |

`e2e/chapter-sync.spec.ts` and `e2e/manuscript-create.spec.ts:5` stay the façade proof. New specs go on `testMatch` in `e2e/playwright.config.ts:5`.

## 3. Moves

Positions are not renumbered. The caller sends `position` (≥ 0). Ties stay `position, id`.

**Placement** changes a root's `container_id` and `position` inside its universe. Storage is `containerId: null`. Membership is checked. Children, deltas, revision rows, and lore are not written. A client posting to that leaf keeps its base revision.

**Attach** is how a loose leaf joins a novel, novella, or screenplay. The source stays the revision key. 422 when the source is a group, has a parent, or is in another universe. Detach is placement of a child back to storage: clear `parent_id`, set `container_id` null, same universe. The leaf's label is left as the writer set it.

**Split** is how a single-text piece gains a first section without copying the Delta. The new group takes the catalogue slot. The leaf id remains the sync key. An open editor keeps posting to that id; the tree shows the new group as the parent.

**Cross-universe** placement, attach, split, and `PUT` of `universeId` are 422. Not designed here.

Delete of a group deletes its leaves and their revisions. Delete of one leaf does not delete universe lore. Container-owned lore does not exist.

## 4. Lore and the AI request

Lore rows point at `universe_id` only. `matchLoreEntries` (`app/src/md_editor/lore_match.ts:16`) takes a string and a list. Pins, keyword edges, NFC, the UTF-16 budget, and skip-when-over-budget stay. `buildAiRequest` still drops `leave_out` and sends `{ name, kind, visible_text, level }` (`ai_contract.ts:102`, `ai_contract.ts:122`). The server rejects unknown fields (`crates/api/src/ai/lore.rs:8`) and prints `<lore>` from that array (`crates/api/src/ai/prompt.rs:28`). It does not load lore from SQLite. Hidden notes stay off the request (`lore_client.ts:6` holds them; `AiLoreEntry` does not).

What changes is which list is loaded. Today `useLore(chapterSync.scope?.seriesId)` (`Editor.tsx:338`) and `LorePanel` (`Editor.tsx:1252`) call `/series/{id}/lore`. `use_lore.ts:10` clears entries when that id changes. After the change, the open piece resolves `universe_id`, and the panel loads `/universes/{id}/lore`. Placement and attach inside the universe pass the same id, so the list stays. A different universe is a different list. Do not union lists. `LoreEntry.series_id` (`lore_client.ts:6`) becomes `universe_id` on the new client. The façade parser can keep reading `series_id` from the old route. Request JSON gains no scope field.

## 5. UI

The drawer is series, then books, then chapters, and only a chapter is selectable (`ManuscriptTree.tsx:122`). Empty copy is "No series yet." (`:112`). Opening a chapter still saves first (`:109`, `docs/editor-design.md:82`). Those rules stay for prose leaves.

[GUZ-891](https://linear.app/guzmonne/issue/GUZ-891/create-chapters-books-and-series-from-the-app) is on main at `3002c89`. The tree posts series, book, and chapter through the old client (`ManuscriptTree.tsx:35`). `docs/e2e-coverage.md:46` still lists that ticket as Backlog. The empty-state "Create chapter" button still bootstraps "My Series" / "My Book" / "Chapter One" (`Editor.tsx:1264`, `chapter_session.ts:12`). The façade keeps those calls working. The create ticket below replaces the buttons; it does not add a second standalone path beside them.

New tree, for the open piece's universe, ordered by `position, id`. Keys are prefixed so a container id cannot collide with a piece id.

- Each container is a disclosure labelled book series, TV series, or collection. Children are its root pieces.
- A group is a disclosure of its leaves. Empty copy follows format: "No chapters yet." or "No scenes yet."
- A leaf is a `ListButton` that opens that piece. A prose group opens one leaf. A script group opens the continuous view (§6), which is not part of the tree ticket.
- Loose pieces are roots beside containers, not a fake series. That is the GUZ-892 tree.

Create uses `Button`, `ListButton`, and `TextField` (`docs/design-system.md:7`). There is still no `Select` primitive; label and format are a short button list until one exists under `app/src/ui/`.

- New universe: title.
- New container: title and kind, in the current universe.
- New root piece: title, label, format, optional container when membership allows.
- New child: on a group, title and label. On a leaf, split first.
- Placement and attach: actions on a root, calling §3. They do not offer another universe.

[GUZ-895](https://linear.app/guzmonne/issue/GUZ-895/rename-series-books-and-chapters-optional-chapter-numbers) (Backlog) is title and position on universe, container, and piece, plus the label. Optional chapter numbers stay a display of child `position` for label `chapter`. No number column.

[GUZ-894](https://linear.app/guzmonne/issue/GUZ-894/landing-page) (Backlog): with no chapter, bootstrap still runs or `loadEditorChapter` throws `NoEditorChapter` (`chapter_session.ts:44`). The landing page lists universes, and inside one universe the same roots as the tree (containers and loose pieces), plus recent leaves by `piece_revisions.created_at`. Resume opens that leaf, or the script group when the leaf's parent is format `script`. Create from the landing page uses the same API, so a piece does not require the editor (GUZ-892). The left-drawer lore in `docs/editor-design.md:109` follows the open piece's universe. Export stays one leaf at a time until the script view exists.

## 6. Script reading and formatting

A screenplay or script episode is a group. Each scene is a leaf with its own text and revision chain. The reading surface shows those leaves as one continuous document in `position, id` order. Prose groups do not use this surface. Schema and façade work do not build it.

The editor today is one Quill (`Editor.tsx:204`) and one in-flight chapter delta (`chapter_sync.ts:41`). A continuous script surface has to sit on top of N chains. That is a design risk:

- The caret is `{ pieceId, index }` inside one leaf, in UTF-16 units, plus a scroll offset equal to the lengths of previous siblings. A scene break between leaves is not stored in either Delta. Treat it as a non-editable virtual index of length 1 so the caret can sit on the boundary without a retain in a piece.
- A selection inside one scene uses the existing selection memory and server-priority transform for that piece only.
- A selection that starts in one scene and ends in another is two endpoints, not one `ChapterScope` range (`chapter_session.ts:25`). The edit becomes one delta per touched leaf, posted separately. There is no server transaction across those posts. A failure after the first post leaves a partial edit. The view must not send one `baseRev` for the whole scroll.
- A remote insert in an earlier scene changes the scroll index of a caret in a later scene and does not change the later piece's `baseRev`. Recompute virtual positions from sibling heads. Do not transform the later piece's pending delta because of that insert.
- Reload loads the group and each leaf's head, then maps the saved caret back to `{ pieceId, index }`. Concatenating scenes into one Quill Delta and one revision chain would throw away per-scene sync. Do not do that.

Block types (scene heading, action, character, dialogue, parenthetical, transition), Fountain, and FDX stay a later phase, after that surface. Store each block as a Quill block attribute on the newline. `quill-delta` and `crates/delta` already compose and transform attributes. Do not add an op kind or an embed. `buildAiRequest` rejects object inserts (`ai_contract.ts:154`); attributes on text inserts still pass. Markdown shortcuts (`markdown.ts:3`, `markdown.ts:21`) turn off when `format` is script. Export walks the group's leaves in order and writes one Fountain or FDX file. Import waits.

## 7. Tickets, risks, open questions

Ordered. Browser-visible tickets include an E2E spec and a `testMatch` entry. GUZ-892 is tickets 4, 5, and 7 together. No route-removal ticket.

| # | Ticket | Proof |
| --- | --- | --- |
| 1 | Migration up/down: one universe and one book series per series, lore on the universe, legacy book and chapter ids, guard refuses a loose piece | `crates/api/tests/story_model_migration_test.rs` (not browser-visible) |
| 2 | New routes and the permanent façade. Old JSON unchanged, including shared universe lore when two book series share a universe | Existing `crates/api/tests/*_test.rs`, `e2e/chapter-sync.spec.ts`, `e2e/manuscript-create.spec.ts:5` |
| 3 | Placement, attach, split, detach. Piece id and revisions stable. Cross-universe 422. Lore rows unchanged | Rust HTTP tests |
| 4 | Tree lists containers and loose pieces in the open universe; opening a loose piece loads its text | `e2e/story-tree.spec.ts`. API-seed a book series novel and a loose piece in one universe; both visible; open the piece; reload; server text matches. A chapter of the novel still opens |
| 5 | Create universe, container, root piece, and child, replacing the GUZ-891 controls. A collection accepts a novel | `e2e/create-work.spec.ts`. Create a loose piece, a novel in a new book series, a novel in a collection, and a TV series with a script episode; reload shows them |
| 6 | Lore panel and Continue/Edit load the piece's universe | `e2e/lore-scope.spec.ts`. Two pieces in one universe send the same lore and omit hidden notes. Placement into another container in that universe does not change the Continue lore list |
| 7 | Landing (GUZ-894) on universes, containers, and loose pieces, including create without the editor | `e2e/landing.spec.ts`. Fresh app shows the landing list. Resume opens the leaf. Create a piece in a universe from the landing page |
| 8 | Continuous script surface over one leaf per scene, with the caret and sync rules in §6 | `e2e/screenplay-scroll.spec.ts`. Two scenes read as one scroll. An edit in the second scene syncs only that piece. A remote edit in the first scene leaves the second scene's text in place and keeps the caret on that text |
| 9 | Script block formats, markdown shortcuts off, Fountain and FDX export | `e2e/screenplay-format.spec.ts`. Scene heading and dialogue survive reload. Downloaded Fountain contains those lines. Depends on ticket 8 |

GUZ-895 can ship against the façade before these tickets.

### Risks

- GUZ-891 posts the old routes (`ManuscriptTree.tsx:35`). Ticket 5 replaces those controls. Until then the façade is what keeps create, sync, and `e2e/manuscript-create.spec.ts:5` green.
- `POST /series` creates a universe. Two containers placed in one universe share lore on both `/series/{id}/lore` routes. Tests that create a series per case still see isolated lore.
- Façade `DELETE /series` deletes the universe only when nothing else remains in it. A shared universe keeps its lore after one book series is deleted.
- Down migration aborts once a loose piece, a script, a collection, or a second container exists. Do not delete those rows to force the down through.
- `pieces.id` is not the book id or the chapter id. Old URLs work only through `legacy_book_id` and `legacy_chapter_id`. Dropping either column breaks the façade.
- Split introduces a new group id while the leaf id stays the sync key. The tree has to retarget its parent pointer. The open delta POST still uses the leaf.
- Attach clears `container_id` before the parent link is visible to a concurrent delete, in one transaction, or the leaf can cascade with the old container.
- Ticket 8 must not concatenate scene deltas into one chain. Cross-scene edits are partial if the second POST fails. The virtual scene break is not a stored character; counting it as text would shift every later index.
- Screenplay embeds would break the text-only AI slice and the Delta parity rule. Ticket 9 is attributes only.
- New Rust files stay at or below 1000 lines (`CLAUDE.md`). New route modules, not more cases in `chapters/handlers.rs`.
- `Editor.tsx:1264` is a raw `<button>`. New create controls do not add another (`docs/design-system.md:7`).

### Open questions

None. Collections hold any label, including novels. Lore is universe-scoped, and cross-universe moves are out of scope. Screenplays are one leaf per scene, shown as one document, with formatting later. The old manuscript and lore routes stay. Piece is the stored unit; label is catalogue only.
