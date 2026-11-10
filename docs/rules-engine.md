# Prose Rules Engine

Design for advisory prose hints in the editor. Offsets are UTF-16 code units over Quill plain text, the same units `prose.ts` and Quill use.

## Purpose

Show the author small, local style hints (passive voice, repeated words, awkward dialogue punctuation) as an overlay on the manuscript. One engine runs many rules over a shared analysis of the text, so each new rule is a small data entry plus one pure function.

Hints are advisory:

- They never block saving.
- They never change the text (no auto-fix).
- They never enter the AI prompt, the Delta, or saved content.

## Non-goals

- No rule DSL and no user-written regex rules. The one exception is the author "watch words" list rule.
- No genre presets in v1. Config is per-rule on/off plus params.
- No grammar or style scoring, readability grades, or AI-based checks.
- No Spanish rules in v1 (see Languages).
- No spell checking. Being researched separately; it may later become a rule source. Undecided.

## Types

Sketch, not final code.

```ts
type Scope = "narration" | "dialogue" | "both";
type Language = "en" | "es";

type ParamDef =
  | { key: string; kind: "int"; label: string; default: number; min: number; max: number }
  | { key: string; kind: "words"; label: string; default: string[] }; // watch words only

type Hint = {
  ruleId: string;
  start: number;   // UTF-16 units, Quill plain text
  length: number;  // UTF-16 units
  message: string;
};

type Rule = {
  id: string;
  label: string;
  languages: Language[];
  defaultEnabled: boolean;
  params: ParamDef[];
  defaultScope: Scope;
  check(doc: ParagraphAnalysis, config: RuleConfig): Hint[];
};

type RuleConfig = {
  enabled: boolean;
  scope: Scope;                          // user-overridable, defaults to rule.defaultScope
  params: Record<string, number | string[]>;
};
```

`check()` is pure: same analysis and config in, same hints out. It reads no DOM, no clock, and no network. Hint offsets are relative to the paragraph; the engine shifts them to document offsets.

## Pipeline

1. Split the chapter plain text into paragraphs (on `\n`).
2. For each paragraph, run the shared pass once: sentences, words, and dialogue/narration spans. This yields a `ParagraphAnalysis`.
3. For each enabled rule whose `languages` include the series language, call `check()` with the analysis and that rule's config.
4. Drop hints that fall entirely outside the rule's scope; clip or drop hints that straddle a boundary (the rule declares nothing about this; the engine decides, see Open questions).
5. Shift to document offsets and hand the list to the overlay.

```ts
type ParagraphAnalysis = {
  text: string;
  sentences: { start: number; end: number }[];
  words: { start: number; end: number; text: string; lower: string; zone: "narration" | "dialogue" }[];
  dialogue: { start: number; end: number }[]; // quoted spans, quotes included
};
```

All rules consume this one structure. A rule never re-tokenizes the text.

`prose.ts` (`analyzeProse(text) -> Hint[]`, with `kind`/`index`) becomes the first rule, passive voice. Its `Hint` shape is replaced by the engine's `Hint`; the detection logic is kept.

## Dialogue and narration detection

First slice, because scope depends on it.

Approach: a single left-to-right scan of each paragraph tracking open and close quote characters.

- Straight `"` toggles in and out of dialogue.
- Curly pairs open on `“` and close on `”`.
- Text between an open and its close, quotes included, is dialogue; everything else is narration.
- An unclosed quote at paragraph end runs to the end of the paragraph. Many novels continue a speech into the next paragraph by omitting the closing quote, so a paragraph that starts with an open quote does not inherit state from the previous one. State never crosses paragraphs; this keeps the per-paragraph cache valid.
- Apostrophes and single quotes are not treated as dialogue marks.

Known limits:

- Nested quotes (a quotation inside speech, or single-quoted dialogue in British style) are not modeled. Inner marks are ignored or can flip state wrongly.
- Dialogue spanning paragraphs is only partly handled, as above.
- Spanish em-dash dialogue (`—Hola —dijo`) has no quote marks and needs its own detector. This is a known future need and part of the Spanish follow-up.
- Stray or unbalanced quotes yield wrong zones for the rest of that paragraph only.

