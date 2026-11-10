import { expect, test } from "bun:test";
import { formatMessage } from "../../i18n/messages";
import { ADVERB_EXCEPTIONS } from "./adverb_exceptions";
import { createEngine, RULES, type Scope } from "./engine";

const ID = "adverbs";
const run = (text: string, stored: { scope?: Scope; max?: number } = {}, language: "en" | "es" = "en") =>
  createEngine()
    .run(text, language, { [ID]: { scope: stored.scope, params: stored.max === undefined ? undefined : { max: stored.max } } })
    .filter((hint) => hint.ruleId === ID);
const words = (text: string, stored?: { scope?: Scope; max?: number }) =>
  run(text, stored).map((h) => text.slice(h.start, h.start + h.length));

test("a sentence with a single -ly adverb gets no hint at the default, dialogue tag included", () => {
  expect(run("She ran quickly.")).toEqual([]);
  expect(run('"Go," she said quietly.')).toEqual([]);
});

test("more than max -ly adverbs in a sentence hints every one with the word as message arg", () => {
  expect(run("She quickly ran quickly home.")).toEqual([
    { ruleId: ID, start: 4, length: 7, messageKey: "hints.adverb", messageArgs: { word: "quickly" } },
    { ruleId: ID, start: 16, length: 7, messageKey: "hints.adverb", messageArgs: { word: "quickly" } },
  ]);
});

test("the count is per sentence and case is ignored", () => {
  expect(run("She ran quickly. He walked slowly.")).toEqual([]);
  expect(words("Quickly she ran slowly.")).toEqual(["Quickly", "slowly"]);
});

test("the max param sets how many a sentence may hold", () => {
  const text = "She quickly and slowly ran.";
  expect(run(text, { max: 2 })).toEqual([]);
  expect(run("She quickly and slowly ran madly.", { max: 2 })).toHaveLength(3);
  expect(run("She ran quickly.", { max: 0 })).toHaveLength(1);
});

test("-ly words that are not adverbs are neither counted nor hinted", () => {
  expect(run("The family only came early to the lonely lovely friendly place.")).toEqual([]);
  expect(run("She quickly ran early.")).toEqual([]);
  expect(run("The fly and the sly ply.")).toEqual([]);
});

test("the exceptions list is lowercase and holds the common non-adverbs", () => {
  for (const word of ["family", "only", "early", "lonely"]) expect(ADVERB_EXCEPTIONS.has(word)).toBe(true);
  expect(ADVERB_EXCEPTIONS.has("quickly")).toBe(false);
  expect([...ADVERB_EXCEPTIONS].every((word) => word === word.toLowerCase())).toBe(true);
});

test("hints are shifted to the document offset of their paragraph", () => {
  expect(run("Hi.\nShe quickly ran quickly.").map((h) => h.start)).toEqual([8, 20]);
});

test("default scope is both; narration scope drops dialogue hints and dialogue scope drops narration hints", () => {
  const dialogue = '"She quickly ran quickly," he said.';
  const narration = "She quickly ran quickly.";
  expect(run(dialogue)).toHaveLength(2);
  expect(run(dialogue, { scope: "narration" })).toEqual([]);
  expect(run(dialogue, { scope: "dialogue" })).toHaveLength(2);
  expect(run(narration)).toHaveLength(2);
  expect(run(narration, { scope: "narration" })).toHaveLength(2);
  expect(run(narration, { scope: "dialogue" })).toEqual([]);
});

test("the rule is English only", () => {
  expect(run("She quickly ran quickly.", {}, "es")).toEqual([]);
});

test("the rule is registered with the spec defaults and the agreed bounds", () => {
  expect(RULES.find((rule) => rule.id === ID)).toMatchObject({
    labelKey: "rules.adverbs.label",
    languages: ["en"],
    defaultEnabled: true,
    defaultScope: "both",
    params: [{ key: "max", kind: "int", labelKey: "rules.adverbs.max", default: 1, min: 0, max: 9 }],
  });
});

test("the message names the word in both locales with no placeholder left", () => {
  const [hint] = run("She quickly ran quickly home.");
  for (const locale of ["en", "es"]) {
    const text = formatMessage(locale, hint!.messageKey, hint!.messageArgs);
    expect(text).toContain("quickly");
    expect(text).not.toContain("{");
  }
});

test("hand test: the default engine hints each quickly as an adverb and the second as a repeat", () => {
  const text = "She quickly ran quickly to the door";
  const found = createEngine().run(text, "en", {}).map((h) => `${h.ruleId}@${h.start}`).sort();
  expect(found).toEqual(["adverbs@16", "adverbs@4", "repeated-word@16"]);
});
