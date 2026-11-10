import { expect, test } from "bun:test";
import { formatMessage } from "../../i18n/messages";
import { createEngine, RULES, type Language, type Scope } from "./engine";

const ID = "watch-words";
const run = (text: string, words: string[] | undefined, extra: { scope?: Scope; enabled?: boolean } = {}, language: Language = "en") =>
  createEngine()
    .run(text, language, { [ID]: { enabled: extra.enabled ?? true, scope: extra.scope, params: words ? { words } : undefined } })
    .filter((hint) => hint.ruleId === ID);
const found = (text: string, words: string[], extra: { scope?: Scope } = {}, language: Language = "en") =>
  run(text, words, extra, language).map((h) => text.slice(h.start, h.start + h.length));

test("each listed word found is hinted with the word as message arg", () => {
  expect(run("It was suddenly dark.", ["suddenly"])).toEqual([
    { ruleId: ID, start: 7, length: 8, messageKey: "hints.watch", messageArgs: { word: "suddenly" } },
  ]);
});

test("an empty list, or no list, gives no hints", () => {
  expect(run("It was suddenly dark.", [])).toEqual([]);
  expect(run("It was suddenly dark.", undefined)).toEqual([]);
});

test("the rule is off by default", () => {
  expect(createEngine().run("It was suddenly dark.", "en", {})).toEqual([]);
  expect(run("It was suddenly dark.", ["suddenly"], { enabled: false })).toEqual([]);
});

test("matching ignores case and respects word boundaries", () => {
  expect(found("Suddenly he ran suddenlyish and unsuddenly.", ["suddenly"])).toEqual(["Suddenly"]);
});

test("phrases match across extra spaces and longer entries win over their prefixes", () => {
  expect(found("All  of a sudden it ended.", ["all of a sudden"])).toEqual(["All  of a sudden"]);
  expect(found("Run fast now.", ["run", "run fast"])).toEqual(["Run fast"]);
});

test("blank entries are ignored and regex characters are literal", () => {
  expect(found("He saw axb and a.b.", ["", "  ", "a.b"])).toEqual(["a.b"]);
});

test("hints are shifted to the document offset of their paragraph", () => {
  expect(run("Hi.\nit was suddenly dark", ["suddenly"]).map((h) => h.start)).toEqual([11]);
});

test("the rule works in Spanish with accented letters", () => {
  expect(found("Él también vino, tambien no.", ["también"], {}, "es")).toEqual(["también"]);
});

test("default scope is both; narration scope drops dialogue hints and dialogue scope drops narration hints", () => {
  const text = '"Suddenly," he said. Suddenly he left.';
  expect(found(text, ["suddenly"])).toEqual(["Suddenly", "Suddenly"]);
  expect(found(text, ["suddenly"], { scope: "narration" })).toEqual(["Suddenly"]);
  expect(run(text, ["suddenly"], { scope: "narration" })[0]!.start).toBe(21);
  expect(found(text, ["suddenly"], { scope: "dialogue" })).toEqual(["Suddenly"]);
  expect(run(text, ["suddenly"], { scope: "dialogue" })[0]!.start).toBe(1);
});

test("the rule is registered with the spec defaults", () => {
  expect(RULES.find((rule) => rule.id === ID)).toMatchObject({
    labelKey: "rules.watch.label",
    languages: ["en", "es"],
    defaultEnabled: false,
    defaultScope: "both",
    params: [{ key: "words", kind: "words", labelKey: "rules.watch.words", default: [] }],
  });
});

test("the message names the word in both locales with no placeholder left", () => {
  const [hint] = run("It was suddenly dark.", ["suddenly"]);
  for (const locale of ["en", "es"]) {
    const text = formatMessage(locale, hint!.messageKey, hint!.messageArgs);
    expect(text).toContain("suddenly");
    expect(text).not.toContain("{");
  }
});