## Shared pass as built (S1)

`app/src/md_editor/rules/shared_pass.ts`, tested in `shared_pass.test.ts`. `splitParagraphs(text)` returns `{start, text}` per paragraph; `analyzeParagraph(text)` returns the `ParagraphAnalysis` above. All offsets are UTF-16 units.

- Paragraphs: the text splits on every `\n`. Empty paragraphs are kept, including the one after a trailing `\n`, so each `start` is the exact document offset.
- Sentences: a sentence starts at the first non-whitespace character and ends after a run of `.`, `!`, `?` or `…`, plus any closing `”`, `’`, `"`, `)` or `]`, when whitespace or the text end follows. Leading and trailing whitespace is not part of a sentence. An unterminated last sentence is kept. An empty paragraph has no sentences.
- Words: runs of letters, marks and digits (`\p{L}`, `\p{M}`, `\p{N}`), joined by `'` or `’` (`Ana’s` is one word). `lower` is `toLowerCase()` only; `’` is not normalized to `'`. A word's `zone` is set by its start offset.
- Quote marks: straight `"` opens in narration and closes in dialogue. `“` opens in narration and is ignored inside dialogue. `”` closes dialogue and is ignored in narration. A straight `"` can close a span opened by `“`. An unclosed quote runs to the paragraph end. State never crosses paragraphs. `‘`, `’` and `'` are never marks.

Known limits of the built pass:

- Abbreviations such as "Mr." end a sentence.
- Spanish `«»` are not dialogue marks.
- Spanish em-dash dialogue is not detected; it is left to GUZ-1533 (see :98 and :160).

## Config storage

Per series, stored server-side like lore, not in browser storage. The series also carries its language.

```json
{
  "language": "en",
  "rules": {
    "long-sentence": { "enabled": true, "scope": "narration", "params": { "maxWords": 35 } },
    "watch-words":   { "enabled": true, "scope": "both", "params": { "words": ["suddenly"] } }
  }
}
```

Missing rules or keys fall back to the rule's defaults, so adding a rule needs no migration of existing configs. Unknown ids are ignored. Columns, routes and validation are in Storage and API, below.

## Rule catalogue

Order is by reliability.

| Tier | Rule | Default | Params | Notes |
|------|------|---------|--------|-------|
| Reliable | Dialogue punctuation | on | none | `"Hello." he said` should use a comma |
| Reliable | Redundant phrases | on | none | Fixed list: "nodded his head", "shrugged his shoulders" |
| Reliable | Repeated word | on | `window` (words) | Same word within N words; skips common function words |
| Reliable | Same sentence start | on | `count` | Several consecutive sentences starting with the same word |
| Useful | Passive voice | on | `maxAdverbs` | Existing `prose.ts` logic |
| Useful | -ly adverbs | on | none | Most useful on dialogue tags |
| Useful | Filter words | on | none | saw, felt, noticed, realized |
| Useful | Began to / started to | on | none | |
| Useful | Long sentence | on | `maxWords` | |
| Opinion | Hedge words | off | none | very, just, really, suddenly |
| Opinion | Said-bookisms | off | none | "he hissed" |
| Opinion | "There was" / "It was" openers | off | none | |
| Opinion | Cliches | off | none | Fixed list |
| Opinion | Exclamation marks | off | none | |
| Author | Watch words | off | `words` | Author's own list; the only free-form rule |

Rule lists are fixed data in the repo. Exact default numbers are set when each rule is built.

## Languages

English first. Every rule declares which languages it supports; the series language selects them. A rule that lacks the series language is not run and is hidden from the toggle UI.

Spanish is planned (the owner wants English and Spanish) and is a follow-up: it needs its own word lists, its own rule set, and the em-dash dialogue detector. Not designed here.

## Performance

