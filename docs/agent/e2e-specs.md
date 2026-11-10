# E2E Specs

### Browser Smoke (`e2e/`)

`text-counts.spec.ts` checks document, selection and emoji counts. Written; not run for GUZ-914.

`ai-not-configured.spec.ts` checks friendly 503 messages in the model picker, Continue status and Edit alert without exposing server details.

`ai-settings.spec.ts` covers AI settings visibility, key source and removal, validation, key redaction, partial updates, and model picker refresh. Written; not run for GUZ-1256 and GUZ-1257.

`empty-book.spec.ts` checks the empty notice and first chapter creation at 1100px and 1440px, isolating the seeded series from shared data. Written; not run for GUZ-893.

`chapter-switch-no-flash.spec.ts` samples chapter switch (50ms fetch delay) and creation (200ms) at 1100px and 1440px, requires no busy state in the first 300ms or after load, and checks one sample shows the busy status with Continue disabled at a 1s delay.

`server-errors.spec.ts` checks friendly errors for chapter load and lore save at 1100px and 1440px using HTTP 500 SQLX responses.

`ai-shortcuts.spec.ts` independently checks Continue start, Stop, Edit note focus, unconsumed Edit without selection, instruction submission, saved selection from body and rail focus, untouched lore modal shortcuts, and accessibility metadata with the fake AI provider at 1100px and 1440px.

`editor-buttons.spec.ts` checks editor button and download link names, roles and non-pill radii, including lore and the Continue window flag.
`rail-buttons.spec.ts` checks rail geometry, names, titles, selection preservation, panel toggles and pending Continue at 1100px and 1440px.
`lore-modal.spec.ts` checks the visible and accessible pin and shared matching hints, draft dismissal and reset, focus restoration, saving, and two-click deletion with API persistence. Matching hint assertions written; not run for GUZ-915.
`lore.spec.ts` checks the saved Series lore row name, level, preview, and accessible name. Row assertions written; not run for GUZ-909.

`lore-cycle.spec.ts` at 1100px seeds one lore entry with three matches separated by filler, and checks that repeated "Lore in window" clicks scroll to successive matches, wrap to the first, and keep the card open until Escape. Written; not run for GUZ-911.
`inspector-readonly.spec.ts` checks read-only Document and Selection, absent reset controls, and editable Instruction and Story context in Full prompt at 1100px and 1440px.

`inspector-fields.spec.ts` checks inspector, settings, manuscript and lore headings and field semantics at 1100px and 1440px.

`inspector-selection.spec.ts` checks Continue caret display, selection token estimates and Edit selection at 1100px.
`edit-tint.spec.ts` checks applied Edit tint, typing cleanup, preference and the 1100px breakpoint.

`inspector-cost.spec.ts` uses a fake provider at 1100px and 1440px to check readable cost fields, the selected model label and ID tooltip, and request model visibility when selected and run IDs match or differ.

`settings-popover.spec.ts` checks Settings dismissal and equal row gaps at 1100px and 1440px.

`ui-components.spec.ts` serves `e2e/fixtures/ui-components/` with `Bun.serve`, asserts every design-system primitive and key attributes, checks Modal and Popover dismissal and Modal focus restoration, and captures full-page screenshots at 1100px and 1440px in `target/guz-874/<browser>/` and `target/guz-884/<browser>/`.

`edit-note.spec.ts` checks the Edit rail toggle, selection restoration, card and note geometry, and matching text sizes at 1100px and 1440px.

`edit-selection-highlight.spec.ts` checks the saved Edit range stays highlighted while the instruction is focused, clears on dismissal, Apply replaces only the selection, and GUZ-919 instruction text selection at 1100px and 1440px.

`edit-lore-popover.spec.ts` checks Lore match dismissal, no Close button, trigger ARIA, and left-out match hover and Alt+L;  it checks the Edit note appearance, viewport bounds, Escape selection restoration, trigger ARIA and outside dismissal at 1100px and 1440px.

