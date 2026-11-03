# Design System

One token set and one primitive set for all QuillAI UI. Read this page before you add or change UI. Layout, sizes and behavior of each surface: `docs/editor-design.md`. Visual reference: `prototypes/editor-desktop/` (white paper palette).

## Rules

- Controls, headings and text styles come from `app/src/ui/` primitives. Do not write a raw `<button>`, `<input>`, `<textarea>` or `<select>` outside `app/src/ui/`.
- Colours, shadows, type sizes, radii and spacing come from `app/src/ui/tokens.css`. Do not write a hex colour or `font-size:` in another `.css` file, and do not use Tailwind arbitrary values such as `w-[262px]`.
- One button family: `Button` (quiet, primary, danger, link). Only the `Button` link variant may use a link-style button. No new pill or one-off button style.
- Sticky notes are yellow. Green is for lore fields and success notices. Pink notes are for the Edit instruction only. Notes have no border and use a paper shadow. Control hover states also use pink.
- Lore editing opens in a `Modal`. The modal has no Cancel; Escape and backdrop click discard the draft. Delete is red (danger) and asks for confirmation: the first click arms it ("Delete lore"), the second click ("Confirm delete") deletes.
- Popovers close from their trigger (toggle), click outside and Escape. Escape returns focus to the trigger, or to `returnFocusRef` when supplied. Outside clicks keep their native focus behavior. Canceled Escape leaves focus unchanged. `ignoreRefs` excludes named DOM areas from outside dismissal without blocking their native pointer action. Controlled anchored popovers may omit the trigger for hover or keyboard openings and leave focus unchanged on dismissal; unanchored popovers require a button trigger. They have no Close button.
- Drawers have no Close button. Their rail toggle, bracket shortcut and Escape close them.

## Enforcement

`cargo xtask lint` check `ui` (`xtask/src/check_ui.rs`) scans tracked files under `app/src/`:

| Rule | Fails on | Scope |
|------|----------|-------|
| `raw-button`, `raw-input`, `raw-textarea`, `raw-select` | the raw tag | `.tsx` outside `app/src/ui/` |
| `tailwind-bracket` | an arbitrary value, e.g. `w-[262px]` | `.tsx` outside `app/src/ui/` |
| `hex-colour` | `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` | `.css`, `.ts`, `.tsx` |
| `font-size` | a `font-size:` declaration | `.css` |
| `link-button` | an anchor whose `class` or `className` contains `button` | `.tsx` |
| `inline-style` | a `style={...}` attribute | `.tsx` |
| `colour-fn` | `rgb()`, `rgba()`, `hsl()`, `hsla()`, `color-mix()` | `.css`, `.tsx` |
| `font-shorthand` | `font:` with a numeric size in px, rem, em or % | `.css` |
| `raw-radius`, `raw-shadow`, `raw-z-index`, `raw-letter-spacing` | radius, box shadow, z-index or letter spacing values other than `var(...)`, `0`, `none`, `normal` | `.css` |
| `raw-spacing` | padding, margin or gap with numeric px, rem or em values | `.css` |
| `click-non-interactive` | `onClick` on a `div`, `span`, `li`, `p` or `section` opening tag | `.tsx` |

All rules skip `*.test.*` files. TS and TSX checks skip `app/src/ui/`; CSS checks skip `app/src/ui/tokens.css`. These are text counters, not parsers; unusual syntax can escape detection or produce false positives.

`app/src/ui/ALLOWLIST` must be empty. Any entry fails lint.

## Tokens

`app/src/ui/tokens.css` defines the CSS variables and maps them to Tailwind 4 utilities with `@theme inline`. Tailwind's default colours, shadows, text sizes and radii are removed. Other default scales remain available.

| Group | CSS variable → utility | Values |
|-------|------------------------|--------|
| Colour | `--paper` → `bg-paper`, `text-paper`, … | paper `#FFFFFF`, desk `#F1EEE7`, ink `#1F2933`, slate `#4E6272`, graphite `#6F757C`, pencil `#A9ADB1`, rule `#EDE3D3`, alert `#B4412F` |
| Post-it | `--pi-*` → `bg-pi-yellow`, … | yellow `#F4E8B5`, pink `#F3D6D8`, green `#D7E6D0`, blue `#D2E2EC`, lav `#E0D8EC` |
| Stroke | `--st-*` → `border-st-blue`, … | blue `#7C9BB0`, lav `#9B8BB8`, tan `#B79A6C` |
| Placeholder | `--placeholder` → `text-placeholder` | `#6b6f73` |
| Surface effects | `--drawer-fade`, `--note-gradient`, `--note-focus-ring` | drawer fade, note gradient and focus ring; see `tokens.css` |
| Shadow | `--sheet-shadow` → `shadow-sheet`; also note, fail, panel, popover, edit, lore | see `tokens.css` |
| Type | `text-sheet`, `text-title`, `text-ui`, `text-label`, `text-meta`, `text-micro` | 21/1.75 (sheet, serif), 16/1.5 (app label), 13/1.5 (UI body, controls, chapter title), 12/1.4 (labels, panel headings 0.2em uppercase, inspector mono), 11/1.3 (flags), 10/1.3 (Continue label) |
| Radius | `rounded-none`, `-note`, `-sheet`, `-card`, `-button`, `-icon`, `-pill` | 0 (drawers, guidance), 1px (notes), 2px (sheet), 4px (edit box, lore card), 7px (buttons outside drawers), 10px (rail icons), 999px (pill) |
| Spacing | `p-1` … `p-6`, `gap-*`, `m-*` | 4, 8, 12, 16, 20, 24px (`--spacing: 4px`; use steps 1–6) |

