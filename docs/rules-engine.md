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
4. Drop hints that fall entirely outside the rule's scope; clip or drop hints that straddle a boundary (the rule declares nothing about this; settled in Q4, Settled questions).
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
- Spanish em-dash dialogue (`—Hola —dijo`) has no quote marks and needs its own detector. Specified in GUZ-1533 dialogue rules, below.
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
- Spanish em-dash dialogue is not detected; it is left to GUZ-1533 (Known limits, above, and GUZ-1533 dialogue rules, below).

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
| Useful | -ly adverbs | on | `max` (-ly adverbs a sentence may hold; default 1) | Most useful on dialogue tags |
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

Spanish is planned (the owner wants English and Spanish) and is a follow-up: it needs its own word lists and its own rule set. The em-dash dialogue detector and the first Spanish dialogue rules are in GUZ-1533 dialogue rules, below.

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
- Owner decision (GUZ-692): `Hint` gains optional `messageArgs?: FormatValues` (`app/src/i18n/messages.ts:786`). `HintTip` passes it to `formatMessage(locale, key, values)` (`messages.ts:854-857`) in place of the bare `t(key)` (`HintTip.tsx:14`, `:77`). `Rule.check` sets it when the message names a word or number.

### Piece to series

Pieces outside a chapter scope get rules too. Chapter scopes already carry `seriesId` (`Editor.tsx:1493`). A `PieceScope` is `{universeId, pieceId}` (`contract.ts:27`) and is not widened. A hook resolves `pieceId -> seriesId | null` from existing calls only, no new API:

1. `getPiece({pieceId})` (`client.ts:166`) returns `containerId` (`contract.ts:39`).
2. A child of a group has `containerId` null (children attach only with it cleared, `docs/agent/api-routes.md:59`, `:61`), so use `parentId` and call `getPiece` on the group once for its `containerId`.
3. `listContainers({universeId})` (`client.ts:163`) returns `kind` (`contract.ts:31`). The series id is the container id when its kind is `book_series` (`:175`); any other result is `null`.

`null` means no Hints section and no rules (`:203`). `loadPiece` (`chapter_session.ts:22-27`) keeps dropping the container; the resolver, not the scope, owns the lookup. Script groups (`scriptGroupId`, `Editor.tsx:433`) are out of scope: prose only. Inferred from the existing chapter path, not run: the resolved id feeds `HintSettings` and the hint runner exactly as `chapterSync.scope.seriesId` does now.

### Owner decisions on the open points

Each answer is the narrowest reading consistent with current editor code.

- a. Tip trigger. Owner decision: pointer rest on a hint opens the tip; a tap or click toggles it. Keyboard: a collapsed caret inside a hint opens it (selection changes, `Editor.tsx:267-269`); Escape closes it. The overlay keeps `pointer-events: none` like the continue window (`ContinueWindow.tsx:41`, `index.css:517-527`), so hit-testing is done on `quill.root` pointer events against hint rects. Rail buttons use `mousedown preventDefault` so they never steal the caret (`editor-design.md:75`); the tip does the same. Content: rule label (`labelKey`) and `messageKey` text. No "ignore" button (Q6). Any new `app/src/ui/` primitive gets its story first (`CLAUDE.md`); prefer the existing `Popover` (`app/src/ui/Popover.tsx`).
- b. Debounce. Owner decision: 400 ms idle after the last user `TEXT_CHANGE`, per `:166`. No debounce exists in `Editor.tsx` to reuse (only the tint timer, `:323`), so the hook owns one timer. Config changes recompute at once.
- c. Multi-line hints. Owner decision: `getBounds(index, length)` returns one rect (`ContinueWindow.tsx:19-20` uses it for two points only). Walk the hint's words, call `getBounds` per word, and start a new segment whenever `top` changes; draw one underline per line segment. Only hints inside the viewport are measured. Measured on scroll, resize and `ResizeObserver`, as in `ContinueWindow.tsx:27-30`.
- d. Continue, AI-streamed and busy text. Owner decision: hints recompute only from `source === "user"` text changes (`Editor.tsx:262`). While `isAiActive()` (`Editor.tsx:404`) there is no recompute; one runs when it ends. Until `chapterSync.ready`, nothing is drawn. AI-inserted text gets hints after that one pass, like any text.
- e. Piece language. Owner decision: the language is the resolved series' language from `GET /series/{id}/rules` (`:184`), the same read `HintSettings` makes; no series, no rules.
- f. Selection and IME. Owner decision: a non-collapsed selection does not hide hints and does not open the tip. While Quill reports `composition.isComposing` (used at `dialogue_typing.dom.test.tsx:120`), no recompute and no tip; one recompute runs when composition ends.
- g. Layering. Owner decision: the overlay is a sibling in `quill.container`, never in the Delta, and sits at z-index 4 (`editor-design.md:86`, "hint 4"); no `--z-hint` token exists yet (`ui/tokens.css:30-36`), so S2 adds one. AI tint is only a CSS colour on `.ql-editor[data-ai-tint]` (`index.css:913-920`) and does not move text, so the two do not collide. The continue marker (`.editor-continue-window`, no z-index) sits at `left: -24px` outside the text; the underline stays on text.

