# Editor Design (v1 desktop)

Approved visual and behavior spec. Visual reference: `prototypes/editor-desktop/` (`index.html`, `style.css`, `proto.js`; plain contenteditable, fake data). Values below were read from `style.css`. Where the prototype differs from the decisions, see "Prototype vs decisions".

## Scope

- Desktop first: 1100px and wider. Below 1100px, open panels use the document flow between the rails, without covering the editor. Mobile visual design remains provisional.
- Existing `context_inspector` (GUZ-664/665/666) is the right-hand drawer. The centre sheet contains only the chapter title, Quill toolbar, and manuscript text. There is no controls card above the sheet.

## Palette

| Token | Value | Use |
|-------|-------|-----|
| `--paper` | `#FFFFFF` | Sheet, panels, cards |
| `--desk` | `#F1EEE7` | Background behind the sheet |
| `--ink` | `#1F2933` | Body text, hover/active tool color |
| `--slate` | `#4E6272` | Headings, caret, AI-streaming text tint, primary button |
| `--graphite` | `#6F757C` | Muted tools, secondary text |
| `--pencil` | `#A9ADB1` | Save dot, faint notes |
| `--rule` | `#EDE3D3` | Hairlines, input borders |
| `--alert` | `#B4412F` | Failed save, "Ignore this match" |
| `--pi-yellow` | `#F4E8B5` | Post-it: author guidance |
| `--pi-pink` | `#F3D6D8` | Post-it: Edit instruction; selection; tool hover bg |
| `--pi-green` | `#D7E6D0` | Post-it: lore note; active tool bg |
| `--pi-blue` | `#D2E2EC` | Flag: Continue window |
| `--pi-lav` | `#E0D8EC` | Lore flag and hover |
| `--st-blue` | `#7C9BB0` | Continue bracket stroke |
| `--st-lav` | `#9B8BB8` | Lore underline |
| `--st-tan` | `#B79A6C` | Lore "don't reveal" underline |
| `--sheet-shadow` | `0 1px 2px rgba(31,41,51,.06), 0 10px 34px rgba(31,41,51,.09)` | Sheet, drawers, author guidance |
| `--note-shadow` | `0 1px 1px rgba(31,41,51,.06), 0 6px 14px -6px rgba(31,41,51,.22)` | Post-its, flags |
| Fail shadow | `0 4px 18px rgba(180,65,47,.35)` | Red save pill |

Also used: panel shadow `0 0 40px rgba(31,41,51,.12)`; popovers `0 8px 30px rgba(31,41,51,.12)`; edit box `0 10px 30px rgba(31,41,51,.13)`; lore card `0 10px 30px rgba(31,41,51,.14)`; sheet edge `1px solid rgba(31,41,51,.07)`.

## Type

| Role | Value |
|------|-------|
| Body (sheet) | Georgia, Iowan Old Style, Palatino; 21px / 1.75 |
| Chapter title | sans 600 13px, 0.22em tracking, uppercase, `--slate`, centered |
| UI | system sans (-apple-system, Segoe UI, Helvetica) |
| Desktop app label (QuillAI) | sans 500 16px / 1.5, muted `--graphite`; chapter title remains the sheet heading |
| Panel/popover headings | sans 600 12px, 0.2em tracking, uppercase, `--slate` |
| Inspector text | mono 12px / 1.6, `--graphite` |

## Writing surface

- Centered sheet: `width: min(760px, 100vw - 400px)`; padding `72px 84px`; radius `2px 2px 0 0`; visible hairline edge plus `--sheet-shadow` on `--desk`.
- At 1100px and wider, an open drawer reserves 356px on its side (56px rail plus 300px drawer) and 56px on the opposite side. Settings reserves the same right-side space so its floating popover cannot cover prose. The sheet and header center in the remaining desk, with width `min(760px, 100% - 48px)`. Both sheet edges stay visible. At 1100px the sheet is 640px wide; at 1440px it is 760px wide. Closed-drawer layout is unchanged.
- Infinite page: the sheet bottom border is removed and never shown.
- Scrollbar hidden; scroll container is full viewport.
- Caret color `--slate`.

## Caret and scroll