Stacking uses `--z-drawer` (8), `--z-rail` (9), `--z-popover` (10), `--z-lore` (11), `--z-flag` (15), `--z-topbar` (20), and `--z-modal` (30). Native dialogs use the browser top layer.

Lint permits the registered animation colour initial value and two exact Continue window inline styles that track measured geometry. Large layout dimensions use multiples of the spacing unit to preserve the desktop sheet and drawer geometry.

Font stacks stay as `--serif`, `--sans` and `--mono`.

## Primitives (`app/src/ui/`)

Built: `Button`, `ButtonLink`, `IconButton`, `TextField`, `Toggle`, `Field`, `Heading`, `Alert`, `StickyNote`, `ListButton`, `Modal`, `Popover` (one `.tsx` per primitive; each exports a pure `*ClassName` function except `ButtonLink`, which shares `buttonClassName`; tests in `primitives.test.tsx`). Component page for visual review: `e2e/fixtures/ui-components/` via `e2e/ui-components.spec.ts`, screenshots in `target/guz-874/<browser>/` and `target/guz-884/<browser>/`. `ListButton` `tree` is a borderless full-width row with pink hover and blue pressed; `lore` is a bordered paper row with lavender hover and a two-column grid. `TextField` also takes `square` for radius 0 in drawers. `StickyNote` `overhang` widens the note by 4px each side; `readOnly` removes the resize handle. `tracking-section` (0.2em) is a token for section headings.

Manuscript tree rows show a small Delete `IconButton` on hover or keyboard focus. Deletion is immediate; an `Alert` offers Undo and Recently deleted.

| Primitive | Props | Replaces |
|-----------|-------|----------|
| `Button` | `variant: "quiet" \| "primary" \| "danger" \| "link"`, `size: "sm" \| "md"`, native button props. Danger is `--alert` fill with paper text; link is inline underlined text without button sizing, for an inline action inside an Alert or text. Rail icons lift on hover and focus; buttons do not lift | pill, small, primary and danger button classes |
| `ButtonLink` | Button variants and sizes, native anchor props | download links styled as buttons |
| `IconButton` | `label`, `pressed`, `glyph`, `badge?`, `kind?: "continue"`. Native `aria-label` overrides `label`; Continue shows `label` then `badge`. 36px, radius 10; Continue is the 56px stack | rail buttons |
| `TextField` | `as: "input" \| "select" \| "textarea"`, native props forwarded. Bordered; radius 0 in drawers, 4px elsewhere | `.editor-input` |
| `Toggle` | checkbox plus label | `.editor-toggle` |
| `Field` | `label`, control child. Label uses the 12px graphite style | `.editor-field` |
| `Heading` | `level: "app" \| "section"`, optional `as: "h1" \| "h2" \| "h3" \| "h4"`. App 16px; section 12px uppercase | clamp `h1`, drawer and inspector headings |
| `Alert` | `tone: "error" \| "warning" \| "success" \| "info"`, `variant: "pill" \| "inline"` (default pill), optional `onDismiss` and role override. Error defaults to `alert`; other tones default to `status`. Pills use alert, pi-yellow, pi-green or pi-blue fills; inline errors use red alert text, other inline tones use ink text, and all inline tones use a coloured dot marker. | save pill, inline notices |
| `StickyNote` | `tone: "yellow" \| "pink" \| "green"`, `overhang?`, `readOnly?`, native textarea props | `ui/StickyNote.tsx` (replaces `components/StickyNote.tsx`), green lore text |
| `Modal` | `title`, `onClose`, children, optional `footer` and stable `returnFocusRef` for the opener. Without a ref, focus returns to the active element captured on mount, except `body`. Native `<dialog>`; footer holds action buttons only; no Cancel prop | native dialog surfaces |
| `Popover` | `open`, `onOpenChange`, children; button `trigger` required unless `anchor: { left, top }` is set, where controlled triggerless opening is supported. Optional `returnFocusRef` overrides the trigger for focus return; `ignoreRefs` excludes DOM areas from outside dismissal. No Close prop | Settings, Edit box, Lore match |
| `ListButton` | `pressed`, `tone: "tree" \| "lore"` | manuscript tree rows, lore-window rows |
