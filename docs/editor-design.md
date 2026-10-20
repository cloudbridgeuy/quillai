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
| `--tan` | `#C4A982` | Declared, unused in the prototype |
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

- At 1100px and wider, Continue leaves the editor writable. Its output starts as a new paragraph after the original caret paragraph, not at a later caret location. Every author change transforms the insertion anchor, append end, and exact surviving AI-owned ranges with `quill-delta`. Later fragments append at that transformed end.
- Stream insertion, completion, and rollback preserve the logical caret or selection, including selections after the AI paragraph. No recentering or stream-driven scrolling is added. Further smooth-writing work remains in GUZ-763.
- Desktop Continue Stop, server cancellation, and failures delete only surviving AI-owned text, including the inserted paragraph separator. Author inserts inside that text survive. Edit selection and screens below 1100px retain their existing locked-editor behavior.

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
- Product rails provide manuscript, Continue/Stop, Edit selection, context inspector, and a bottom-pinned desktop Settings gear.
- The right drawer holds Story Context, Instruction, and the AI Thread bar with Reset Thread beside the live Continue context. Existing Max Output Tokens and Thread continuation prompt controls live under Advanced AI controls in this drawer. There is no Preview Context button or separate preview modal.
- At 1100px and wider, Settings opens a 280px popover at right 72px, bottom 18px, outside the prose. It holds Window Size, Whole document, and Tint AI text, using the existing persisted local preferences. Window controls remain disabled during AI runs. Opening it closes either drawer; opening a drawer closes Settings. Click outside dismisses it; Escape and Close settings restore the remembered editor selection/focus. Narrow screens retain the window controls in the context drawer. Prototype-only recentering, extra text effects, window markers, and save-failure simulation are not exposed.
- Export local chapter is in the left manuscript drawer. It remains available after a save failure.
- Both drawers start closed. Only one can be open. Bracket shortcuts toggle their drawer; Escape and drawer Close restore the remembered editor selection and focus. Hover changes only appearance.
- Drawers and all descendants have square corners. Drawers use `--sheet-shadow`; buttons and the request picker stay flat, with no shadow or hover lift. Section boxes remain square. Below 1100px, drawers remain in document flow.
- The manuscript drawer loads series → books → chapter summaries through the existing manuscript client, with loading, empty, error, and retry states. Selecting a chapter waits for the current save and any pending edits, then fetches that chapter once through the single-chapter GET. Editing and further switches pause until loading completes. Only a successful load changes the title, selected highlight, URL, and AI context. Save/load failures keep the current chapter and its text, show an error, and preserve recovery/export. Reopening refreshes the hierarchy.
- Z-order: hint 4, flags 5, panels 8, rails 9, settings popover 9, switcher/edit box 10, lore card 11, save bar 20.

## Sticky notes (styled textareas)

Common: `.postit` textarea, full width, vertical resize, no border, radius 1px, padding `14px 12px 10px`, sans 13px / 1.5, `--ink` text, `--note-shadow`, top "adhesive" gradient `rgba(31,41,51,.045)` to transparent over 12px, focus ring `0 0 0 2px rgba(78,98,114,.35)`. Placeholder `#6b6f73`. Prototype notes are flat: no rotation. Any rotation is a design addition, not in the prototype; if added, keep it under about 1deg.

Author guidance uses the yellow post-it styling above, with owner-approved square corners and `--sheet-shadow` instead of the common radius and note shadow. Its focus ring is added to the sheet shadow; the yellow fill remains on focus.

The shared `StickyNote` textarea provides both yellow guidance and pink Edit input. Guidance still saves only with Save author guidance. The right-rail Edit selection control opens the focused pink note without sending a request; there is no middle Edit button. Enter and Apply use the same selection, busy, chapter-ready, and blocked-save checks before invoking the existing Edit stream. Shift+Enter inserts a newline, and Esc cancels without changing manuscript text and restores the editor selection. Composing text does not submit. The note edits the existing shared Instruction value used by Continue and Edit.

| Note | Fill | Appears | Where |
|------|------|---------|-------|
| Yellow, author guidance | `#F4E8B5` | Inspector drawer, always, 5 rows | Section "Author guidance" |
| Pink, Edit instruction | `#F3D6D8` | Edit click with a non-empty selection; Enter applies, Esc cancels | Edit box, right 72px, top 96px, 280px wide |
| Green, lore note | `#D7E6D0` | In the lore card, 2 rows, per-match note for the AI | Lore card |