- At 1100px and wider, Continue leaves the editor writable. Its output starts at the original caret, except on a blank line or after a completed sentence at the paragraph end, where it starts a new paragraph. Inline output receives a separating space when needed. Every author change transforms the insertion anchor, append end, and exact surviving AI-owned ranges with `quill-delta`. Later fragments append at that transformed end.
- Stream insertion, completion, and rollback preserve the logical caret or selection, including selections after the AI text. No recentering or stream-driven scrolling is added. Further smooth-writing work remains in GUZ-763.
- Desktop Continue Stop, server cancellation, and failures delete only surviving AI-owned text, including any inserted separator. Author inserts inside that text survive. Edit selection and screens below 1100px retain their existing locked-editor behavior.

## AI text

- Desktop Continue text uses `--slate` while streaming. CSS custom highlights cover only surviving AI-owned text; author inserts are not tinted. Tint is local presentation, never chapter Delta formatting.
- On successful completion, tint transitions to plain ink over 1.8s after a 250ms delay, then clears. Cancellation and failure clear it with rollback.
- The desktop Settings gear popover contains Tint AI text, on by default. The on/off preference uses `editor_persistence` local storage, not server settings. Browsers without CSS custom highlights show plain ink. Prototype extra effects remain out of scope.

## Tools

- Left rail (56px): navigation (manuscript tree panel, 300px, opens at `left: 56px`).
- Right rail (56px): Continue, Edit selection, Context inspector, Settings (gear pinned bottom). Inspector drawer 300px at `right: 56px`.
- Rails are muted: opacity .35, .95 on hover/focus-within. The right rail stays at .95 when the thread indicator is present so history use remains visible. Tool buttons are 36px with radius 10px and `--graphite`; Continue uses the full 56px rail width for its label and indicator.
- At 1100px and wider, both rails have 4px top padding; the first buttons and the QuillAI label share the context drawer heading's centre line near 22px from the viewport top. The app header has a 10px top margin. The right rail and drawer heading define this alignment.
- Hover = more contrast (`--ink`), `--pi-pink` background, `translateY(-1px)`. Hover never opens or closes a panel. Panels toggle by key or click; Esc closes all.
- Rail buttons use `mousedown preventDefault` so they do not steal caret or selection.
- Product rails use IconButton for Library, manuscript, Continue/Stop, Edit selection, context inspector, and a bottom-pinned desktop Settings gear.
- The right drawer holds Story Context, Instruction, and a cut-off reply notice with Discard when a reply can resume. Existing Max Output Tokens and Thread continuation prompt controls live under Advanced AI controls in this drawer. There is no Preview Context button or separate preview modal.
- At 1100px and wider, Settings opens a 280px popover at right 72px, bottom 18px, outside the prose. It holds Window Size, Whole document, Show Continue window, and Tint AI text, using persisted local preferences. Window controls remain disabled during AI runs; marker visibility and tint remain available. Opening it closes either drawer; opening a drawer closes Settings. Settings has no Close button: its gear toggle, click outside, and Escape close it, and the gear and Escape restore the remembered editor selection/focus. Narrow screens retain the window controls in the context drawer. Prototype-only recentering, extra text effects, and save-failure simulation are not exposed.
- Export local chapter is in the left manuscript drawer. It remains available after a save failure.
- Both drawers start closed. Only one can be open. Bracket shortcuts toggle their drawer. Neither drawer has a Close button; Escape and a second click on the drawer's rail toggle close it and restore the remembered editor selection and focus. Hover changes only appearance.
- Drawers and all descendants have square corners. Drawers use `--sheet-shadow`; buttons and the request picker stay flat, with no shadow or hover lift. Section boxes remain square. Below 1100px, drawers remain in document flow.
- The manuscript drawer loads series → books → chapter summaries through the existing manuscript client, with loading, empty, error, and retry states. Selecting a chapter waits for the current save and any pending edits, then fetches that chapter once through the single-chapter GET. Editing and further switches pause until loading completes. Only a successful load changes the title, selected highlight, URL, and AI context. Save/load failures keep the current chapter and its text, show an error, and preserve recovery/export. Reopening refreshes the hierarchy.
- Z-order: hint 4, flags 5, panels 8, rails 9, settings popover 9, switcher/edit box 10, lore card 11, save bar 20.

## Sticky notes (styled textareas)

Common: `.postit` textarea, full width, vertical resize, no border, radius 1px, padding `14px 12px 10px`, sans 13px / 1.5, `--ink` text, `--note-shadow`, top "adhesive" gradient `rgba(31,41,51,.045)` to transparent over 12px, focus ring `0 0 0 2px rgba(78,98,114,.35)`. Placeholder `#6b6f73`. Prototype notes are flat: no rotation. Any rotation is a design addition, not in the prototype; if added, keep it under about 1deg.

