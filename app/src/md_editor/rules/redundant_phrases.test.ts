import { expect, test } from "bun:test";
import { formatMessage } from "../../i18n/messages";
import { createEngine, RULES, type Scope } from "./engine";
import { REDUNDANT_PHRASES } from "./redundant_phrases";

const run = (text: string, scope?: Scope, language: "en" | "es" = "en") =>
  createEngine().run(text, language, scope ? { "redundant-phrases": { scope } } : {});

test("a redundant phrase is flagged over its whole span with the phrase as message arg", () => {
  const text = "She nodded her head and left.";
  const hints = run(text);
  expect(hints).toHaveLength(1);
  expect(text.slice(hints[0]!.start, hints[0]!.start + hints[0]!.length)).toBe("nodded her head");
  expect(hints[0]).toMatchObject({ ruleId: "redundant-phrases", messageKey: "hints.redundantPhrase", messageArgs: { phrase: "nodded her head" } });
});

test("matching ignores case and keeps the listed phrase as the arg", () => {
  const hints = run("He Shrugged His Shoulders.");
  expect(hints.map((h) => h.messageArgs)).toEqual([{ phrase: "shrugged his shoulders" }]);
});

test("the list holds about fifteen phrases and every one is flagged", () => {
  expect(REDUNDANT_PHRASES).toHaveLength(15);
  for (const phrase of REDUNDANT_PHRASES) {
    expect(run(`Then ${phrase} again.`).map((h) => h.messageArgs)).toEqual([{ phrase }]);
  }
});

test("shorter phrases and phrases inside longer words are not flagged", () => {
  for (const text of ["She nodded.", "She shook her head.", "He nodded his headlights.", "She shrugged."]) {
    expect(run(text)).toEqual([]);
  }
});

test("hints are shifted to the document offset of their paragraph", () => {
  expect(run("One.\nHe blinked his eyes.").map((h) => h.start)).toEqual([8]);
});

test("narration scope flags narration only, dialogue scope flags dialogue only, both flags both", () => {
  const dialogue = '"He nodded his head," she said.';
  const narration = "He nodded his head.";
  expect(run(dialogue)).toEqual([]);
  expect(run(dialogue, "narration")).toEqual([]);
  expect(run(dialogue, "dialogue")).toHaveLength(1);
  expect(run(dialogue, "both")).toHaveLength(1);
  expect(run(narration)).toHaveLength(1);
  expect(run(narration, "dialogue")).toEqual([]);
});

test("the rule is English only", () => {
  expect(run("He nodded his head.", undefined, "es")).toEqual([]);
});

test("the rule is registered with the spec defaults", () => {
  const rule = RULES.find((candidate) => candidate.id === "redundant-phrases");
  expect(rule).toMatchObject({ labelKey: "rules.redundantPhrases.label", languages: ["en"], defaultEnabled: true, defaultScope: "narration", params: [] });
});

test("the message renders in both locales with the phrase and no placeholder left", () => {
  const [hint] = run("He nodded his head.");
  for (const locale of ["en", "es"]) {
    const text = formatMessage(locale, hint!.messageKey, hint!.messageArgs);
    expect(text).toContain("nodded his head");
    expect(text).not.toContain("{");
  }
});

test("extra spaces inside the phrase still match", () => {
  expect(run("He nodded  her   head.").map((h) => h.messageArgs)).toEqual([{ phrase: "nodded her head" }]);
});
