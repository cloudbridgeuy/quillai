import { expect, test } from "bun:test";
import { formatMessage } from "../../i18n/messages";
import { createEngine, RULES, type Scope } from "./engine";
import { FUNCTION_WORDS } from "./function_words";

const ID = "repeated-word";
const run = (text: string, stored: { scope?: Scope; window?: number } = {}, language: "en" | "es" = "en") =>
  createEngine()
    .run(text, language, { [ID]: { scope: stored.scope, params: stored.window === undefined ? undefined : { window: stored.window } } })
    .filter((hint) => hint.ruleId === ID);

test("the repeat is hinted, not the first use, with the word as message arg", () => {
  expect(run("She quickly ran quickly to the door")).toEqual([
    { ruleId: ID, start: 16, length: 7, messageKey: "hints.repeatedWord", messageArgs: { word: "quickly" } },
  ]);
});

test("every later repeat in a chain is hinted and case is ignored", () => {
  expect(run("Red red RED").map((h) => h.start)).toEqual([4, 8]);
});

test("function words are not hinted", () => {
  expect(run("the cat and the dog and the bird")).toEqual([]);
});

test("the function word list is lowercase, small and fixed", () => {
  for (const word of ["the", "and", "of", "to", "a", "she"]) expect(FUNCTION_WORDS.has(word)).toBe(true);
  expect(FUNCTION_WORDS.has("quickly")).toBe(false);
  expect([...FUNCTION_WORDS].every((word) => word === word.toLowerCase())).toBe(true);
  expect(FUNCTION_WORDS.size).toBeLessThan(150);
});

test("the window counts words between the two uses", () => {
  expect(run("red blue green red", { window: 3 })).toHaveLength(1);
  expect(run("red blue green gold red", { window: 3 })).toEqual([]);
  expect(run("red blue green gold red", { window: 4 })).toHaveLength(1);
});

test("the default window is 20 words", () => {
  const filler = (n: number) => Array.from({ length: n }, (_, i) => `w${i}`).join(" ");
  expect(run(`red ${filler(19)} red`)).toHaveLength(1);
  expect(run(`red ${filler(20)} red`)).toEqual([]);
});

test("a repeat in another paragraph is not hinted and offsets are document offsets", () => {
  expect(run("red\nred")).toEqual([]);
  expect(run("Hi.\nred blue red").map((h) => h.start)).toEqual([13]);
});

test("narration scope flags narration repeats only, dialogue scope dialogue only, both flags both", () => {
  const dialogue = '"red red," she said.';
  expect(run(dialogue)).toEqual([]);
  expect(run(dialogue, { scope: "dialogue" })).toHaveLength(1);
  expect(run(dialogue, { scope: "both" })).toHaveLength(1);
  expect(run("red red")).toHaveLength(1);
  expect(run("red red", { scope: "dialogue" })).toEqual([]);
});

test("the rule is English only", () => {
  expect(run("red red", {}, "es")).toEqual([]);
});

test("the rule is registered with the spec defaults and the agreed bounds", () => {
  expect(RULES.find((rule) => rule.id === ID)).toMatchObject({
    labelKey: "rules.repeatedWord.label",
    languages: ["en"],
    defaultEnabled: true,
    defaultScope: "narration",
    params: [{ key: "window", kind: "int", labelKey: "rules.repeatedWord.window", default: 20, min: 1, max: 100 }],
  });
});

test("the message names the word in both locales with no placeholder left", () => {
  const [hint] = run("She quickly ran quickly to the door");
  for (const locale of ["en", "es"]) {
    const text = formatMessage(locale, hint!.messageKey, hint!.messageArgs);
    expect(text).toContain("quickly");
    expect(text).not.toContain("{");
  }
});