Below 1100px (`Editor.tsx:319`, `:544`) there is no Settings popover and no toggles (`:202`), but the saved config still applies, so the overlay and tip draw there too. Owner decision: same rules, tip opens by tap or caret; no drawer interaction.

## Delivery

- GUZ-691: engine, shared pass with dialogue detection, per-series config and storage, rule toggle UI.
- GUZ-692: first rule batch, starting with the reliable tier and passive voice ported from `prose.ts` into `app/src/md_editor/rules/passive_voice.ts` (`prose.ts` is gone; `analyzeProse` is now `passiveSpans`).
- GUZ-690: the hint overlay and tip wait for the engine and are built on it, not as a passive-only overlay.
- GUZ-1533: Spanish em-dash detector and further dialogue rules (spec below; build stages S1-S3).

## GUZ-692 rule batch

Owner decisions:

- Passive voice ships in GUZ-692, as the last stage. English only, param `maxAdverbs`.
- Watch words work in en and es. Every other GUZ-692 rule is en only.
- Leísmo stays out of the catalogue. The keys `rules.leismo.label` and `hints.longSentence` (`messages.ts:308`, `:312`) stay until the Spanish follow-up.
- Redundant phrases: a fixed list of about 15 body-part phrases, in the repo.
- Repeated word: function words come from one small fixed en list, in one file.
- `Hint.messageArgs` is added (see Overlay and tip).

| Rule id | Languages | Scope | Defaults | `messageArgs` |
|---------|-----------|-------|----------|---------------|
| `dialogue-punctuation` | en | both | none | none |
| `redundant-phrases` | en | narration | none | `phrase` |
| `repeated-word` | en | narration | `window` 20 words | `word` |
| `same-sentence-start` | en | narration | `count` 3 | `word`, `count` |
| `adverbs` | en | both | `max` 1 (label `rules.adverbs.max`) | `word` |
| `watch-words` | en, es | both | off; `words` empty list | `word` |
| `passive-voice` | en | narration | `maxAdverbs` 3 | none |

Owner decision: the default numbers are repeated-word `window` 20, same-sentence-start `count` 3, passive `maxAdverbs` 3, adverbs `max` 1. Scopes, rule ids and `messageArgs` names are worker picks, not owner-set; scope cells are the defaults the rule declares. The dialogue-punctuation hint is anchored at the closing mark, which sits inside the dialogue span, so its scope is `both`.

Owner decision: adverbs `max` is the number of -ly adverbs a sentence may contain. When a sentence has more than `max`, every -ly adverb in it gets a hint. At the default of 1, a sentence with a single -ly adverb, including a dialogue tag, gets no adverb hint.

Hand test: `She quickly ran quickly to the door` has two -ly adverbs (2 > 1), so the `adverbs` rule hints each `quickly`; the `repeated-word` rule hints the second `quickly`, which repeats within `window` 20 words. A Bun test over `createEngine` (`engine.ts:36`) with `RULES` pins this.

Every rule gets Bun tests in its own file, including a dialogue case and a narration case. `RULES` (`engine.ts:29`) and the server catalogue (see Storage and API) gain each rule in the same stage.

Stages for GUZ-692, each under about 400 changed lines including tests, red test first:

| Stage | Content | Estimate |
|-------|---------|----------|
| S1 | `messageArgs` on `Hint`, `HintTip` pass-through, keys for the batch (en and es). Red: tip shows an interpolated message | ~200 lines |
| S2 | `dialogue-punctuation` and `redundant-phrases`: rules, catalogue entries (app and server), Bun tests, Rust catalogue test | ~350 lines |
| S3 | `repeated-word` (function-word list) and `same-sentence-start`, with `window` and `count` | ~350 lines |
| S4 | `adverbs` (-ly, dialogue tags, exceptions such as family, only, early) and `watch-words` | ~300 lines |
| S5 | `passive-voice` ported from `analyzeProse` into `rules/passive_voice.ts`; `MAX_ADVERBS` becomes the `maxAdverbs` param (min 0, max 6) | ~350 lines |
| S6 | Hand-test unit; `e2e/hint-tip.spec.ts` gets `RULES_SHIPPED` true and `FLAGGED_TEXT` set to the hand-test text (written, not run) | ~120 lines |

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

