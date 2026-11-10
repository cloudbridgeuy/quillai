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
| GUZ-690 | Overlay and tip, E2E at 1100px and 1440px | separate ticket |

## Settled questions

Each answer is an owner decision.

- Q4. A hint that crosses a dialogue/narration boundary is kept when its start is in the rule's scope, and dropped otherwise. It is not clipped.
- Q5. One neutral underline for every tier in v1. Tier colors wait for author feedback.
- Q6. No per-hint "ignore this one" in v1. Turning a rule off is the only control.
- Q7. No cross-paragraph rules in v1. The per-paragraph cache stays exact.

## Open questions

- Spell checking: possible rule source, undecided.