Author guidance uses the yellow post-it styling above, with owner-approved square corners and `--sheet-shadow` instead of the common radius and note shadow. Its focus ring is added to the sheet shadow; the yellow fill remains on focus.

The shared `StickyNote` textarea provides both yellow guidance and pink Edit input. Guidance still saves only with Save author guidance. The right-rail Edit selection control toggles the focused pink note without sending a request; closing it restores the editor selection. The note overhangs the borderless, softly shadowed card, and its flag, input, hint and Apply button share the label type size. Enter and Apply use the same selection, busy, chapter-ready, and blocked-save checks before invoking the existing Edit stream. Shift+Enter inserts a newline, and Esc cancels without changing manuscript text and restores the editor selection. Composing text does not submit. The note edits the existing shared Instruction value used by Continue and Edit.

| Note | Fill | Appears | Where |
|------|------|---------|-------|
| Yellow, author guidance | `#F4E8B5` | Inspector drawer, always, 5 rows | Section "Author guidance" |
| Pink, Edit instruction | `#F3D6D8` | Edit click with a non-empty selection; Enter applies, Esc cancels | Edit box, right 72px, top 96px, 280px wide |
| Green, lore text | `#D7E6D0` | In the lore card, read-only visible AI context | Lore card |

Arrow-shaped flag tabs (`clip-path`, 11px sans, `--note-shadow`) label the edit box ("Edit selection", pink), the lore card ("Lore match", lavender), and an off-screen Continue window start. The Continue bracket itself is a gutter stroke, not a flag.

## Context window

- At 1100px and wider, one blue Continue bracket in the left gutter marks the exact live `calculateDocumentWindow` start-to-end range: 9px wide, 2px `#7C9BB0` stroke, opacity .6, at `left: -24px`. The lower cap reaches the end index's line bottom, which can be after the caret. Whole document spans the full calculated document length. The calculator and transmitted slice are unchanged. Saved thread history does not create a second document window or lavender bracket.
- When the window start is above the viewport, a flag sits at the top reading `Continue window / starts ↑ N pages up`. N is the distance in viewport heights rounded to an integer; less than one viewport height reads `starts ↑ just above`. Clicking scrolls to the start.
- Settings includes a persisted Show Continue window checkbox, enabled by default, alongside Tint AI text. It hides both bracket and off-screen flag. Markers follow selection, author typing, concurrent Continue fragments and author edits, window controls, layout changes, and scrolling without moving the caret or recentering the viewport; only clicking the flag scrolls to the start. Below 1100px both markers are absent.

## Lore

- The left manuscript drawer includes Series lore for the current chapter's series, with loading, empty, error, and retry states. New lore and entry buttons open a modal for kind, name, newline-separated keywords/aliases, visible AI context, author-only hidden notes, request level, and pin. Save and Delete persist through the series-scoped API. The modal has no Cancel; Escape and backdrop click discard the draft. Delete is red and confirmed: the first click arms it, the second deletes. Author-only notes never enter AI requests. Series changes clear previous entries and drafts.
- The context drawer's Injected lore section previews the next selected request: name, kind, request level, keyword or pin reason, and per-entry token estimate, followed by matches skipped by budget. It uses the same transmitted window, level filter, matcher, and 65,536 UTF-16 budget as request assembly. Pins may exceed the cap. Per-entry estimates include standalone Lore tags and are not additive; the existing Lore prompt section provides the combined estimate. No author-only notes appear here.