## GUZ-1533 dialogue rules

Spec only; no code in the spec stage. "Owner decision" marks the owner's answers, including the choices the spec proposed and the owner accepted. GUZ-692 owns English dialogue punctuation (`dialogue-punctuation`) and -ly adverbs (`adverbs`); GUZ-1533 duplicates neither.

### Spanish dash detector

- Owner decision: the dash detector runs only when the series language is `es`, and only for a paragraph whose first non-whitespace character is `—` (U+2014). English zones do not change, so a mid-paragraph `—` in English prose is never dialogue.
- Owner decision: the language joins the analysis cache key. Today the analysis key is the paragraph text alone (`engine.ts:55`) while the hint key already holds the language (`engine.ts:57`). `analyzeParagraph(text, language)` is the new signature, and the key becomes `[language, text]`.
- Owner decision: text inside dash dialogue is dialogue for scope. `ParagraphAnalysis.dialogue` and `Word.zone` stay the only channels (`shared_pass.ts:4`); no new field.
- Owner decision: scan left to right. The leading `—` opens dialogue. Each later `—` toggles between dialogue and tag narration. The two dashes that bound a tag belong to the narration span; the leading `—` belongs to the dialogue span. So `—Hola —dijo ella—. ¿Qué tal?` has dialogue `—Hola `, narration `—dijo ella—`, dialogue `. ¿Qué tal?`. A tag with no closing dash runs to the paragraph end (`—Hola —dijo ella.`).
- Owner decision: only `—` is a dash mark. `-` and `–` are not (GUZ-1532 types `—`, `docs/editor-design.md:206`).
- Owner decision: in a dash paragraph the quote scan is skipped; dashes alone set the zones.
- Owner decision: `«` and `»` are quote marks in `es` only. Nested quotes, Owner decision: while a `«` span is open only `»` closes it; `“`, `”` and `"` inside are ignored, so `«dijo “no” y calló»` is one span. English quote scanning is unchanged.
- Owner decision (dialogue across paragraphs): unchanged from the built pass (`shared_pass.ts:47`, rules-engine Known limits). An unclosed quote, or a dash paragraph, runs to the paragraph end, and the next paragraph starts fresh. A paragraph that continues speech with an open quote only is its own dialogue span. State never crosses paragraphs (Q7).
- Measured: no shipped rule changes behaviour. Only `watch-words` runs in `es` and its default scope is `both` (`watch_words.ts:8`, `:11`); the other six declare `languages: ["en"]`.

### Rules

| Rule id | Languages | Scope | Default | Params | Message keys (en / es) |
|---------|-----------|-------|---------|--------|------------------------|
| `dash-tag-punctuation` | es | both | on | none | `rules.dashTagPunctuation.label` "Dash tag punctuation" / "Puntuación de la raya de acotación"; `hints.dashTagPunctuation` "No period or comma before a dialogue dash tag" / "Sin punto ni coma antes de la raya de la acotación" |
| `dash-tag-unclosed` | es | both | off | none | `rules.dashTagUnclosed.label` "Unclosed dash tag" / "Acotación sin cerrar"; `hints.dashTagUnclosed` "Close the dialogue tag with a dash" / "Cierra la acotación con una raya" |
| `said-bookisms` | en, es | narration | off | none | `rules.bookisms.label` "Said-bookisms" / "Verbos de habla marcados"; `hints.bookism` "“{word}” is a said-bookism; consider “said”" / "«{word}» es un verbo de habla marcado; considera «dijo»" (`messageArgs`: `word`) |

