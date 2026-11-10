import { expect, test } from "bun:test";
import { formatMessage } from "../../i18n/messages";
import { createEngine, RULES, type Scope } from "./engine";

const ID = "same-sentence-start";
const run = (text: string, stored: { scope?: Scope; count?: number } = {}, language: "en" | "es" = "en") =>
  createEngine()
    .run(text, language, { [ID]: { scope: stored.scope, params: stored.count === undefined ? undefined : { count: stored.count } } })
    .filter((hint) => hint.ruleId === ID);

test("the sentence that completes a run of count is hinted on its first word", () => {
  expect(run("She ran. She hid. She slept.")).toEqual([
    { ruleId: ID, start: 18, length: 3, messageKey: "hints.sameStart", messageArgs: { count: 3, word: "She" } },
  ]);
});

test("each longer sentence in the run is hinted with the run length so far", () => {
  const hints = run("She ran. She hid. She slept. She woke.");
  expect(hints.map((h) => [h.start, h.messageArgs])).toEqual([
    [18, { count: 3, word: "She" }],
    [29, { count: 4, word: "She" }],
  ]);
});

test("shorter or interrupted runs are not hinted and case is ignored", () => {
  expect(run("She ran. She hid.")).toEqual([]);
  expect(run("She ran. He hid. She slept. She woke.")).toEqual([]);
  expect(run("she ran. She hid. SHE slept.")).toHaveLength(1);
});

test("the count param sets the run length", () => {
  expect(run("She ran. She hid.", { count: 2 }).map((h) => h.messageArgs)).toEqual([{ count: 2, word: "She" }]);
});

test("the default count is 3", () => {
  const rule = RULES.find((candidate) => candidate.id === ID);
  expect(rule?.params).toEqual([{ key: "count", kind: "int", labelKey: "rules.sameStart.count", default: 3, min: 2, max: 10 }]);
});

test("a run does not cross paragraphs and offsets are document offsets", () => {
  expect(run("She ran. She hid.\nShe slept.")).toEqual([]);
  expect(run("Hi.\nShe ran. She hid. She slept.").map((h) => h.start)).toEqual([22]);
});

test("a sentence opened by a quote still starts with its first word", () => {
  expect(run('"Run now. Run fast. Run far."', { scope: "dialogue" }).map((h) => h.messageArgs)).toEqual([{ count: 3, word: "Run" }]);
});

test("narration scope flags narration runs only, dialogue scope dialogue runs only, both flags both", () => {
  const dialogue = '"Run now. Run fast. Run far."';
  expect(run(dialogue)).toEqual([]);
  expect(run(dialogue, { scope: "both" })).toHaveLength(1);
  expect(run("She ran. She hid. She slept.")).toHaveLength(1);
  expect(run("She ran. She hid. She slept.", { scope: "dialogue" })).toEqual([]);
});

test("known limit: Mr. ends a sentence, so the next word counts as a sentence start", () => {
  expect(run("Mr. Smith left. Smith hid. Smith slept.")).toHaveLength(1);
});

test("known limit: Dr. ends a sentence, so the next word counts as a sentence start", () => {
  expect(run("Dr. Ng left. Ng hid. Ng slept.")).toHaveLength(1);
});

test("the rule is English only", () => {
  expect(run("She ran. She hid. She slept.", {}, "es")).toEqual([]);
});

test("the rule is registered with the spec defaults", () => {
  expect(RULES.find((rule) => rule.id === ID)).toMatchObject({ labelKey: "rules.sameStart.label", languages: ["en"], defaultEnabled: true, defaultScope: "narration" });
});

test("the message names the word and the count in both locales with no placeholder left", () => {
  const [hint] = run("She ran. She hid. She slept.");
  for (const locale of ["en", "es"]) {
    const text = formatMessage(locale, hint!.messageKey, hint!.messageArgs);
    expect(text).toContain("She");
    expect(text).toContain("3");
    expect(text).not.toContain("{");
  }
});
