import { expect, test } from "bun:test";
import { createEngine, RULES, type Language } from "./engine";

const ID = "dash-tag-punctuation";
const run = (text: string, language: Language = "es", stored = {}) => createEngine().run(text, language, stored).filter((h) => h.ruleId === ID);
const rule = RULES.find((r) => r.id === ID);

test("is registered: es only, scope both, default on, no params", () => {
  expect(rule?.languages).toEqual(["es"]);
  expect(rule?.defaultScope).toBe("both");
  expect(rule?.defaultEnabled).toBe(true);
  expect(rule?.params).toEqual([]);
  expect(rule?.labelKey).toBe("rules.dashTagPunctuation.label");
});

test("a period or comma right before a tag-opening dash is hinted at that character", () => {
  expect(run("—Hola. —dijo ella—.")).toEqual([{ ruleId: ID, start: 5, length: 1, messageKey: "hints.dashTagPunctuation" }]);
  expect(run("—Hola, —dijo ella—.").map((h) => h.start)).toEqual([5]);
  expect(run("—Hola.—dijo ella—.").map((h) => h.start)).toEqual([5]);
});

test("question marks, exclamation marks and clean tags are not hinted", () => {
  for (const text of ["—¿Qué? —preguntó—.", "—¡Hola! —gritó—.", "—Hola —dijo ella—.", "—Hola —dijo ella."]) {
    expect(run(text)).toEqual([]);
  }
});

test("the tag verb may be the second word", () => {
  expect(run("—Hola. —Ana dijo—.").map((h) => h.start)).toEqual([5]);
});

test("a dash pair without a tag verb is never hinted", () => {
  expect(run("—Hola. —y entonces nada—.")).toEqual([]);
  expect(run("—Ven, —con calma— dijo.")).toEqual([]);
});

test("later tags in the same paragraph are checked too", () => {
  expect(run("—Uno —dijo—, dos. —añadió—.").map((h) => h.start)).toEqual([16]);
});

test("english text and non-dash paragraphs are not hinted", () => {
  expect(run("—Hola. —dijo ella—.", "en")).toEqual([]);
  expect(run("Ella dijo. —dijo— algo")).toEqual([]);
});

test("hints are shifted to the document offset of their paragraph", () => {
  expect(run("Intro.\n—Hola. —dijo ella—.").map((h) => h.start)).toEqual([12]);
});

test("the punctuation sits in dialogue: narration scope drops it", () => {
  expect(run("—Hola. —dijo ella—.", "es", { [ID]: { scope: "narration" } })).toEqual([]);
  expect(run("—Hola. —dijo ella—.", "es", { [ID]: { scope: "dialogue" } })).toHaveLength(1);
});

test("can be switched off", () => {
  expect(run("—Hola. —dijo ella—.", "es", { [ID]: { enabled: false } })).toEqual([]);
});