Arrow-shaped flag tabs (`clip-path`, 11px sans, `--note-shadow`) label the edit box ("Edit selection", pink), the lore card ("Lore match", lavender), and the window markers.

## Context window

- One blue Continue bracket in the left gutter marks the document slice sent: 9px wide, 2px stroke, opacity .6, at `left: -24px`. It runs from window start to the caret. Saved thread history does not create a second document window.
- When the window start is off-screen, a flag sits at the top of the viewport reading `starts ↑ N pages up` (or "just above"); clicking scrolls to the start.
- Intended setting: Show Continue window. Window markers remain prototype-only.

## Lore

- Names inside the displayed window get a dotted lavender underline; hover shows `--pi-lav` background. Hover for 600ms (or Alt+L for the nearest name) opens a 300px lore card; Esc closes.
- Card: name, matched alias, "In AI requests" radios (Include | Known, don't reveal | Leave out), "Lore as of" book and chapter selects (story-point level gating what enters the completion), green note, "Ignore this match".
- Underline style per level: include dotted lavender; known-don't-reveal dashed tan; leave out dotted pencil.
- Inspector section "Lore in window": list of names with type, match count, and level; click scrolls to the first match and opens its card.

## Continue

- One Continue control in the right rail; no separate Resume or inline Continue button. Its visible "with thread" indicator appears whenever the next request includes history, including inspector history overrides. Clearing history overrides removes the indicator; instruction changes, thread reset, chapter changes, and cancellation use the existing thread lifecycle.
- While streaming the same control shows Stop. Desktop Continue rolls back only surviving AI-owned text. Edit and narrow-screen Continue keep their previous cancellation behavior.
- The context drawer has one Continue view, labeled "Continue · with thread" when history is included, otherwise "Continue". Edit selection remains a separate request view. Prompt text and payload assembly are unchanged.

## Saves

- Header status: quiet dot plus "Saved" or "Unsaved changes", 12px `--graphite`, centered above the sheet; the chapter title is not repeated. Loading and switching messages remain visible. Header and sheet stay in document flow, so scrolling cannot place the status over prose. Failure messages take a separate full-width header row and push the sheet down.
- Failed save: clearly visible red label with `--alert` fill, white text and dot, `8px 18px` padding, rounded corners, and fail shadow. The full failure/recovery message stays visible and is announced assertively. Existing uncertain-submission failures stop saving; the UI must not promise automatic retry. Export remains in the left drawer.

## Shortcuts (prototype)

| Keys | Action | Status |
|------|--------|--------|
| Ctrl/Cmd+[ | Toggle manuscript panel | Implemented |
| Ctrl/Cmd+] | Toggle context inspector | Implemented |
| Alt+L | Open lore card for nearest in-window name | Intended for product |
| Esc | Close panels, Settings, and Edit note; restore editor selection/focus | Implemented; lore remains future work |
| Enter (in edit note) | Apply edit; Shift+Enter inserts a newline | Implemented |
| Alt+? (or ? outside text) | Toggle state switcher (idle, streaming, panel open, lore card, save stopped) | Prototype-only |
| Settings "Simulate Save stopped" | Force the red pill | Prototype-only |

## Prototype vs decisions

- The prototype's second lavender window marker is obsolete. The product uses one Continue view with a visible "with thread" indicator; the prototype has no such indicator.
- Off-screen marker reads `Continue window / starts ↑ N pages up`, not "window starts ↑ N pages".
- AI text effect has four options, not an on/off switch.
- Post-its have no rotation, and yellow lives in the drawer rather than on the sheet.
- Lore card opens on hover after 600ms. It is a card, not a panel, so the "hover never opens panels" rule holds, but note it.
- Failed save is a loud red pill in the prototype (not faint grey), so no gap there.

## Known gaps

- Prototype "Saved" label overlaps the first line of text; the product uses a separate flowing header above the sheet.
- The shortcut hint line overlaps text on partial scroll.
- Narrow screens untested; no typing test.
- Plain contenteditable, not Quill; data and streaming are faked.
- Typing during a stream is not exercised in the prototype.