- Owner decisions: new rule ids; `dialogue-punctuation` stays `en` (no `es` added). `dash-tag-punctuation` defaults on; `dash-tag-unclosed` and `said-bookisms` default off. `dash-tag-unclosed` is off because a tag can end a dialogue line and narration can continue with no closing dash (`—Ya voy —dijo Juan. Y salió corriendo.`), which is valid Spanish; the rule cannot tell continued narration from resumed speech. Every key exists in both locales.
- Owner decision: a tag is a narration span that opens with `—` and whose first or second word is in the Spanish tag-verb list (below). The list gates both dash rules, so a parenthetical `—` pair is never flagged.
- `dash-tag-punctuation`: hints a `.` or `,` right before a tag-opening `—` (optional space between). `—Hola. —dijo ella—.` and `—Hola, —dijo ella—.` are hinted; `—¿Qué? —preguntó—.` and `—Hola —dijo ella—.` are not. The hint covers the one punctuation character (length 1).
- `dash-tag-unclosed`: hints the sentence-final `.`, `!` or `?` inside a tag when text follows it in the same paragraph and no closing `—` came first (`—Hola —dijo ella. ¿Qué tal?`). A tag that ends the paragraph, closed or not, is valid. The hint covers that one character. Owner decision: a tag with no sentence end before the paragraph end gets no hint.
- `said-bookisms`: hints a listed word in a narration zone within 3 words after the end of a dialogue span or within 3 words before the start of one, in the same paragraph. Owner decision: exact lowercase forms, no stemming; the hint covers the word.
- Owner decision lists, fixed data in the repo, one file each: Spanish tag verbs (about 25, past tense and present: dijo, preguntó, respondió, contestó, añadió, murmuró, susurró, gritó, exclamó, replicó, comentó, explicó, continuó, siguió, insistió, suspiró, musitó, masculló, gruñó, espetó, ladró, siseó, bramó, farfulló, ordenó). English bookisms (about 20: hissed, snarled, growled, barked, spat, purred, breathed, gasped, chirped, mused, cooed, drawled, bellowed, retorted, panted, sneered, whined, crooned, grunted, boomed). Spanish bookisms (about 16: siseó, gruñó, espetó, ladró, susurró, bramó, rugió, masculló, farfulló, graznó, bufó, jadeó, musitó, ronroneó, canturreó, vociferó). Owner reviews all three lists; `said-bookisms` has no params, so no bounds.
- Owner decision: catalogue entries (app and `crates/api/src/manuscripts/rules/catalogue.rs`, shape `:10-19`) have empty `params` for all three.

### Straddling hints

- Owner decision: a hint is kept when its start offset is in the rule's scope and dropped otherwise; it is never clipped. Measured: `inScope` reads only `hint.start` (`engine.ts:65-69`, applied at `:58`), the same as Q4. Nothing changes; the Pipeline step 4 pointer now names Q4.

### Advisory

Same as every rule: hints never block saving, never change text, and never enter the Delta or the AI request. Each rule gets Bun tests with a dialogue case and a narration case in `en` and `es` where the rule supports the language, red first. The E2E spec is written, not run.

### Build stages

Each stage is under about 400 changed lines including tests, red test first.

| Stage | Content | Estimate |
|-------|---------|----------|
| S1 | `analyzeParagraph(text, language)`, dash detector, `«»` in es, `[language, text]` analysis key. Red: `—Hola —dijo ella—. ¿Qué tal?` gives the three spans in es and none in en; en zones unchanged; the same text in two languages does not share a cache entry | ~280 lines |
| S2 | `dash-tag-punctuation` and `dash-tag-unclosed`, Spanish tag-verb list, app and server catalogue, en and es keys, Rust catalogue test. Red: the examples above | ~350 lines |
| S3 | `said-bookisms` and its two lists, catalogue and keys, `e2e/dialogue-hints.spec.ts` written not run | ~300 lines |

### Risks

- Heuristic: a Spanish dash paragraph with no tag verb in the list gets no dash hint (missed hints, no false ones). The list is the tuning knob.
- GUZ-1532 types `—` anywhere (`docs/editor-design.md:206`); the paragraph-start trigger and the `es` gate keep that from making English prose dialogue.
- `dash-tag-unclosed` cannot tell continued narration from resumed speech after a tag, so it is off by default.
- GUZ-1535 converts AI quotes after a run (`docs/editor-design.md:278`); it does not change dash text, so no overlap in code.

## Settled questions

Each answer is an owner decision.

- Q4. A hint that crosses a dialogue/narration boundary is kept when its start is in the rule's scope, and dropped otherwise. It is not clipped.
- Q5. One neutral underline for every tier in v1. Tier colors wait for author feedback.
- Q6. No per-hint "ignore this one" in v1. Turning a rule off is the only control.
- Q7. No cross-paragraph rules in v1. The per-paragraph cache stays exact.

## Open questions

- Spell checking: possible rule source, undecided.