- At 1100px and wider, names, aliases, and keywords inside the calculated Continue window get visual-only CSS Custom Highlights, independent of bracket visibility. No Quill format or saved Delta changes. Hover for 600ms (or Alt+L for the nearest match) opens a 300px lore card with lavender highlight; Esc closes.
- If `CSS.highlights` or `Highlight` is unavailable, omit underlines and lavender hover coloring without errors. Cards still use DOM ranges: pointer hover, list navigation, Alt+L, and Escape remain functional. Chromium coverage deletes both APIs before startup; real Firefox/WebKit runs remain GUZ-762.
- Card: name, kind, request level (Include | Known, don't reveal | Leave out), and read-only visible text on green. Hidden notes never appear. Editing radios, story-point selects, and per-match ignore/note controls remain prototype-only.
- Underline style per level: include dotted lavender; known-don't-reveal dashed tan; leave out dotted pencil.
- Inspector Document and Selection show read-only request content. Instruction, Story context, and history remain editable for the next request.
- The inspector Full prompt button opens a read-only modal with ordered prompt parts, estimated tokens, and Copy. Escape closes it.
- Inspector section "Lore in window": the lore Continue would send, with type, match count, and level. It shares `context_inspector.ts`'s injected lore model and budget, even when inspecting Edit. Leave-out entries and budget skips are absent; unmatched pins remain with zero matches and disabled navigation. Click a matched entry to scroll to its next match (wrapping after the last) and open its card. Each further click on the same entry moves to the following match and the card stays open; Escape or an outside click closes it. Clicking only scrolls and never moves the caret.

## Continue

- One Continue control in the right rail; no separate Resume or inline Continue button. Its visible "from a cut-off reply" indicator appears whenever the next request includes history, including inspector history overrides. Clearing history overrides removes the indicator; instruction changes, thread reset, chapter changes, and cancellation use the existing thread lifecycle.
- Continue and Edit reserve the session and save pending changes before the AI request. The control shows Saving… and remains available as Stop AI; typing remains available while saving. Save errors stop the request.
- While streaming the same control shows Stop. Desktop Continue rolls back only surviving AI-owned text. Edit and narrow-screen Continue keep their previous cancellation behavior.
- The context drawer has one Continue view, labeled "Continue · from a cut-off reply" when history is included, otherwise "Continue". Edit selection remains a separate request view. Prompt text and payload assembly are unchanged.

## Lore field completion (GUZ-912)

Spec for a later build ticket. "Proposed" marks decisions this spec makes; the rest is owner-approved or existing behaviour with a citation.

### Scope

- Completion works only in the lore modal's visible AI context field (`visible_text`, green StickyNote, `app/src/md_editor/components/LorePanel.tsx:114-115`). It never runs in the author-only hidden notes field (`LorePanel.tsx:116-117`). "Author-only notes never enter AI requests" (Lore, above) stays unchanged.
- Keywords and aliases fields (`LorePanel.tsx:109-112`) get no completion (proposed).
- Inspector: no request view or cost line for lore completion in this build; deferred to a follow-up ticket (owner-approved).
- Same behaviour at all widths, including below 1100px (owner-approved).

### Action

- New AI action `lore_complete`. Today the client union is `"generate" | "edit"` (`app/src/md_editor/ai_contract.ts:13`) and the server enum is `Generate | Edit | Image` (`crates/api/src/ai/models.rs:75-79`, wire names `models.rs:84-85`). The build adds `lore_complete` to both.
- Model choice follows the per-action Settings rule (Model selection, above): Settings → AI gets a Lore completion model select next to Continue and Edit; the request sends no `model`; the server uses the saved model for the action, then the catalog default (`crates/api/src/ai/catalog.rs:191-194`). Saved per-action models live in `ai_settings` (`crates/api/src/settings/ai.rs:38`); the build adds a `lore_complete_model` column by migration (proposed).
- `GET /ai/models` adds `defaults.lore_complete` (proposed), resolved like `defaults.generate` (`docs/agent/api-routes.md:62`). A catalog file without a `lore_complete` default falls back to the `generate` default (proposed).
- Missing model: same as other actions. Missing or unknown saved IDs use the server action default (`.claude/context/ai-editor.md:13`); no catalog returns 503 `AI_NOT_CONFIGURED` (`docs/agent/api-routes.md:62`, `crates/api/src/error.rs:52`), shown as the field error below.

### Prompt

- Input: the entry's draft name, kind, and the `visible_text` before the caret. Text after the caret is not sent (owner-approved). Hidden notes, keywords, aliases, level, and pin are never request fields, matching the lore contract (`.claude/context/ai-streaming.md:62`).
- Other lore entries are not included. No manuscript window, selection, history, or Story context is sent (proposed). Author guidance is still added by the server for every accepted request (`.claude/context/ai-streaming.md:56`).
- Known, don't reveal adds no non-reveal marker for the target: the text is lore notes, not prose (proposed).
- New rules constant beside `GENERATE_RULES` (`crates/api/src/ai/prompt.rs:3`), because Continue rules assume a manuscript caret. Proposed text: "You are completing a reference note about a story element. Respond with plain text only, and do not exceed two sentences. Start exactly where the note ends. Do not repeat or rephrase it." Name, kind, and text go in escaped tags, using the same `&`, `<`, `>` escaping as lore (`.claude/context/ai-streaming.md:64`).
- `visible_text` before the caret is capped at 65,536 UTF-8 bytes before escaping, the lore cap (`.claude/context/ai-streaming.md:64`); overflow returns 422 before provider startup (proposed).
- Code placement: `crates/api/src/ai/provider.rs` is at 992 of 1000 lines (`cargo xtask lint` cap, CLAUDE.md Rust Conventions). Request parsing and prompt building go in a new `crates/api/src/ai/lore_complete.rs`; `provider.rs` changes only by the action match arm (proposed).

### Trigger

- A Complete button (IconButton, `app/src/ui/`) under the visible AI context field, disabled while the modal is pending (proposed).
- Ctrl/Cmd+Enter inside the visible AI context field triggers it (proposed). Today Ctrl/Cmd+Enter runs Continue only outside dialogs and popovers (Shortcuts, above), so it does not collide. In hidden notes and other modal fields it keeps the browser default.
- Leave out entries are blocked (owner-approved): the Complete button is hidden and Ctrl/Cmd+Enter sends no request when the entry's resolved level is `leave_out`. Use the same check LoreImages uses to hide Generate (`app/src/md_editor/components/LoreImages.tsx:33`, `:72`), the `resolveLoreEntry` resolver (`app/src/md_editor/lore_states.ts:29`, GUZ-1433).
- While streaming the button shows Stop and Ctrl/Cmd+Enter stops, matching Continue/Stop (Continue, above).

### Streaming and accept/reject

- Streams NDJSON like Continue (`.claude/context/ai-streaming.md`), not a single response.
- Output inserts at the caret, with a separating space when needed, as Continue does (Caret and scroll, above).
- Accept/reject matches Continue (Continue and Caret and scroll, above): success keeps the text with no extra confirm; Stop, server cancellation, and failure delete only the AI-inserted text. Nothing persists until the author presses Save in the modal (`LorePanel.tsx:100`); Escape and backdrop click still discard the whole draft (Lore, above).
- The field is read-only while streaming (proposed). A textarea cannot hold CSS custom highlights or keep author inserts apart from AI text, so this follows the locked behaviour Edit and narrow-screen Continue keep (Caret and scroll, above). No tint (proposed).
- Save and Delete are disabled while streaming; closing the modal cancels the request (proposed).
- Max output: `DEFAULT_MAX_OUTPUT_TOKENS` 512 (`ai_contract.ts:15`) plus the two-sentence rule. The drawer's Max Output Tokens control does not apply (proposed).

### Errors

- Failures roll back, then show the shared `Alert` fail style under the field with the server error message; the draft stays editable and Complete can retry (proposed). Error codes follow the JSON error contract (`.claude/context/error-handling.md`).
- An empty field still completes from name and kind (proposed). An empty draft name disables Complete, as it disables Save (`LorePanel.tsx:100`).

### E2E cases (build ticket writes them; they run nightly)

New spec `e2e/lore-complete.spec.ts` (proposed name), at 1100px and 1440px:

1. Complete in visible AI context streams text at the caret; Save persists it; reload shows it.
2. Request body has action `lore_complete`, name, kind, text before caret only; no `hidden_notes`, keywords, aliases, other entries, or `model`.
3. Hidden notes and keyword fields show no Complete button; Ctrl/Cmd+Enter there does not send a request.
4. Ctrl/Cmd+Enter in the visible field starts, a second press stops; Stop removes only AI text.
5. Server error and premature EOF roll back the AI text, show the error, keep the draft.
6. 503 `AI_NOT_CONFIGURED` shows the error with no text change.
7. Escape during a stream cancels the request and discards the draft; the saved entry is unchanged.
8. Settings → AI Lore completion model persists and appears in `defaults.lore_complete`.
9. A Leave out entry shows no Complete button, and Ctrl/Cmd+Enter in its field sends no request; Ctrl/Cmd+Enter outside the modal still runs Continue.

## Saves

- Header status: quiet dot plus "Saved" or "Unsaved changes", 12px `--graphite`, centered above the sheet; the chapter title is not repeated. Loading and switching messages remain visible. Header and sheet stay in document flow, so scrolling cannot place the status over prose. Desktop failure messages float over the desk at the top of the header without changing the sheet's top edge or covering the chapter title, toolbar, or rails.
- Failed save or recovery failure: a floating "Not saved" pill using the shared `Alert` fail style, with `--alert` fill, white text and dot, token padding, rounded corners, and fail shadow. The unchanged detailed reason is available in the pill's `title` tooltip and visually hidden screen-reader description, announced assertively by an alert. Saving and saved states use a status. A failed delta POST keeps that pill and resumes on a later poll after history since the submitted base revision is reconciled, so an applied submission whose response was lost is not inserted twice. History gaps still stop saving. Export remains in the left drawer.

## Model selection

- Settings → AI is the one place to choose a model for each AI action (Continue and Edit). The Inspector has no model menus.
- Continue and Edit requests send no `model`. The server uses the model saved in Settings for the action, then the catalog default.
- `GET /ai/models` returns `defaults` per action, which equal the Settings model when one is saved. The Inspector's Model and cost lines use `defaults[action]` for the inspected request, so cost is known whenever the catalog loads.
- Saving AI settings fetches the catalog again, so the Inspector line follows the change.
- "Request model" appears only when the last run used a different model than the one expected now (for example Settings changed after the run).
- If the catalog cannot load, the Model line reads "unknown — unresolved model" and requests still work with server defaults.

## Shortcuts (prototype)

| Keys | Action | Status |
|------|--------|--------|
| Ctrl/Cmd+[ | Toggle manuscript panel | Implemented |
| Ctrl/Cmd+] | Toggle context inspector | Implemented |
| Ctrl/Cmd+Enter | Continue or stop AI from the manuscript, page body, or non-text control outside dialogs and popovers | Implemented |
| Ctrl/Cmd+K | Open Edit note for selected manuscript text; leave browser default when unavailable | Implemented |
| Alt+L | Open lore card for nearest in-window match | Implemented on desktop |
| Esc | Close panels, Settings, Edit note, and lore card; restore editor selection/focus | Implemented |
| Enter (in edit note) | Apply edit; Shift+Enter inserts a newline | Implemented |
| Alt+? (or ? outside text) | Toggle state switcher (idle, streaming, panel open, lore card, save stopped) | Prototype-only |
| Settings "Simulate Save stopped" | Force the red pill | Prototype-only |

## Prototype vs decisions

- The prototype's second lavender window marker is obsolete. The product uses one Continue view with a visible "from a cut-off reply" indicator; the prototype has no such indicator.
- Off-screen marker reads `Continue window / starts ↑ N pages up`, not "window starts ↑ N pages".
- AI text effect has four options, not an on/off switch.
- Post-its have no rotation, and yellow lives in the drawer rather than on the sheet.
- Lore card opens on hover after 600ms. It is a card, not a panel, so the "hover never opens panels" rule holds, but note it.
- Failed save is a loud red pill in the prototype (not faint grey), so no gap there.

## Product browser coverage

`e2e/lore.spec.ts` creates and updates a character through the left drawer, matches near the caret in the inspector, reloads persisted lore and chapter text, verifies Continue and Edit request bodies omit hidden notes, and deletes the entry. Desktop captures at 1100px and 1440px are stored in `target/guz-671/`.

The same spec checks visual-only name/alias/keyword underlines, hover cards without hidden notes, the budgeted Continue lore list, window changes, and unchanged saved chapter content/revision at 1100px and 1440px. Lore UI screenshots are stored in `target/guz-759/`.

Run with `cargo xtask e2e`. Existing `e2e/continue-stream.spec.ts` covers concurrent typing before and inside AI text, caret and selection preservation, Stop and server-error rollback that retains author inserts, server cancellation, completion tint fade, persisted tint preference, saved-selection deletion/replacement transforms, and the narrow-screen locking boundary. Existing `e2e/author-guidance.spec.ts` covers Settings persistence and idle dismissal, forced-503 save failure, accessible recovery details, export, and failure-pill geometry without sheet movement or prose overlap at 1100px and 1440px.

`e2e/desktop-edge-cases.spec.ts` adds Escape and outside-click Settings dismissal during Continue, disabled window controls, live tint changes, continued streaming and author typing after dismissal, plus transport disconnection and premature EOF after author typing inside AI text. Each case runs at 1100px and 1440px and checks saved text after reload. Recentering remains outside this coverage. `e2e/save-recovery.spec.ts` forces a chapter delta POST failure, waits for the red Not saved pill, releases the route, and checks a later poll saves text typed before and during the failure.

## Known gaps

- Prototype "Saved" label overlaps the first line of text; the product uses a separate flowing header above the sheet.
- The shortcut hint line overlaps text on partial scroll.
- Narrow screens untested; no typing test.
- Plain contenteditable, not Quill; data and streaming are faked.
- Typing during a stream is not exercised in the prototype.
