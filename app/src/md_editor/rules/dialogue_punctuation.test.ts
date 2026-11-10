import { expect, test } from "bun:test";
import { formatMessage } from "../../i18n/messages";
import { createEngine, RULES, type Scope } from "./engine";

const run = (text: string, scope?: Scope, language: "en" | "es" = "en") =>
  createEngine().run(text, language, scope ? { "dialogue-punctuation": { scope } } : {});

test("a period before a lowercase dialogue tag is flagged at the period", () => {
  const hints = run('"Hello." he said.');
  expect(hints).toEqual([{ ruleId: "dialogue-punctuation", start: 6, length: 1, messageKey: "hints.dialoguePunctuation" }]);
});

test("curly quotes are flagged the same way", () => {
  expect(run("“Hello.” he said.").map((h) => h.start)).toEqual([6]);
});

test("a comma, a new sentence, an ellipsis, a question mark and narration are not flagged", () => {
  for (const text of ['"Hello," he said.', '"Hello." He nodded.', '"Well..." he said.', '"Hello?" he asked.', "He said hello. she left.", '"Hello."']) {
    expect(run(text)).toEqual([]);
  }
});

test("hints are shifted to the document offset of their paragraph", () => {
  expect(run('Intro.\n"Hi." she said.').map((h) => h.start)).toEqual([10]);
});

test("the period sits in dialogue: narration scope drops it, dialogue and both keep it", () => {
  expect(run('"Hello." he said.', "narration")).toEqual([]);
  expect(run('"Hello." he said.', "dialogue")).toHaveLength(1);
  expect(run('"Hello." he said.', "both")).toHaveLength(1);
});

test("the rule is English only", () => {
  expect(run('"Hello." he said.', undefined, "es")).toEqual([]);
});

test("the rule is registered with the spec defaults", () => {
  const rule = RULES.find((candidate) => candidate.id === "dialogue-punctuation");
  expect(rule).toMatchObject({ labelKey: "rules.dialoguePunctuation.label", languages: ["en"], defaultEnabled: true, defaultScope: "both", params: [] });
});

test("the message renders in both locales with no placeholder left", () => {
  const [hint] = run('"Hello." he said.');
  for (const locale of ["en", "es"]) {
    expect(formatMessage(locale, hint!.messageKey, hint!.messageArgs)).not.toContain("{");
  }
});