- Cache the `ParagraphAnalysis` and each rule's hints per paragraph, keyed by paragraph text (plus language and the rule's config for the hint cache). Typing in one paragraph rechecks only that paragraph; others are cache hits.
- Cross-sentence rules work within a paragraph only, so they cache the same way. Cross-paragraph rules are out of scope for v1.
- Run after a short idle debounce, not on every keystroke.
- Bound the cache (for example, the last few thousand paragraphs, least recently used).
- Config changes invalidate the hint cache but not the analysis cache.
- Rules are pure, so the whole pass could later move to a worker without API change.

## Storage and API

Q1 is an owner decision. Every other point in this section is an owner decision.

Q1. Columns (Owner decision). Settings are per series: each series keeps its own config. Since `migrations/20261025000000_story_model.up.sql`, `series` is no longer a table. It is the view `legacy_series` over `containers` of kind `book_series` (`migrations/20261025000000_story_model.up.sql:222-223`, recreated with deleted filters at `migrations/20261101000002_container_trash.up.sql:17-19`); a series id is its container id. One migration adds to `containers` (`migrations/20261025000000_story_model.up.sql:7-13`):

- `language TEXT NOT NULL DEFAULT 'en' CHECK (language IN ('en', 'es'))`.
- `rules_config TEXT NOT NULL DEFAULT '{}'`, a JSON object keyed by rule id, each value `{enabled?, scope?, params?}` as in Config storage. `language` lives in its own column, not in the JSON.

Only `book_series` rows are read or written; the columns keep their defaults on `tv_series` and `collection` rows. Existing rows get the defaults, so no data migration. The down migration drops both columns.

Q2. Routes (Owner decision):

- `GET /series/{id}/rules` returns `{language, rules}`. `rules` is the stored object as saved, not merged with defaults; the client merges with the catalogue. Missing or deleted series: 404 `NOT_FOUND`.
- `PUT /series/{id}/rules` takes `{language?, rules?}`. `rules` replaces the whole stored object; a partial body keeps the omitted field. Returns the same shape as GET (200).
- The handler resolves the series id to its container row through `legacy_series`. A missing id, a deleted container or universe, or a container that is not `book_series` is 404 `NOT_FOUND`.

Validation against the catalogue (Owner decision). The server holds the rule catalogue as data: id, languages, scope list, and each param's kind, min and max. It runs no rule.

- Malformed JSON body: 400 `INVALID_JSON` (`crates/api/src/error.rs:37`, `:51`).
- `language` not `en` or `es`; `scope` not `narration`, `dialogue` or `both`; `enabled` not a boolean; an `int` param out of `min..max` or not an integer; a `words` param not an array of nonblank strings: 422 `VALIDATION_ERROR` (`crates/api/src/error.rs:36`, `:50`).
- Unknown rule ids and unknown param keys are kept, not rejected, so an older server never drops config from a newer client. The engine ignores them (Config storage, above).

## Settings toggles

Q8 (Owner decision). The desktop Settings popover (`docs/editor-design.md:78`) gets a Hints section for the series of the open chapter:

- Language select (English, Spanish) and one checkbox per rule that supports that language, in catalogue order. Rules for the other language are hidden (Languages, above).
- `int` params show as a number field; the watch-words list shows as a text field, one word per line.
- Each change saves with `PUT /series/{id}/rules`; a failed save shows the error and restores the last saved value. The preference is not in local storage.
- Scope override has no control in v1; it stays at the rule default unless set through the API.
- Below 1100px there is no Settings popover, so there are no toggles; the saved config still applies.
- A piece outside a `book_series` container has no series, so the section is absent and no rules run.

## Scope of GUZ-691 and GUZ-690

Q3 (Owner decision). GUZ-691 ships the engine and the toggles only, with no visible hint. The overlay (drawn from `quill.getBounds`, as in `app/src/md_editor/components/ContinueWindow.tsx:19-20`) and the hover/click tip belong to GUZ-690.

## Overlay and tip (GUZ-690)

