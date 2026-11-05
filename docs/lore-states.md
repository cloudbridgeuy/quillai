# Lore states over time (GUZ-1429 spike)

Docs only. No code, schema, or route changes ship with this note. Lines marked "guess" were not checked against code.

## 1. Problem and decision

A lore entry has one `visible_text` and one `hidden_notes` for the whole universe (`migrations/20261020000000_lore_entries.up.sql:1`). A character who is a squire in book one and a king in book three cannot be described correctly in both. Today the AI would be told one version everywhere.

Decision (owner approved, option A, universe timeline):

- Each universe has an ordered list of named timeline points ("Year 1", "After the siege").
- A lore entry may have named states, each with its own description and notes, each starting at a point.
- A piece (chapter, scene, group) sets a point or inherits it from its parent. No point anywhere means the base entry text, which is today's behaviour.
- The active state is the latest state starting at or before the piece's point. It feeds lore matching and AI injection. The context inspector shows which state applied.

Rejected:

- B, per-chapter state pick: one explicit choice per entry per chapter. It does not scale; 200 entries times 40 chapters is 8,000 choices and every new chapter starts blank.
- C, manuscript order: states follow position in the manuscript. It breaks on flashbacks, and order across books in one universe is undefined (containers and loose pieces have only per-sibling `position`, `docs/story-model.md:40`).

## 2. Data model sketch

Names are proposals. Follow the existing style: `INTEGER PRIMARY KEY`, nullable `deleted_at TEXT`, `ORDER BY position, id` with non-unique `position` (`docs/story-model.md:40`).

- `timeline_points`: `id`, `universe_id NOT NULL REFERENCES universes(id) ON DELETE CASCADE`, `name` nonblank, `position INTEGER NOT NULL`, `deleted_at TEXT NULL`. Index `(universe_id, position)`.
- `lore_entry_states`: `id`, `lore_entry_id NOT NULL REFERENCES lore_entries(id) ON DELETE CASCADE`, `name` nonblank, `point_id NOT NULL REFERENCES timeline_points(id) ON DELETE CASCADE`, `visible_text TEXT NOT NULL DEFAULT ''`, `hidden_notes TEXT NOT NULL DEFAULT ''`, `deleted_at TEXT NULL`. Same universe is a trigger check (entry and point), like the same-universe triggers in `docs/story-model.md:74`. Two states of one entry on the same point: reject with a unique index on `(lore_entry_id, point_id)` where `deleted_at IS NULL` (guess that SQLite partial unique index fits the existing style).
- `pieces.timeline_point_id INTEGER NULL REFERENCES timeline_points(id) ON DELETE SET NULL`. Null means inherit. Pieces table is described at `docs/story-model.md:60`.

Soft delete follows the lore rule: deleting sets `deleted_at`, restoring clears it, reads filter `deleted_at IS NULL` (`crates/api/src/manuscripts/lore/store.rs:12`, delete at `:92`, restore at `:110`). A state is also hidden when its entry is deleted. Permanent delete keeps the foreign-key cascades.

Deleting a point (soft): the resolver treats it as absent. A piece pointing at it falls back to its parent's point, then to none. States starting at it are skipped, not moved. Restore brings everything back unchanged. Permanent delete of a point cascades its states and nulls piece references. The API should show the writer the count of states and pieces affected before deleting (UI detail, guess).

Reordering points changes only `position`. No state or piece row is rewritten; the active state is recomputed on the next read. This is the reason states reference a point and not a number.

## 3. Resolution rule (functional core)

Pieces nest one level only: a leaf has at most one group parent, and groups do not nest (`docs/story-model.md:39`). A screenplay is a group of leaves labelled `scene` (`docs/story-model.md:47`), so a scene inherits from the screenplay group the same way a chapter inherits from its novel. Containers hold placement only and are not part of inheritance (guess: the owner did not name them; see call c1).

```ts
type Point = { id: number; position: number; deletedAt: string | null };
type PieceRef = { timelinePointId: number | null; parent: PieceRef | null };
type LoreState = { pointId: number; visibleText: string; hiddenNotes: string };

// Effective point: own point if active, else parent's, else null.
function effectivePoint(piece: PieceRef, points: ReadonlyMap<number, Point>): Point | null;

// Latest state at or before the point; null when point is null or none qualifies.
function activeState(point: Point | null, states: readonly LoreState[], points: ReadonlyMap<number, Point>): LoreState | null;

// Entry as the matcher sees it: base fields with the active state laid over them.
function resolveLoreEntry(entry: LoreEntry, state: LoreState | null): ResolvedLoreEntry;
```

