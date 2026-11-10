import { expect, test } from "bun:test";
import { createEngine, resolveConfig, type Rule, type RuleConfig } from "./engine";
import type { ParagraphAnalysis } from "./shared_pass";

type Call = { doc: ParagraphAnalysis; params: RuleConfig["params"] };

function wordRule(word: string, overrides: Partial<Rule> = {}, calls: Call[] = []): Rule {
  return {
    id: `word-${word}`,
    labelKey: "rules.adverbs.label",
    languages: ["en"],
    defaultEnabled: true,
    params: [{ key: "max", kind: "int", labelKey: "rules.adverbs.max", default: 3, min: 1, max: 9 }],
    defaultScope: "both",
    check(doc, config) {
      calls.push({ doc, params: config.params });
      return doc.words
        .filter((w) => w.lower === word)
        .map((w) => ({ ruleId: `word-${word}`, start: w.start, length: w.end - w.start, messageKey: "hints.adverb" }));
    },
    ...overrides,
  };
}

const spans = (text: string, hints: { start: number; length: number }[]) =>
  hints.map((h) => text.slice(h.start, h.start + h.length));

test("missing rules and keys fall back to the rule defaults", () => {
  const rule = wordRule("x", {
    params: [
      { key: "max", kind: "int", labelKey: "rules.adverbs.max", default: 3, min: 1, max: 9 },
      { key: "words", kind: "words", labelKey: "rules.watch.words", default: ["a"] },
    ],
  });
  expect(resolveConfig(rule, undefined)).toEqual({ enabled: true, scope: "both", params: { max: 3, words: ["a"] } });
  expect(resolveConfig(rule, { scope: "dialogue", params: { max: 5 } })).toEqual({
    enabled: true,
    scope: "dialogue",
    params: { max: 5, words: ["a"] },
  });
  expect(resolveConfig(rule, { enabled: false })).toEqual({ enabled: false, scope: "both", params: { max: 3, words: ["a"] } });
});

test("shifts paragraph hints to document offsets", () => {
  const text = "a cat\nthe cat sat";
  const hints = createEngine([wordRule("cat")]).run(text, "en", {});
  expect(hints).toEqual([
    { ruleId: "word-cat", start: 2, length: 3, messageKey: "hints.adverb" },
    { ruleId: "word-cat", start: 10, length: 3, messageKey: "hints.adverb" },
  ]);
  expect(spans(text, hints)).toEqual(["cat", "cat"]);
});

test("runs only enabled rules for the series language and ignores unknown ids", () => {
  const rules = [wordRule("cat"), wordRule("dog", { defaultEnabled: false }), wordRule("sol", { languages: ["es"] })];
  const engine = createEngine(rules);
  const text = "cat dog sol";
  expect(spans(text, engine.run(text, "en", { nope: { enabled: true } }))).toEqual(["cat"]);
  expect(spans(text, engine.run(text, "en", { "word-dog": { enabled: true }, "word-cat": { enabled: false } }))).toEqual([
    "dog",
  ]);
  expect(spans(text, engine.run(text, "es", {}))).toEqual(["sol"]);
});

test("scope keeps a hint whose start is in scope and drops the rest, without clipping", () => {
  const text = "“go now,” go said";
  const straddle: Rule = {
    ...wordRule("go", { defaultScope: "narration" }),
    check: () => [
      { ruleId: "s", start: 1, length: 2, messageKey: "hints.adverb" },
      { ruleId: "s", start: 10, length: 2, messageKey: "hints.repeatedWord" },
      { ruleId: "s", start: 3, length: 9, messageKey: "hints.sameStart" },
      { ruleId: "s", start: 10, length: 7, messageKey: "hints.longSentence" },
    ],
  };
  const engine = createEngine([straddle]);
  expect(engine.run(text, "en", {}).map((h) => h.messageKey)).toEqual(["hints.repeatedWord", "hints.longSentence"]);
  expect(engine.run(text, "en", { "word-go": { scope: "dialogue" } }).map((h) => h.messageKey)).toEqual([
    "hints.adverb",
    "hints.sameStart",
  ]);
  expect(engine.run(text, "en", { "word-go": { scope: "both" } })).toHaveLength(4);
});

test("typing in one paragraph rechecks only that paragraph", () => {
  const calls: Call[] = [];
  const engine = createEngine([wordRule("cat", {}, calls)]);
  engine.run("a cat\nb cat\nc cat", "en", {});
  expect(calls.map((c) => c.doc.text)).toEqual(["a cat", "b cat", "c cat"]);
  calls.length = 0;
  engine.run("a cat\nb cats\nc cat", "en", {});
  expect(calls.map((c) => c.doc.text)).toEqual(["b cats"]);
});

test("config changes recheck hints but reuse the analysis", () => {
  const calls: Call[] = [];
  const engine = createEngine([wordRule("cat", {}, calls)]);
  engine.run("a cat", "en", {});
  engine.run("a cat", "en", { "word-cat": { params: { max: 7 } } });
  expect(calls.map((c) => c.params)).toEqual([{ max: 3 }, { max: 7 }]);
  expect(calls[0]?.doc === calls[1]?.doc).toBe(true);
});

test("the cache is bounded and evicts the least recently used paragraph", () => {
  const calls: Call[] = [];
  const engine = createEngine([wordRule("cat", {}, calls)], 2);
  engine.run("a\nb", "en", {});
  engine.run("a", "en", {});
  engine.run("c", "en", {});
  calls.length = 0;
  engine.run("a\nb", "en", {});
  expect(calls.map((c) => c.doc.text)).toEqual(["b"]);
});

test("the same text in en and es does not share an analysis", () => {
  const calls: Call[] = [];
  const engine = createEngine([wordRule("hola", { languages: ["en", "es"] }, calls)]);
  engine.run("—Hola —dijo ella—.", "en", {});
  engine.run("—Hola —dijo ella—.", "es", {});
  expect(calls).toHaveLength(2);
  expect(calls[0]?.doc === calls[1]?.doc).toBe(false);
  expect(calls[0]?.doc.dialogue).toEqual([]);
  expect(calls[1]?.doc.dialogue).toEqual([{ start: 0, end: 6 }, { start: 17, end: 18 }]);
});

test("a dash paragraph scopes hints by dash zones in es", () => {
  const text = "—Hola —dijo ella—. Dijo dijo.";
  const inDialogue = createEngine([wordRule("dijo", { languages: ["es"], defaultScope: "dialogue" })]);
  expect(spans(text, inDialogue.run(text, "es", {}))).toEqual(["Dijo", "dijo"]);
  const inNarration = createEngine([wordRule("dijo", { languages: ["es"], defaultScope: "narration" })]);
  expect(spans(text, inNarration.run(text, "es", {}))).toEqual(["dijo"]);
});