`edit-replacement.spec.ts` arms one-use `e2e-edit` identities at 1100px: tags split across fragments with commentary around them leave only the replacement in the editor and server; missing or unclosed tags show the error and restore the selection; a whole-paragraph selection with its line ending keeps the paragraph break (replacement `Revised paragraph` or `Revised paragraph\n`, editor and server GET), and an unclosed tag restores that paragraph.

`piece-move-lore.spec.ts` at 1100px seeds one universe with lore and two collections, records the Continue lore list for a piece, places the piece in the other collection through `POST /pieces/{id}/placement`, reloads, and checks the same lore list, text, head revision, and universe lore.

`screenplay-scroll.spec.ts` at 800px and 1100px opens a two-scene screenplay as one surface with both break titles; checks per-scene saves, remote edits keeping the caret, paste, drop, drag and IME next to a break (Chromium real CDP IME, Firefox/WebKit synthetic composition events), a server delete of a scene's trailing newline, delete+reinsert of a scene's last newline, save failure recovery, reload and Resume.

`screenplay-cross-scene.spec.ts` at 1100px opens a script surface from a scene and from the group in the manuscript tree (caret in that scene; reopening the group restores the last caret), steps arrows over the break, selects across it, blocks Delete/Backspace/typing/Enter/paste over a cross-break selection and Backspace/Delete that would merge scenes with an Alert, keeps Enter at a scene end in that scene, checks each scene's server GET, and keeps prose pieces opening alone; Alert screenshots at 1100px and 1440px in `target/guz-984/<browser>/`.

`screenplay-notice.spec.ts` at 1100px and 1440px opens a two-scene script with a 40-line first scene: a tree click on a scene of the open script moves the caret there with no navigation and highlights that scene; a cut event and a drop on the break line across a break are refused before they apply (no edit, undo still restores the last typed text); the break Alert shows its message once on the page, sits in the sheet's top padding below the header, and never overlaps the scene's last line or shifts the sheet. Screenshots in `target/guz-988/<browser>/`.

`screenplay-new-scene.spec.ts` at 1100px and 1440px: New scene with the caret in scene 1 adds a scene between two (surface, tree, server order, reload), puts the caret in it and typing saves only to it; a forced create failure and a forced failure of the first or second renumber PUT each show "New scene not added" and a reload keeps the exact scene order and text on the surface, in the tree and on the server; a scene added from another tab shows "Scenes changed; reload" without rebuilding the surface. Screenshots in `target/guz-986/<browser>/`.

`screenplay-format.spec.ts` at 1100px and 1440px: on a two-scene script, a scene heading set from the Block type select and character/dialogue lines made with Tab and Enter save as `script-block` attributes on that scene's server GET and survive reload; the Download Fountain link yields the expected `.fountain` text; typing `# `, `- ` and `**x**` creates no header, list or bold; a format on the break line shows the break Alert and is never saved. At 1100px, remote same-scene insertion, Continue Finish, and Edit strictly inside a typed heading preserve block types on the surface and group scenes GET after reload; action lines have no `script-block` attribute. Edit including the heading newline keeps that line ending: `Revised` stays a scene heading and `ANNA` a character on the surface, after reload and on the group scenes GET. Screenshots in `target/guz-944/<browser>/`.

`screenplay-ai.spec.ts` at 1100px arms one-use real-provider identities on a two-scene script surface: the Continue lore window lists scene-1 lore with the caret in scene 2; Continue waits for a held scene-2 save and writes output only in scene 2; Edit over a break shows a reason and disables Apply, Edit inside scene 2 saves only scene 2; Continue at scene 1's end sends no group poll while it runs, saves only scene 1 on Finish and then shows a remote scene-2 edit; Stop at scene 2's end leaves both scenes' server GET and headRev unchanged.

`screenplay-fdx.spec.ts` at 1100px sets every block type on a two-scene script with the Block type select, downloads `Hall script.fdx`, parses it with `DOMParser` (no `parsererror`) and checks paragraph Types and Text in scene order, including an escaped `&`; Chromium saves the sample to `target/guz-992/sample.fdx`.

