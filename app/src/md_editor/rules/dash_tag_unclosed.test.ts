import { expect, test } from "bun:test";
import { createEngine, RULES, type Language, type StoredRules } from "./engine";

const ID = "dash-tag-unclosed";
const run = (text: string, language: Language = "es", stored: StoredRules = { [ID]: { enabled: true } }) =>
  createEngine().run(text, language, stored).filter((h) => h.ruleId === ID);
const rule = RULES.find((r) => r.id === ID);

test("is registered: es only, scope both, default off, no params", () => {
  expect(rule?.languages).toEqual(["es"]);
  expect(rule?.defaultScope).toBe("both");
  expect(rule?.defaultEnabled).toBe(false);
  expect(rule?.params).toEqual([]);
  expect(rule?.labelKey).toBe("rules.dashTagUnclosed.label");
});

test("is off unless the series turns it on", () => {
  expect(run("—Hola —dijo ella. ¿Qué tal?", "es", {})).toEqual([]);
});

test("a sentence end inside a tag with text after it is hinted at that character", () => {
  expect(run("—Hola —dijo ella. ¿Qué tal?")).toEqual([{ ruleId: ID, start: 16, length: 1, messageKey: "hints.dashTagUnclosed" }]);
  expect(run("—Hola —dijo ella! Vamos").map((h) => h.start)).toEqual([16]);
  expect(run("—Hola —preguntó ella? Vamos").map((h) => h.start)).toEqual([20]);
});

test("a tag that ends the paragraph, or has no sentence end, is not hinted", () => {
  for (const text of ["—Hola —dijo ella.", "—Hola —dijo ella", "—Hola —dijo ella— ¿qué tal?", "—Hola —dijo ella—. ¿Qué tal?", "—Hola —dijo ella.—"]) {
    expect(run(text)).toEqual([]);
  }
});

test("a dash pair without a tag verb is never hinted", () => {
  expect(run("—Hola —y nada más. Fin")).toEqual([]);
});

test("english text and non-dash paragraphs are not hinted", () => {
  expect(run("—Hola —dijo ella. ¿Qué tal?", "en")).toEqual([]);
  expect(run("Ella dijo —dijo ella. Fin")).toEqual([]);
});

test("hints are shifted to the document offset of their paragraph", () => {
  expect(run("Intro.\n—Hola —dijo ella. ¿Qué tal?").map((h) => h.start)).toEqual([23]);
});

test("the sentence end sits in narration: dialogue scope drops it", () => {
  expect(run("—Hola —dijo ella. ¿Qué tal?", "es", { [ID]: { enabled: true, scope: "dialogue" } })).toEqual([]);
  expect(run("—Hola —dijo ella. ¿Qué tal?", "es", { [ID]: { enabled: true, scope: "narration" } })).toHaveLength(1);
});
