# Editor Design (v1 desktop)

Approved visual and behavior spec. Visual reference: `prototypes/editor-desktop/` (`index.html`, `style.css`, `proto.js`; plain contenteditable, fake data). Values below were read from `style.css`. Where the prototype differs from the decisions, see "Prototype vs decisions".

## Scope

- Desktop first: 1100px and wider. Narrow/mobile layout is undecided and untested.
- Existing `context_inspector` (GUZ-664/665/666) becomes the right-hand drawer.

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
| `--pi-lav` | `#E0D8EC` | Flag: Resume window; lore hover |
| `--st-blue` | `#7C9BB0` | Continue bracket stroke |
| `--st-lav` | `#9B8BB8` | Resume bracket, lore underline |
| `--st-tan` | `#B79A6C` | Lore "don't reveal" underline |
| `--sheet-shadow` | `0 1px 2px rgba(31,41,51,.06), 0 10px 34px rgba(31,41,51,.09)` | Sheet |
| `--note-shadow` | `0 1px 1px rgba(31,41,51,.06), 0 6px 14px -6px rgba(31,41,51,.22)` | Post-its, flags |
| Fail shadow | `0 4px 18px rgba(180,65,47,.35)` | Red save pill |

Also used: panel shadow `0 0 40px rgba(31,41,51,.12)`; popovers `0 8px 30px rgba(31,41,51,.12)`; edit box `0 10px 30px rgba(31,41,51,.13)`; lore card `0 10px 30px rgba(31,41,51,.14)`; sheet edge `1px solid rgba(31,41,51,.07)`.

## Type

| Role | Value |
|------|-------|
| Body (sheet) | Georgia, Iowan Old Style, Palatino; 21px / 1.75 |
| Chapter title | sans 600 13px, 0.22em tracking, uppercase, `--slate`, centered |
| UI | system sans (-apple-system, Segoe UI, Helvetica) |
| Panel/popover headings | sans 600 12px, 0.2em tracking, uppercase, `--slate` |
| Inspector text | mono 12px / 1.6, `--graphite` |

## Writing surface

- Centered sheet: `width: min(760px, 100vw - 400px)`; padding `72px 84px`; radius `2px 2px 0 0`; visible hairline edge plus `--sheet-shadow` on `--desk`.
- Infinite page: wrapper has `50vh` top margin and the page has `50vh - 14px` bottom padding, so any line can sit mid-screen. The sheet bottom border is removed and never shown.
- Scrollbar hidden; scroll container is full viewport.
- Caret color `--slate`.

## Caret and scroll

- Recenter the caret vertically only on typing (`input` event); not on click or arrow-key moves.
- Dead zone 80px around vertical center; smooth scroll when exceeded.
- Settings: Recenter = while typing | never.
- While AI streams, the caret does not move and the author can keep typing. Streamed text is inserted as a new paragraph after the caret paragraph and grows below.

## AI text

- While streaming: inked in `--slate` (tint).
- When streaming ends: tint transitions to plain ink over 1.8s (`.old` class added 250ms after the end).
- Settings: AI text effect. Prototype options: none | tinted ink | soft highlight (`--pi-green`) | fade-in (1.2s). v1 decision is tint on/off; extra options are optional.

## Tools

- Left rail (56px): navigation (manuscript tree panel, 300px, opens at `left: 56px`).
- Right rail (56px): Continue, Edit selection, Context inspector, Settings (gear pinned bottom). Inspector drawer 300px at `right: 56px`.
- Rails are muted: opacity .35, .95 on hover/focus-within; buttons 36px, radius 10px, `--graphite`.
- Hover = more contrast (`--ink`), `--pi-pink` background, `translateY(-1px)`. Hover never opens or closes a panel. Panels toggle by key or click; Esc closes all.
- Rail buttons use `mousedown preventDefault` so they do not steal caret or selection.
- Z-order: hint 4, flags 5, panels 8, rails 9, settings popover 9, switcher/edit box 10, lore card 11, save bar 20.

## Sticky notes (styled textareas)