Owner decision: passive voice stays in GUZ-692 (Delivery, GUZ-692 bullet). GUZ-690 ships the overlay and the tip only, tested with a test rule. The GUZ-690 E2E spec waits for the first real rule.

Owner decision: rule labels, param labels and hint messages are i18n message keys (en and es in `app/src/i18n/messages.ts`, where `settings.hints.*` lives at `:300-303` and `:679-682`), not plain strings. The type change, built before GUZ-692 adds rules:

- `Rule.label` becomes `labelKey`; `ParamDef.label` becomes `labelKey`; `Hint.message` becomes `messageKey` (`engine.ts:11`, `:8-9`, `:17`).
- `HintSettings.tsx:50-53` renders `t(rule.labelKey)` and `t(param.labelKey)`.
- Both locales get every key.

### Piece to series

Pieces outside a chapter scope get rules too. Chapter scopes already carry `seriesId` (`Editor.tsx:1493`). A `PieceScope` is `{universeId, pieceId}` (`contract.ts:27`) and is not widened. A hook resolves `pieceId -> seriesId | null` from existing calls only, no new API:

1. `getPiece({pieceId})` (`client.ts:166`) returns `containerId` (`contract.ts:39`).
2. A child of a group has `containerId` null (children attach only with it cleared, `docs/agent/api-routes.md:59`, `:61`), so use `parentId` and call `getPiece` on the group once for its `containerId`.
3. `listContainers({universeId})` (`client.ts:163`) returns `kind` (`contract.ts:31`). The series id is the container id when its kind is `book_series` (`:175`); any other result is `null`.

`null` means no Hints section and no rules (`:203`). `loadPiece` (`chapter_session.ts:22-27`) keeps dropping the container; the resolver, not the scope, owns the lookup. Script groups (`scriptGroupId`, `Editor.tsx:433`) are out of scope: prose only. Inferred from the existing chapter path, not run: the resolved id feeds `HintSettings` and the hint runner exactly as `chapterSync.scope.seriesId` does now.

### Proposed answers (owner review)

Each answer is the narrowest reading consistent with current editor code.

- a. Tip trigger. Proposed (owner review): pointer rest on a hint opens the tip; a tap or click toggles it. Keyboard: a collapsed caret inside a hint opens it (selection changes, `Editor.tsx:267-269`); Escape closes it. The overlay keeps `pointer-events: none` like the continue window (`ContinueWindow.tsx:41`, `index.css:517-527`), so hit-testing is done on `quill.root` pointer events against hint rects. Rail buttons use `mousedown preventDefault` so they never steal the caret (`editor-design.md:75`); the tip does the same. Content: rule label (`labelKey`) and `messageKey` text. No "ignore" button (Q6). Any new `app/src/ui/` primitive gets its story first (`CLAUDE.md`); prefer the existing `Popover` (`app/src/ui/Popover.tsx`).
- b. Debounce. Proposed (owner review): 400 ms idle after the last user `TEXT_CHANGE`, per `:166`. No debounce exists in `Editor.tsx` to reuse (only the tint timer, `:323`), so the hook owns one timer. Config changes recompute at once.
- c. Multi-line hints. Proposed (owner review): `getBounds(index, length)` returns one rect (`ContinueWindow.tsx:19-20` uses it for two points only). Walk the hint's words, call `getBounds` per word, and start a new segment whenever `top` changes; draw one underline per line segment. Only hints inside the viewport are measured. Measured on scroll, resize and `ResizeObserver`, as in `ContinueWindow.tsx:27-30`.
- d. Continue, AI-streamed and busy text. Proposed (owner review): hints recompute only from `source === "user"` text changes (`Editor.tsx:262`). While `isAiActive()` (`Editor.tsx:404`) there is no recompute; one runs when it ends. Until `chapterSync.ready`, nothing is drawn. AI-inserted text gets hints after that one pass, like any text.
- e. Piece language. Proposed (owner review): the language is the resolved series' language from `GET /series/{id}/rules` (`:184`), the same read `HintSettings` makes; no series, no rules.
- f. Selection and IME. Proposed (owner review): a non-collapsed selection does not hide hints and does not open the tip. While Quill reports `composition.isComposing` (used at `dialogue_typing.dom.test.tsx:120`), no recompute and no tip; one recompute runs when composition ends.
- g. Layering. Proposed (owner review): the overlay is a sibling in `quill.container`, never in the Delta, and sits at z-index 4 (`editor-design.md:86`, "hint 4"); no `--z-hint` token exists yet (`ui/tokens.css:30-36`), so S2 adds one. AI tint is only a CSS colour on `.ql-editor[data-ai-tint]` (`index.css:913-920`) and does not move text, so the two do not collide. The continue marker (`.editor-continue-window`, no z-index) sits at `left: -24px` outside the text; the underline stays on text.

