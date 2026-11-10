import { expect, test } from "bun:test";
import { createEngine, RULES, type Language, type StoredRules } from "./engine";
import { ENGLISH_BOOKISMS } from "./english_bookisms";
import { SPANISH_BOOKISMS } from "./spanish_bookisms";
import { SPANISH_TAG_VERBS } from "./spanish_tag_verbs";

const ID = "said-bookisms";
const on: StoredRules = { [ID]: { enabled: true } };
const run = (text: string, language: Language = "en", stored: StoredRules = on) =>
  createEngine().run(text, language, stored).filter((h) => h.ruleId === ID);
const words = (text: string, language: Language = "en", stored: StoredRules = on) => run(text, language, stored).map((h) => text.slice(h.start, h.start + h.length));
const rule = RULES.find((r) => r.id === ID);

test("is registered: en and es, scope narration, default off, no params", () => {
  expect(rule?.languages).toEqual(["en", "es"]);
  expect(rule?.defaultScope).toBe("narration");
  expect(rule?.defaultEnabled).toBe(false);
  expect(rule?.params).toEqual([]);
  expect(rule?.labelKey).toBe("rules.bookisms.label");
});

test("is off unless the series turns it on", () => {
  expect(run('"Stop," he hissed.', "en", {})).toEqual([]);
});

test("an english bookism after a quote is hinted at the word with the word as argument", () => {
  expect(run('"Stop," he hissed.')).toEqual([
    { ruleId: ID, start: 11, length: 6, messageKey: "hints.bookism", messageArgs: { word: "hissed" } },
  ]);
});

test("a bookism before a quote is hinted, and case is ignored", () => {
  expect(words('He Hissed, "Stop."')).toEqual(["Hissed"]);
});

test("plain said and ask are not hinted", () => {
  expect(words('"Stop," he said.')).toEqual([]);
  expect(words('"Stop," he asked.')).toEqual([]);
});

test("a bookism in narration far from any dialogue is not hinted", () => {
  expect(words("The snake hissed at the wind.")).toEqual([]);
});

test("the window is 3 words after the dialogue span", () => {
  expect(words('"Stop" one two hissed')).toEqual(["hissed"]);
  expect(words('"Stop" one two three hissed')).toEqual([]);
});

test("the window is 3 words before the dialogue span", () => {
  expect(words('hissed one two three "Stop"')).toEqual([]);
  expect(words('one hissed two three "Stop"')).toEqual(["hissed"]);
  expect(words('hissed two three "Stop"')).toEqual(["hissed"]);
});

test("a bookism inside the dialogue is not hinted under narration scope", () => {
  expect(words('"He hissed at me," she said.')).toEqual([]);
});

test("a bookism in dialogue is never hinted even when another span is within the window and scope is both", () => {
  expect(words('"Stop," "it hissed now."', "en", { [ID]: { enabled: true, scope: "both" } })).toEqual([]);
});

test("a bookism reachable from two spans is hinted once", () => {
  expect(words('"Stop," he hissed, "now."')).toEqual(["hissed"]);
});

test("dialogue scope hints bookisms inside the dialogue", () => {
  expect(words('"He hissed at me," she said.', "en", { [ID]: { enabled: true, scope: "dialogue" } })).toEqual([]);
});

test("spanish bookisms after a guillemet or curly quote are hinted in es", () => {
  expect(words("«Alto», siseó él.", "es")).toEqual(["siseó"]);
  expect(words("“Alto”, gruñó él.", "es")).toEqual(["gruñó"]);
});

test("spanish bookisms around a dash dialogue are hinted in es", () => {
  expect(words("—Alto —siseó él—.", "es")).toEqual(["siseó"]);
  expect(words("—Alto —dijo él—.", "es")).toEqual([]);
});

test("plain spanish verbs are not bookisms", () => {
  for (const verb of ["dijo", "preguntó", "respondió", "contestó"]) {
    expect(SPANISH_BOOKISMS.has(verb)).toBe(false);
    expect(ENGLISH_BOOKISMS.has(verb)).toBe(false);
  }
});

test("english plain verbs are not bookisms", () => {
  for (const verb of ["said", "asked", "replied", "answered"]) expect(ENGLISH_BOOKISMS.has(verb)).toBe(false);
});

test("lists have the specified sizes", () => {
  expect(ENGLISH_BOOKISMS.size).toBe(20);
  expect(SPANISH_BOOKISMS.size).toBe(16);
});

test("present-tense spanish tag verbs are tag verbs", () => {
  for (const verb of ["dice", "pregunta", "responde", "contesta", "añade", "murmura", "susurra", "grita", "exclama", "replica", "comenta", "explica", "continúa", "sigue", "insiste", "suspira", "musita", "masculla", "gruñe", "espeta", "ladra", "sisea", "brama", "farfulla", "ordena"]) {
    expect(SPANISH_TAG_VERBS.has(verb)).toBe(true);
  }
});