`persistent-dev-database.spec.ts` at 1100px starts `target/debug/api` in a temporary directory with no `--db-url` and no `QUILLAI_API_DB_URL`, creates a chapter, types further text, kills that process, starts it again in the same directory, and checks the editor and the chapter GET still show the text. It does not use the sandbox database and does not open `<workspace>/quillai.db`.

`continue-inline.spec.ts` checks inline and paragraph placement, suffix preservation, AI tint ownership, and exact Stop rollback.

`continue-empty.spec.ts` arms one-use real-provider identities at 1100px and checks empty length and stop finishes, exact Alerts, unchanged chapter/piece, selection and AI thread, and Alert clearing.

`continue-reasoning.spec.ts` arms real-provider identities at 1100px: a model with low reasoning effort writes text that persists on server GET; a model without it returns an empty length finish and leaves the chapter unchanged.

`continue-tie-order.spec.ts` also arms one-use real-provider identities at 1100px: server-priority ties place remote typing before inline AI text; Finish retains both, and Stop keeps only the remote text, with exact two-tab/server convergence and no API errors.
`stream-remote-edit.spec.ts` checks exact two-tab/server convergence after Finish or Stop during desktop Continue at 1100px, preserving author and remote edits at distinct insertion points, retaining AI text only on Finish, and rejecting API errors; AI transport is mocked, not the real generation API.

`emoji-cursor-remote.spec.ts` checks caret/selection surrogate boundaries during remote insert/delete, subsequent typing, HTTP errors, and exact two-tab/server convergence at 1100px, including adjacent 😀😃 and a deliberately requested mid-surrogate caret.