Rules: compare by `position`, ties broken by `id` (same order as lists). Deleted points and deleted states are ignored. A point with a null effective point yields the base entry. Nothing in these functions reads I/O; the shell loads points, states, and the piece chain.

## 4. Path to matching, AI request, and inspector

Current path, read-only:

- `app/src/manuscripts/use_lore.ts:22` lists lore for the universe with `client.list` (`lore_client.ts`); the API list is `crates/api/src/manuscripts/lore/store.rs:12`.
- `app/src/md_editor/components/Editor.tsx:443-444` takes `lore.entries` as `loreEntries`; they go to the request builder at `:798` and `:810` and to `buildContextInspector` at `:1329` and `:1366`.
- `app/src/md_editor/lore_match.ts:1-10` defines `LoreMatchEntry`; `matchLoreEntries` (`:16`) matches name, aliases, keywords, and spends budget on `name.length + visible_text.length` (`:40`).
- `app/src/md_editor/ai_contract.ts:124-131` filters `leave_out`, calls `matchLoreEntries`, and sends `{ name, kind, visible_text, level }`.
- `app/src/md_editor/context_inspector.ts:12` builds injected and skipped lists from the same entries; `Editor.tsx:1753-1764` renders them.

Proposed change: the entries reaching `Editor.tsx:444` are already resolved. The client loads points and states with the lore list (extend the list response or add routes, see ticket 2) and the editor maps `LoreEntry[]` to `LoreMatchEntry[]` with `resolveLoreEntry` before `:444`. `lore_match.ts` and `ai_contract.ts` stay unchanged under the default of call (a), because the resolved entry has the same shape. The resolved entry also carries `stateName: string | null`; `LoreMatch` and the inspector list pass it through (`lore_match.ts:12`, `:43-48`) and `Editor.tsx:1759` prints it next to the entry name. Doing resolution in the client keeps one code path for every editor surface; a server-side twin is only needed if another caller (the eval binary `crates/api/src/bin/eval.rs`, guess) injects lore.

The lore editor panel (`LorePanel`) gains a states list per entry and a timeline manager; a piece gets a point picker in its metadata. These are new UI and follow story-first (section 6).

## 5. Owner decisions (2026-11-05)

(a) Decided: a state replaces `visible_text` and `hidden_notes` only. Name, kind, aliases, keywords, pinned, and level stay on the base entry, so matching never changes by state. Rejected: a state may also override aliases, keywords, and level (more power, but more fields in every state and a harder inspector).

(b) Decided: matching does not change per state. Matching and the budget stay as today (`lore_match.ts:34-40`); only the text and its cost change, so this work is independent of GUZ-910 (plan for many lore matches, Backlog, `docs/e2e-coverage.md:65`). Rejected: per-state matching, which would overlap GUZ-910 on `matchLoreEntries` and force GUZ-910 first.

(c1) Decided: only pieces carry a point, with parent inheritance; there is no container or universe default, and a loose root with no point shows base text. Rejected: a container default point for books in one series (saves setting a point on every novel, but adds a second place a point can come from).

(c2) Decided: the point picker shows "No point (base text)" explicitly on a piece with no point and no parent point, and the inspector says "base" for every entry. Rejected: new pieces copy their previous sibling's point (easy to get wrong when pieces are reordered).

## 6. Proposed split of GUZ-1430

Each ticket is red first (failing unit or integration test before the code). UI tickets add stories before any screen uses the change (`docs/design-system.md:14`). Browser specs are written, not run (`CLAUDE.md`). Sizes are rough guesses.

1. Timeline points and piece point, API (about 350 lines). Migration, `timeline_points` routes under `/universes/{id}`, `pieces.timeline_point_id` on `PUT /pieces/{id}`, soft delete and restore, same-universe checks. Red: integration tests for universe scoping, 422 on a cross-universe point, delete and restore.
2. Lore entry states, API (about 300 lines). Migration, state routes under `/universes/{id}/lore/{loreId}`, list response including states, soft delete with the entry. Red: integration tests for unique point per entry and trigger checks.
3. Resolution core and wiring (about 250 lines). Pure `effectivePoint`, `activeState`, `resolveLoreEntry` in `app/src/md_editor/` with `bun test` tables (flashback, no point, deleted point, reorder, scene inherits from screenplay group); map in `Editor.tsx`; `stateName` through `lore_match.ts` and the inspector row. Red: the unit table first. Inspector text is browser-visible, so add an E2E spec.
4. Timeline and states UI (about 450 lines). Stories first for any new primitive (point list, state tab), then the timeline manager, per-entry states in `LorePanel`, and a point picker in piece metadata. Needs an E2E spec. Split in two if it passes 500 lines.

Tickets 1 and 2 can run in parallel; 3 needs both; 4 needs 1 and 2 and can overlap 3.