Below 1100px (`Editor.tsx:319`, `:544`) there is no Settings popover and no toggles (`:202`), but the saved config still applies, so the overlay and tip draw there too. Proposed (owner review): same rules, tip opens by tap or caret; no drawer interaction.

## Delivery

- GUZ-691: engine, shared pass with dialogue detection, per-series config and storage, rule toggle UI.
- GUZ-692: first rule batch, starting with the reliable tier and passive voice ported from `prose.ts` (`analyzeProse`, `app/src/md_editor/prose.ts:302`).
- GUZ-690: the hint overlay and tip wait for the engine and are built on it, not as a passive-only overlay.
- GUZ-1533: Spanish em-dash detector and further dialogue rules.

Stages for GUZ-691, each under about 400 changed lines, red test first:

| Stage | Content | Estimate |
|-------|---------|----------|
| S1 | Shared pass: paragraphs, sentences, words, quote dialogue zones; Bun tests | ~250 lines |
| S2 | Engine: rule registry, language and scope filter, offset shift, LRU caches; one test rule | ~300 lines |
| S3 | Migration, catalogue data, `GET`/`PUT /series/{id}/rules`, Rust integration tests | ~350 lines |
| S4 | Settings Hints section (story first for any new `app/src/ui/` primitive), client, DOM tests, E2E spec written, not run | ~350 lines |
| GUZ-690 | Overlay and tip, E2E at 1100px and 1440px | separate ticket, S1-S5 below |

Stages for GUZ-690, each under about 400 changed lines, red test first. Passive voice is not in this ticket.

| Stage | Content | Estimate |
|-------|---------|----------|
| S1 | Message-key type change (`labelKey`, `messageKey`), en and es keys, `HintSettings` update; hint runner hook: debounce, user-source filter, `engine.run`, series config read. Red: Bun test with a test rule | ~300 lines |
| S2 | Overlay component and `--z-hint`: per-line `getBounds` rects, scroll/resize/`ResizeObserver`. Red: DOM test for rects, resize, and an unchanged Delta (no `formatText`) | ~300 lines |
| S3 | Tip: hover, tap, caret and Escape, `preventDefault` on mousedown. Red: DOM test | ~250 lines |
| S4 | Piece to series resolver, `Editor.tsx` wiring, `HintSettings` for pieces. Red: resolver test (leaf, group child, non-`book_series`) | ~250 lines |
| S5 | Persistence and AI-request test: no hint data in `quillai.editor.state.v1` or the built request. E2E spec at 1100px and 1440px, written not run, blocked on the first real rule from GUZ-692 | ~200 lines |

## Settled questions

Each answer is an owner decision.

- Q4. A hint that crosses a dialogue/narration boundary is kept when its start is in the rule's scope, and dropped otherwise. It is not clipped.
- Q5. One neutral underline for every tier in v1. Tier colors wait for author feedback.
- Q6. No per-hint "ignore this one" in v1. Turning a rule off is the only control.
- Q7. No cross-paragraph rules in v1. The per-paragraph cache stays exact.

## Open questions

- Spell checking: possible rule source, undecided.