Common: `.postit` textarea, full width, vertical resize, no border, radius 1px, padding `14px 12px 10px`, sans 13px / 1.5, `--ink` text, `--note-shadow`, top "adhesive" gradient `rgba(31,41,51,.045)` to transparent over 12px, focus ring `0 0 0 2px rgba(78,98,114,.35)`. Placeholder `#6b6f73`. Prototype notes are flat: no rotation. Any rotation is a design addition, not in the prototype; if added, keep it under about 1deg.

| Note | Fill | Appears | Where |
|------|------|---------|-------|
| Yellow, author guidance | `#F4E8B5` | Inspector drawer, always, 4 rows | Section "Author guidance" |
| Pink, Edit instruction | `#F3D6D8` | Edit click with a non-empty selection; Enter applies, Esc cancels | Edit box, right 72px, top 96px, 280px wide |
| Green, lore note | `#D7E6D0` | In the lore card, 2 rows, per-match note for the AI | Lore card |

Arrow-shaped flag tabs (`clip-path`, 11px sans, `--note-shadow`) label the edit box ("Edit selection", pink), the lore card ("Lore match", lavender), and the window markers.

## Context window

- Brackets in the left gutter mark what a completion sends: 9px wide, 2px stroke, opacity .6; Continue at `left: -24px` (blue), Resume at `left: -38px` (lavender). Bracket runs from window start to the caret.
- When the window start is off-screen, a flag sits at the top of the viewport reading `starts ↑ N pages up` (or "just above"); clicking scrolls to the start.
- Settings: Show Continue window, Show Resume window (checkboxes).

## Lore

- Names inside the displayed window get a dotted lavender underline; hover shows `--pi-lav` background. Hover for 600ms (or Alt+L for the nearest name) opens a 300px lore card; Esc closes.
- Card: name, matched alias, "In AI requests" radios (Include | Known, don't reveal | Leave out), "Lore as of" book and chapter selects (story-point level gating what enters the completion), green note, "Ignore this match".
- Underline style per level: include dotted lavender; known-don't-reveal dashed tan; leave out dotted pencil.
- Inspector section "Lore in window": list of names with type, match count, and level; click scrolls to the first match and opens its card.

## Continue

- One Continue control (right rail) with a "with thread" indicator in place of a separate Resume. Resume is the implicit thread-history variant (`Editor.tsx:502-506`).
- Click while streaming stops the stream.

## Saves

- Quiet status at top center: dot plus "Saved" / "Saving…", 12px `--graphite`, opacity .7.
- Failed: red pill, `--alert` fill, white 13px text, "Save stopped" bold plus "latest changes are not saved, retrying", 8px white dot, padding `8px 18px`, full radius, fail shadow, opacity 1.

## Shortcuts (prototype)

| Keys | Action | Status |
|------|--------|--------|
| Ctrl/Cmd+[ | Toggle manuscript panel | Intended for product |
| Ctrl/Cmd+] | Toggle context inspector | Intended for product |
| Alt+L | Open lore card for nearest in-window name | Intended for product |
| Esc | Close panels, popovers, cards; refocus the page | Intended for product |
| Enter (in edit note) | Apply edit | Intended for product |
| Alt+? (or ? outside text) | Toggle state switcher (idle, streaming, panel open, lore card, save stopped) | Prototype-only |
| Settings "Simulate Save stopped" | Force the red pill | Prototype-only |

## Prototype vs decisions

- Resume is still a separate second window, bracket and flag. The decision folds it into Continue with a "with thread" indicator; the prototype has no such indicator.
- Off-screen marker reads `Continue window / starts ↑ N pages up`, not "window starts ↑ N pages".
- AI text effect has four options, not an on/off switch.
- Post-its have no rotation, and yellow lives in the drawer rather than on the sheet.
- Lore card opens on hover after 600ms. It is a card, not a panel, so the "hover never opens panels" rule holds, but note it.
- Failed save is a loud red pill in the prototype (not faint grey), so no gap there.

## Known gaps

- "Saved" label overlaps the first line of text.
- The shortcut hint line overlaps text on partial scroll.
- Narrow screens untested; no typing test.
- Plain contenteditable, not Quill; data and streaming are faked.
- Typing during a stream is not exercised in the prototype.