`story-tree.spec.ts` at 1100px and 1440px seeds a book-series novel chapter and a loose piece in one universe; the tree lists every universe (other universes collapsed first, the open one expanded last) with its containers and loose pieces; book series show their façade novels and other roots (`manuscripts/story_tree.ts`), the piece opens by `pieceId` (lore from the piece's universe via `/universes/{id}/lore`), syncs through `/pieces/{id}/deltas`, survives reload, and the chapter still opens.

`tree-delete.spec.ts` covers row controls, immediate deletion, Undo and Recently deleted for six item kinds at 1100px and 1440px. It gates an open chapter save to check deletion waits for pending text.

`tree-load-retry.spec.ts` at 1100px and 1440px fails the tree fetch until retry, checks the exact "Try loading again" button and absence of "Retry manuscript", then retries and checks the seeded chapter is visible. Written; not run for GUZ-886.

`create-form-fit.spec.ts` at 1100px and 1440px checks every field and button of the container and loose piece create forms sits inside the tree area without manual scrolling (the open form scrolls itself into view; Label and Format sit side by side), create controls align with sibling rows, and the no-scope lore wording. The initial no-scope visit mocks an empty series list to isolate it from the shared E2E database; screenshots in `target/guz-971/<browser>/`.

`landing.spec.ts` at 1100px and 1440px opens `/` with no target: three seeded universes show as collapsed cards (narrower than half the page) and an opened universe card spans the grid row; an edited leaf (not one only opened) becomes the Resume card with its universe · container path and Open, and its universe starts open; the empty state shows when nothing is stored; a stored leaf deleted on the server is removed from Resume; a chapter-scope (series/book/chapter) Resume shows its universe · series · book path and Open reopens it; the editor's Library button saves and returns; a loose piece created from landing opens and stays listed after reload. Screenshots in `target/guz-978/<browser>/`.

`landing-activity.spec.ts` edits a chapter, reloads the landing page, and opens it from Recent activity. Written; not run for GUZ-973.

`landing-tips.spec.ts` checks that dismissing Tips persists after reload. Written; not run for GUZ-975.

`landing-recent.spec.ts` edits two chapters, checks both Resume rows, and verifies Forget persists after reload.
WebKit page errors containing `due to access control checks` are ignored for in-flight fetch teardown during reload (GUZ-1163).

`reload-in-flight.spec.ts` checks server-authoritative reload, full unsaved recovery export, and resumed saves at 1100px.

`offline-stale-base.spec.ts` holds one tab's delta submission offline while the other commits three edits, then verifies HTTP 200 for the stale base and exact convergence of both tabs and server with all in-flight and pending edits at 1100px.

`model-picker.spec.ts` also runs in the explicit allowlist. It mocks `/ai/models` and `/settings/ai`, checks the Inspector has no Continue/Edit model menus and shows the Settings model on its Model line (or "unknown — unresolved model" when the catalog fails), and checks Continue and Edit requests carry no `model` (GUZ-1427). Screenshots at 1100px and 1440px go to `target/guz-680/<browser>/`. `inspector-cost.spec.ts` covers the "Request model" line: it appears after a Settings save changes the model once a run has finished.
`model-label.spec.ts` also runs in the explicit allowlist. It mocks `/ai/models` and `/settings/ai`, checks the Continue rail button tooltip names the model ("· Precise"), the Continue window and Edit popover show a "Model: <name>" link (title = model id), and that clicking either link opens Settings focused on the matching Continue/Edit model menu (GUZ-1426). Not run in the ticket.

`playwright.config.ts` loads the app's `@playwright/test` dependency and runs every allowlisted spec in Chromium, Firefox and WebKit projects by default. Install the browsers once with `cd app && bun --bun x playwright install chromium firefox webkit`. Specs run in the nightly run (GUZ-1206), not inside tickets; a full three-browser run is estimated to take about 10 minutes. The chapter smoke seeds one chapter through the API, opens two pages with the sandbox `apiBaseUrl` and chapter IDs, edits from each page, checks remote convergence and server revisions/content, then reloads both pages. The guidance smoke verifies locked System, explicit save, escaped preview, reload, and clearing. The mocked Continue stream checks concurrent author edits, selection preservation, exact rollback, local tint preference, and completion. A `usage` event before `done` does not end the run, and `finishReason: length` still arms Continue with thread. Desktop edge cases check Settings dismissal and live tint during streaming, plus transport failure and premature EOF rollback with author inserts, at 1100px and 1440px. Continue window coverage checks bracket geometry and unchanged transmitted slices, single-marker thread history, off-screen flag navigation, persisted visibility, and the 1100px breakpoint; screenshots at 1100px and 1440px go to `target/guz-757/<browser>/`. Lore coverage creates, edits, matches, reloads, and deletes series lore, verifies author-only notes never enter Continue/Edit requests, and captures the panel/inspector at 1100px and 1440px in `target/guz-671/<browser>/`. Named screenshot and artifact directories include the browser project (`chromium`, `firefox` or `webkit`). Alerts and toggles coverage checks four error states, failed-save alert semantics, pending and saved status, and checkbox state at 1100px and 1440px. Save recovery forces a chapter delta POST failure, waits for the red Not saved pill, releases the route, and checks the client resumes so server content matches text typed before and during the failure at 1100px. Other artifacts go to project-specific test directories under `target/xtask-e2e/results/`. The directory is outside `app/`, so `cargo xtask lint`'s `bun test` does not discover browser tests. See [browser sandbox context](../../.claude/context/ai-editor.md#browser-smoke-sandbox).

`ai-thread.spec.ts` uses a fake provider to check cut-off reply resume, discard, and chapter isolation.

`notices.spec.ts` checks notice tones and roles for scene breaks, empty Continue, guidance save, tree deletion and prompt copy at 1100px and 1440px.

`full-prompt.spec.ts` checks read-only full prompt order and Escape at 1100px and 1440px.

`recently-deleted.spec.ts` at 1100px and 1440px checks trash item types and dates, restore and library refresh, ancestor confirmation, and permanent deletion with descendant counts.

`sync-push-screenplay.spec.ts` checks idle grouped reads, native remote scene delivery, scroll preservation and deleted-scene fallback at 1100px and 1440px. `sync-push-chapter.spec.ts` checks idle delta reads, native stream remote saves, deleted-leaf fallback and page lifecycle reconnect for legacy chapters and pieces at 1100px and 1440px. `fixtures/fake-event-source.ts` provides controllable EventSource events for deterministic fixture tests.

`caret-restore.spec.ts` checks chapter switch and reload selection and viewport restoration from the first populated frame at 1100px and 1440px.
`caret-reflow.spec.ts` checks that at 1100px the caret line keeps its viewport position when the Manuscript drawer opens and the editor narrows (GUZ-1409).

`tree-rename.spec.ts` checks series, book, chapter and piece rename at 1100px and 1440px: Enter/blur saves trimmed titles, Escape cancels, blank titles fail, unchanged titles send no request, server titles persist, and the open chapter header and tab update. Written; not run for GUZ-895.

`chapter-export.spec.ts` checks chapter and book download names, Markdown, text, Word, and EPUB formats, chapter order, unsaved live text with saves held, and export failure at 1100px and 1440px. Word and EPUB downloads check ZIP signatures and package contents. Written; not run for GUZ-1240.

`stories-visual.spec.ts` serves `storybook/storybook-static` from a small in-spec HTTP server and screenshots each `story` entry from `index.json` (one test per story, docs entries skipped, max 1 percent pixel diff). The whole file skips when the build is missing. Written; not run for GUZ-1438.

`lore-states-inspector.spec.ts` creates a universe with two timeline points and a lore entry with a state on each, then opens three pieces (point Early, point Late, no point) and checks the context inspector shows `state Young`, `state Old`, and `base` beside the entry (GUZ-1460).

`lore-timeline.spec.ts` opens a piece with no point, checks the "This chapter happens at" picker shows `No point (base text)`, adds two points in the Timeline modal, moves one up, renames one, then picks it and checks the choice survives a reload. A second test seeds a lore entry with a state at a point, picks the point and checks the inspector shows the state, returns to base, deletes the point in the Timeline modal and checks the picker falls back to `No point (base text)`, and checks a 500 on `PUT /pieces/:id` shows an alert and leaves the picker unchanged (GUZ-1461 stage 1). Written; not run.
`lore-states.spec.ts` seeds a universe with two timeline points and one lore entry, opens the entry modal, and in its States section adds a state at Early, sees the duplicate-point message for a second state at Early, moves the first to Late, and deletes it with the two-click confirmation, then checks the API lists no states (GUZ-1461 stage 2). Written; not run for GUZ-1461.

`dialogue-typing.spec.ts` (GUZ-1532) at 1100px and 1440px types `"Hi," she said.` and checks the saved `“Hi,” she said.` survives reload; checks `don’t`, nested `‘no’`, the `’90s` fix-up, and `—Hola —dijo—.`; one Cmd/Ctrl+Z restores the straight `"` and a second removes it; a paste stays straight; `**bold**` still formats; a CDP IME commit of one `"` converts; Continue-streamed quotes stay straight while a typed quote converts and `--` stays during the stream; a screenplay group keeps `"` and `--`.

`dialogue-exchange.spec.ts` (GUZ-1534, stage 3a) at 1100px and 1440px picks a Dialogue model in Settings → AI, checks the PUT body is `{ dialogue_model }`, that the re-fetched catalog reports it as `defaults.dialogue`, and that the menu keeps it with "Saved here" after reload. Stage 3b adds cases (1)–(6) at both widths with a page-side mocked `/ai/generate` stream: two speakers stream as a new paragraph at the caret and persist after reload, and a mid-line caret splits the line there; the request body holds speakers, brief, speaker lore first, no `model` and no hidden notes; a Leave out character is not listed; Stop and a stream error roll back only AI text and keep author typing; a screenplay group gets character/parenthetical/dialogue blocks; 503 `AI_NOT_CONFIGURED` shows the friendly error with no text change.
