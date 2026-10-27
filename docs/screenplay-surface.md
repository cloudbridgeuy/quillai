# Continuous script surface

Design spike for [GUZ-943](https://linear.app/guzmonne/issue/GUZ-943). Docs only. Base `83273b8`. It expands `docs/story-model.md` §6 and ticket 8 of §7. It does not change them.

Ticket rules, kept as given:

- One leaf per scene. Each scene keeps its own Delta, revision chain, and `/pieces/{id}/deltas` sync.
- Do not concatenate scene Deltas into one chain or send one `baseRev` for the whole scroll.
- The scene break is virtual. It is never a stored character.
- Proof: `e2e/screenplay-scroll.spec.ts`. Two scenes read as one scroll. An edit in scene 2 syncs only that piece. A remote edit in scene 1 keeps scene 2 text and the caret in place.

Owner decisions (2026-10-26), referenced below as "decision N":

1. Opening a script: a new group route returns the script group's child scenes with `content` and `headRev` in one GET. No N+1.
2. Polling: a new group delta poll route returns missed revisions for all scenes of the group in one request, keyed by scene id, each with its own `since`. No round-robin.
3. Scene break: a thin rule with the scene title.
4. New scene is in this epic and reuses `POST /pieces/{id}/children`. Merge scenes is out of this epic (later ticket).
5. Cross-scene delete: blocked in v1.
6. Continue pauses sync of all scenes while it runs: accepted for v1.
7. Below 1100px: the same combined surface. No separate one-scene mode.

## Recommendation

**One Quill with a composed view and an offset map (option B).** Each scene keeps its own `createChapterSession`. A pure layout module maps the composed Quill document to `{ pieceId, index }` and back. The break between scenes is one Quill line that the client owns and never posts.

Reason: Continue, Edit, the Continue window, the lore window, tint, undo, and selection memory all assume one Quill (`Editor.tsx:206`, `ContinueWindow.tsx:7`, `LoreWindow.tsx:19`, `continue_tint.ts:5`). Option B keeps that assumption. The new code sits in the sync layer, which is already per document and pure at its core (`chapter_sync.ts:5`).

Split GUZ-943 into its own epic: **yes**. See §5.

## 0. Current code, as measured

| Fact | Evidence |
| --- | --- |
| The editor makes one Quill, history `userOnly` | `Editor.tsx:206-209` |
| Markdown shortcuts run on every text change | `Editor.tsx:93`, `Editor.tsx:220` |
| One sync hook owns one session, one scope, one recovery key | `use_chapter_sync.ts:11`, `:25`, `:58`, `:77` |
| Open a document: `setContents`, clear history, start a session | `use_chapter_sync.ts:67-72` |
| Remote change: diff Quill against the session document, apply, transform the selection with `transformPosition`, clamp to surrogate boundaries | `use_chapter_sync.ts:85-95` |
| One poll per second for the one session; skipped while AI runs or a switch is open | `use_chapter_sync.ts:145-147` |
| Every user change goes to the session as the full Quill contents | `use_chapter_sync.ts:221-224`, `Editor.tsx:1318` |
| Sync state is `{ rev, server, flight, pending }`; one in-flight Delta | `chapter_sync.ts:5-10`, `:37-43` |
| Remote revisions transform flight and pending with server priority | `chapter_sync.ts:60-66` |
| Session API: `edit`, `tick`, `flush`, `reserveAi`, `canStartAi`, `stop` | `chapter_session.ts:187-237` |
| A group cannot open in the editor | `chapter_session.ts:17` |
| A group GET returns child summaries, no content and no `headRev` | `crates/api/src/manuscripts/pieces/handlers.rs:38-49`, `contract.ts:49-51` |
| A new child leaf starts with the empty Delta at rev 0 | `pieces/handlers.rs:186` |
| Concurrent Continue transforms its anchor, end, and owned ranges on every non-AI text change | `Editor.tsx:383-389`, `continue_stream.ts:25-32` |
| Continue anchors at the next newline after the caret | `continue_stream.ts:19-23` |
| The AI request slice is `quill.getContents(start, length)` | `Editor.tsx:604-622` |
| AI slices reject object inserts (embeds) | `ai_contract.ts:158` |
| Tint uses CSS highlights on one Quill root | `continue_tint.ts:5-18` |
| The tree already shows "No scenes yet." for script groups | `ManuscriptTree.tsx:218` |
| `Editor.tsx` refers to `quillRef`/`markerQuill` 25 times | `grep -c` |

## 1. Rendering options

**A. One Quill per scene, stacked.** Each scene is its own Quill in one scroll container. Each Quill owns one session.

**B. One Quill, composed view, offset map.** The client builds one Delta: scene 1, a break line, scene 2, and so on. A pure module keeps `layout = [{ pieceId, start, length }]`. User changes are split by the layout into one Delta per scene. Remote changes are shifted by the scene's `start` and applied to the one Quill.

**C. Read-only scroll, edit one scene at a time.** Other scenes render as static HTML. Clicking a scene swaps the live Quill into it. Small change, but the caret cannot move across a break and selection cannot span scenes. It fails "read as one document" for writing flow. Not recommended past a prototype.

| Concern | A: Quill per scene | B: one Quill, offset map |
| --- | --- | --- |
| Caret across scenes | Custom ArrowUp/Down/Left/Right bindings move focus between Quills and guess the column | Native. Bindings only skip the break line |
| Cross-scene selection and copy | Browser selection spans two `contenteditable` roots; each Quill sees only its part; copy is plain DOM text | Native Quill selection and clipboard |
| Delete across a break | Not possible without custom code in both Quills | Possible to detect; v1 blocks it (§3) |
| Undo | One stack per scene. Cmd+Z in scene 2 does not undo the last edit in scene 1 | One stack. `userOnly` already ignores remote api changes (`Editor.tsx:208`) |
| Continue/Edit AI | `buildRequestPreview` must read N Quills and stitch text; stream code targets one Quill | Unchanged slice code (`Editor.tsx:617-622`). The break line is plain `\n`; it reads as a blank line to the model |
| Concurrent Continue + remote edit | Remote edit in another scene is in another Quill; no transform needed, but the window bracket spans Quills | Existing transform path (`Editor.tsx:383-389`) already handles any non-AI change |
| Continue window and lore window | Both take one `quill` and use `getBounds`/`getLeaf` (`ContinueWindow.tsx:12-13`, `LoreWindow.tsx:23`). Need a multi-Quill rewrite | Unchanged |
| Tint | One highlight set per Quill root | Unchanged |
| Markdown shortcuts | Per Quill | One Quill; off for script (`Editor.tsx:220`) |
| Change to `Editor.tsx` | Large. 25 `quillRef`/`markerQuill` sites assume one Quill | Small. Choose the sync hook by format; guard break edits; status per scene |
| Change to sync | None to `chapter_sync.ts`; hook runs N sessions | None to `chapter_sync.ts`; hook runs N sessions plus the layout map |
| Remote edit in scene 1, caret in scene 2 | Trivially stable: different Quill | `retain(start₁) + remote` shifts the caret by `transformPosition`, as today (`use_chapter_sync.ts:92-93`) |
| Main risk | AI, windows, and undo fork into multi-Quill code | Quill can merge or delete the break line; the guard must catch every path (keys, paste, IME, `deleteText` by AI) |

Recommend **B**. A moves the cost into every AI feature. B puts the cost in one pure module and one guard.

### Break line in option B

- One `\n` with a client-only line format, for example `scene-break: <pieceId>`, registered as a block blot with `contenteditable=false`. It draws a thin rule with the scene title (decision 3). The title is read-only text from the leaf's `title`, drawn by the blot (CSS or a non-text node), not text in the Delta, so it does not change the length of 1.
- Length 1 in the composed index, as §6 says. It is a text insert, not an embed, so `ai_contract.ts:158` still passes.
- It is never in a posted Delta. The split step drops it. A posted Delta with that attribute is a programming error: throw before `postDelta`.
- Each scene keeps its own trailing `\n`. A new leaf has empty content (`pieces/handlers.rs:186`); the composed view gives it `\n`. Today an opened empty leaf also posts `\n` on first edit diff (`use_chapter_sync.ts:112` with `chapter_sync.ts:29-35`, inferred). Keep that behaviour; do not special-case it.

## 2. Sync under option B

New pure module `app/src/md_editor/script_layout.ts`:

- `composeScenes(scenes) → { document, layout }`. Order is `position, id`, as the tree.
- `splitChange(change, layout) → { kind: "scenes", changes: Map<pieceId, Delta> } | { kind: "crossesBreak" }`. A change that deletes or formats a break line, or inserts inside one, is `crossesBreak`.
- `placeRemote(pieceId, sceneChange, layout) → composed change` = `retain(start) + sceneChange`, then a new layout with that scene's length updated.
- `toScenePosition(index, layout) → { pieceId, index } | { break: pieceId }` and the inverse. Used for reload caret, recovery, and Resume.

New hook `use_script_sync.ts` beside `use_chapter_sync.ts`, same return shape so `Editor.tsx` can pick one by format:

- **Load.** One GET on a new group route returns the script group and every child scene with `content` and `headRev` (decision 1; API in §5 ticket 2). No N+1. Start one `createChapterSession` per scene with that scene's `PieceScope`. `chapter_session.ts` is reused unchanged.
- **Lazy load.** Not in v1. All scenes load in that one GET before the editor turns ready.
- **Local edit.** On `documentChanged`, split the Quill change by layout. Each touched scene gets `session.edit(sceneDocument)` with that scene's slice of the composed document. Untouched scenes get no call, so they post nothing. This is the "edit in scene 2 syncs only that piece" proof.
- **Remote edit.** Each session's `publish` callback already diffs and applies (`use_chapter_sync.ts:85-95`). In the script hook the diff is against the scene slice, then `placeRemote` shifts it to the composed index. The selection transform stays `change.transformPosition(index, true)`. An insert of k units in scene 1 moves a scene-2 caret by k, so it stays on the same text. Scene 2's session state is not touched, as §6 requires.
- **Polls.** Today one GET per second (`use_chapter_sync.ts:145`). The script hook keeps one timer and sends one group delta poll per tick (decision 2): the request carries each scene's own `since`, and the response holds missed revisions keyed by scene id. No round-robin. `createChapterSession` already takes its client as `Pick<Client, "postDelta" | "getDeltas">` (`chapter_session.ts:69`), so the hook passes an adapter whose `getDeltas` is answered from that tick's group response. Posts stay per scene on `/pieces/{id}/deltas`.
- **AI.** Today every tick stops while AI runs (`use_chapter_sync.ts:146`) and Continue reserves the one session (`Editor.tsx:662`). v1 keeps that (decision 6): Continue reserves every scene session, so all must be clean, and the group poll pauses until Stop or Finish. Edit needs all clean too. Remote edits arrive after Stop or Finish, as in `stream-remote-edit.spec.ts` today.
- **Status.** One header line. "Saved" only when all sessions are clean. Any blocked session shows the "Not saved" pill with that scene's title in the reason. The cross-break notice is its own Alert pill under the top bar; the header line does not repeat it.
- **Recovery.** Today one key holds `{ scope, document }` (`use_chapter_sync.ts:77`). The script hook writes one entry per dirty scene, keyed by leaf scope, so export and recovery stay per piece.
- **Leave and switch.** `flush` every session in order. The first failure stops the switch and keeps the surface, as for chapters today (`use_chapter_sync.ts:121-131`).
- **URL and Resume.** The surface opens with `?pieceId=<groupId>` and the caret scene in a second parameter, for example `scenePieceId`. `loadPiece` stops throwing for a group whose format is `script` (`chapter_session.ts:17`); a prose group still throws. Landing Resume stores the leaf; it opens the parent group when the parent is script (`docs/story-model.md` §5).

## 3. Cross-scene edits

Allowed in v1:

- Caret moves across a break with arrows, Home/End, PageUp/PageDown, and click. Bindings step over the break line so the caret never rests on it.
- Selection across scenes, and copy of that selection. The copied text has a blank line at each break.
- Continue anywhere. The window may hold text from earlier scenes (read-only context). The stream anchors in the caret's scene; `createContinueStream` finds the scene's own trailing `\n` first (`continue_stream.ts:20-21`), so AI text stays in that scene.
- Edit when the selection is in one scene.

Blocked in v1, with a short status message, and no change sent:

- Delete, cut, type over, or paste over a selection that crosses a break.
- Backspace at the start of a scene, and Delete at the end of a scene (they would merge two scenes).
- Edit with a selection that crosses a break. The pink note's Apply is disabled with a reason.
- Format changes across a break (toolbar). Format each scene alone.

The guard has two layers: keyboard bindings stop the common keys, and `splitChange` returning `crossesBreak` reverts any change that slipped through (paste, drop, IME, `deleteText`). The revert is a `silent` inverse; nothing is posted.

Enter at a scene end inserts a newline in that scene, as in prose. It does not make a new scene.

In this epic (decision 4): **New scene** adds an empty scene after the caret's scene through the existing `POST /pieces/{id}/children` (`pieces/mod.rs:21`). No text moves, so a failure loses no text. Later siblings need a new `position`; that is one `PUT /pieces/{id}` per later scene, not atomic. Shipped (GUZ-986): later scenes are renumbered first, last to first, to `max(position, caret position + 1 + k) + 1`; then the scene is created at `caret position + 1`, so no step reorders scenes and a failure leaves the order intact (at most a position gap, no scene added). The page then flushes every session and reloads with `pieceId` set to the new scene, so the caret lands there. The surface does not rebuild in place. A poll whose `children` differs from the surface shows a persistent "Scenes changed; reload".

Out of scope for this epic:

- Split a scene at the caret (move text into a new scene). Needs a text move across two leaves with no server transaction (§6).
- Merge two scenes. A later ticket outside this epic (decision 4).
- Delete across a break as one action. Blocked in v1 (decision 5). It would be one Delta per touched scene, posted separately (§6).
- Drag a scene to reorder. Today only the tree and `PUT /pieces/{id}` position would do it.

## 4. Block formats and prose

GUZ-944 (block formats, Fountain/FDX):

- Scene heading, action, character, dialogue, parenthetical, and transition are line attributes on the scene's own newlines (`docs/story-model.md` §6). `splitChange` passes attributes through unchanged, so the per-scene Delta carries them to the server.
- The break-line format must not be one of these names, and the block-format toolbar must refuse the break line.
- A scene's first line is most often a scene heading. The break line sits above it; it does not replace it.
- Markdown shortcuts turn off when the open piece is script (`Editor.tsx:220`); §6 already states that. GUZ-943 can ship that switch, since the surface is the first script-only view.
- Fountain/FDX export walks `layout` in order. It needs the surface's scene list, which is why GUZ-944 depends on GUZ-943.

Shipped (GUZ-944): `script-block` is a block ClassAttributor (`ql-script-block-<type>`, `scene_break.ts`) holding scene-heading, character, dialogue, parenthetical or transition; action is no attribute, so unformatted text and AI output read as action and no migration is needed. Tab and Shift-Tab cycle the type, plain Enter sends one Delta that keeps the line's type and gives the new line its successor (character to dialogue, dialogue to action, heading to action, transition to heading), and a Block type select in the top bar sets it. A format on the break line or across a break reverts with the break notice (`splitChange` returns `crossesBreak`). Quill's built-in list autofill is stopped on the surface (a space after `-` or `1.`), since `markdownShortcuts` only covers the app's own shortcuts. Download Fountain exports the current scene Deltas in surface order (`fountain.ts`). Download FDX (GUZ-992, `fdx.ts`) exports the same lines as a Final Draft `.fdx` file: one `<Paragraph Type>` per non-empty line (Scene Heading, Action, Character, Dialogue, Parenthetical, Transition), text kept as typed except a parenthetical gets brackets when it has none.

Prose stays unaffected:

- Prose groups still open one leaf at a time (`docs/story-model.md` §5). `loadPiece` keeps the group error for prose.
- `use_chapter_sync.ts`, `chapter_session.ts`, and `chapter_sync.ts` do not change. The script hook is new and only mounts for a script group.
- The break blot is registered with Quill globally but never appears in a prose document. A prose delta with that attribute throws before post, as for script.
- Every existing chapter and piece spec runs unchanged and stays the regression proof.

## 5. Ticket breakdown

Own epic: **yes.** Reasons: six tickets with a strict order; a new sync shape (N sessions, one view, two new routes) with its own risk; GUZ-944 and script export block on it; none of it changes prose. Keeping it under the story-model epic hides that risk behind finished schema work.

| # | Ticket | Proof |
| --- | --- | --- |
| 1 | Pure `script_layout.ts`: compose, split, place remote, scene position. No UI | `app/src/md_editor/script_layout.test.ts` plus a seeded property test like `chapter_sync.property.test.ts`: random scene edits and remote revisions; every scene's server text equals its slice of the composed view; no break line in any split Delta. Not browser-visible, so no E2E |
| 2 | API: group scenes GET and group delta poll (decisions 1, 2). See route sketch below | Rust HTTP tests in a new `crates/api/tests/script_group_test.rs`: one GET returns all scenes in `position, id` order with rebuilt `content` and `headRev`; prose group, leaf, and missing id give 422/422/404; poll returns only revisions above each scene's own `since`; a scene id not in the group is 422; one scene with unavailable history reports a conflict for that scene only. Not browser-visible |
| 3 | Open a script group as one surface on routes from ticket 2: N sessions, per-scene posts, one group poll per tick, remote placement, break line with scene title, status, recovery, Resume to the group, same surface below 1100px. Markdown off for script | `e2e/screenplay-scroll.spec.ts` (the GUZ-943 proof). API-seed a script group with two scenes. Both read in one scroll at 1100px and at a narrow width; both break titles show. Type in scene 2: only scene 2's `/deltas` gets a POST; scene 1's `headRev` is unchanged; no per-scene `GET /deltas` is sent. A second tab types at the start of scene 1: scene 2 text is unchanged and the caret is still on the same scene-2 character. Reload; both scenes match the server |
| 4 | Cross-scene guard and navigation: arrows over the break, selection and copy across scenes, blocked delete/Backspace/paste/format across a break with a message (decision 5) | `e2e/screenplay-cross-scene.spec.ts`. Arrow from scene 1 end into scene 2. Select across the break, press Delete: both scenes unchanged on the server, message shown. Backspace at scene 2 start: no merge. Copy across the break gives both texts |
| 5 | AI on the surface: Continue in a scene with a window that spans scenes, Edit in one scene, tint, Continue window, lore window; all scene sync paused while AI runs (decision 6) | `e2e/screenplay-ai.spec.ts`, armed fake provider. Continue at scene 1 end: AI text lands in scene 1 only; scene 2 server text unchanged. Edit across a break is disabled. Lore in a scene-1 match shows in "Lore in window" when the caret is in scene 2 |
| 6 | New scene: an empty scene after the caret's scene through `POST /pieces/{id}/children`, later siblings renumbered, caret moves into it | `e2e/screenplay-new-scene.spec.ts`. Between two scenes, add a scene: it shows between them on the surface and in the tree, and reload keeps that order. A forced failure on the create POST leaves both scenes' text intact and shows the error |

Order: 1 and 2 in parallel → 3 → 4 → 5 → 6. Tickets 4, 5, and 6 touch different code and could run in parallel after 3, but they share `Editor.tsx`; serial is safer. GUZ-944 can start after ticket 3.

Out of this epic: merge scenes (decision 4), split a scene at the caret, delete across a break, drag to reorder scenes.

### Route sketch for ticket 2

Names are proposals; ticket 2 decides.

- `GET /pieces/{groupId}/scenes`: `[{ ...piece, content, headRev }]` for every leaf child of a script group, `position, id` order, each rebuilt in the same transaction as today's leaf GET. A prose group or a leaf is 422; a missing id is 404.
- `GET /pieces/{groupId}/scenes/deltas?since=<id>:<rev>,<id>:<rev>`: `{ children: [id...], scenes: { "<id>": { revisions: [{ rev, delta }] } | { conflict: string } } }`. Each scene uses the existing per-leaf history rules; a scene with missing or future history gets `conflict` for that scene only, so the adapter fails only that session. An id that is not a child of the group is 422. `children` is the current order, so the surface can show "Scenes changed; reload" when another tab adds a scene.

## Not verified

- Quill behaviour of a `contenteditable=false` block blot under IME, drag, and paste. The guard's revert layer exists because of this. Ticket 3 must prove it in Chromium, Firefox, and WebKit.
- Whether the scene title can sit in the break blot without Quill counting it as text. Ticket 3 must check `getLength` and `getText` around a break.
- Payload size of the group GET and poll for a long script (about 100 scenes). No measurement was made.
- Typical scene count per script is an estimate, not data from this app.
- New-scene renumbering is one `PUT` per later scene, not atomic; a partial failure leaves position gaps but the order intact, and no scene is added.
