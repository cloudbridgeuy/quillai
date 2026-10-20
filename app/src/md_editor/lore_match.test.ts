import { expect, test } from "bun:test";

import { matchLoreEntries, matchLoreOccurrences, type LoreMatch, type LoreMatchEntry } from "./lore_match";
import { calculateDocumentWindow } from "./window";

function entry(overrides: Partial<LoreMatchEntry> = {}): LoreMatchEntry {
  return {
    id: 1,
    kind: "character",
    name: "Mira",
    aliases: [],
    keywords: [],
    visible_text: "A traveler.",
    pinned: false,
    level: "include",
    ...overrides,
  };
}

test("occurrence offsets preserve decomposed text, emoji and overlapping aliases", () => {
  const text = "🐉 E\u0301lodie met Captain Mira by the compass. Mirage.";
  const entries = [entry({ name: "Élodie" }), entry({ id: 2, aliases: ["Captain Mira", "Mira"], keywords: ["compass"] })];
  const matches = matchLoreOccurrences(text, entries);
  expect(matches.map(match => match.text)).toEqual(["E\u0301lodie", "Captain Mira", "compass"]);
  for (const match of matches) expect(text.slice(match.start, match.end)).toBe(match.text);
  expect(matches[0]?.start).toBe(3);
});

test("matches an entry name once even with repeated mentions", () => {
  expect(matchLoreEntries("Mira met Mira.", [entry()], 100)).toEqual([{
    id: 1,
    kind: "character",
    name: "Mira",
    visible_text: "A traveler.",
    level: "include",
    reason: "keyword",
  }]);
});

test("returns no matches for unrelated or empty window text", () => {
  expect(matchLoreEntries("Someone else arrived.", [entry()], 100)).toEqual([]);
  expect(matchLoreEntries("", [entry()], 100)).toEqual([]);
  expect(matchLoreEntries("Mira", [], 100)).toEqual([]);
});

test("matches aliases including multiword phrases", () => {
  expect(matchLoreEntries("The Red Fox arrived.", [entry({ aliases: ["Red Fox"] })], 100)[0]?.reason).toBe("keyword");
});

test("matches independent keywords", () => {
  expect(matchLoreEntries("The moonstone glowed.", [entry({ keywords: ["moonstone"] })], 100)[0]?.id).toBe(1);
});

test("matches names, aliases, and keywords without case sensitivity", () => {
  for (const text of ["mIRA", "RED FOX", "MOONstone"]) {
    expect(matchLoreEntries(text, [entry({ aliases: ["Red Fox"], keywords: ["moonstone"] })], 100)).toHaveLength(1);
  }
});

test("uses Unicode simple case folding without expanding sharp s to SS", () => {
  const lore = entry({ name: "Straße" });
  expect(matchLoreEntries("STRAẞE", [lore], 100)).toHaveLength(1);
  expect(matchLoreEntries("STRASSE", [lore], 100)).toEqual([]);
});

test("rejects substrings within longer Unicode words or identifiers", () => {
  for (const text of ["Mirage", "adMira", "Mira2", "_Mira", "Mira_", "éMira", "Mira界", "Mira\u0301"]) {
    expect(matchLoreEntries(text, [entry()], 100)).toEqual([]);
  }
});

test("accepts word edges at punctuation and text boundaries", () => {
  for (const text of ["Mira", "(Mira)", "Mira's", "Hello—Mira!"]) {
    expect(matchLoreEntries(text, [entry()], 100)).toHaveLength(1);
  }
});

test("treats regular expression syntax literally", () => {
  const lore = entry({ name: "C++ [mage].*" });
  expect(matchLoreEntries("C++ [mage].*", [lore], 100)).toHaveLength(1);
  expect(matchLoreEntries("CCCC mage!", [lore], 100)).toEqual([]);
});

test("ignores blank aliases and keywords", () => {
  expect(matchLoreEntries("Other words", [entry({ aliases: ["", " "], keywords: ["\n"] })], 100)).toEqual([]);
});

test("normalizes accented names and supports isolated CJK names", () => {
  expect(matchLoreEntries("E\u0301LODIE!", [entry({ name: "Élodie" })], 100)).toHaveLength(1);
  expect(matchLoreEntries("「龍」", [entry({ name: "龍" })], 100)).toHaveLength(1);
  expect(matchLoreEntries("青龍", [entry({ name: "龍" })], 100)).toEqual([]);
});

test("matches complete emoji names without matching parts of joined or modified emoji", () => {
  expect(matchLoreEntries("A 🧙🏽‍♀️ appeared.", [entry({ name: "🧙🏽‍♀️" })], 100)).toHaveLength(1);
  for (const text of ["🧙🏽", "🧙‍♀️", "a🧙"]) {
    expect(matchLoreEntries(text, [entry({ name: "🧙" })], 100)).toEqual([]);
  }
});

test("always includes pins, with pinned reason even if the name matches", () => {
  const lore = entry({ pinned: true });
  expect(matchLoreEntries("", [lore], 0)[0]?.reason).toBe("pinned");
  expect(matchLoreEntries("Mira", [lore], 0)[0]?.reason).toBe("pinned");
});

test("orders pins first and preserves input order within both groups", () => {
  const entries = [entry({ id: 4 }), entry({ id: 3, pinned: true }), entry({ id: 2 }), entry({ id: 1, pinned: true })];
  expect(matchLoreEntries("Mira", entries, 100).map(match => match.id)).toEqual([3, 1, 4, 2]);
});

test("respects the exact character budget and skips overflow to include smaller later matches", () => {
  const entries = [entry({ id: 1, visible_text: "ab" }), entry({ id: 2 }), entry({ id: 3, visible_text: "" })];
  expect(matchLoreEntries("Mira", entries, 6).map(match => match.id)).toEqual([1]);
  expect(matchLoreEntries("Mira", entries, 10).map(match => match.id)).toEqual([1, 3]);
  expect(matchLoreEntries("Mira", entries, 5).map(match => match.id)).toEqual([3]);
});

test("pins consume the budget but are never dropped even when they overflow", () => {
  const entries = [entry(), entry({ id: 2, pinned: true }), entry({ id: 3, pinned: true })];
  expect(matchLoreEntries("Mira", entries, 0).map(match => match.id)).toEqual([2, 3]);
  expect(matchLoreEntries("Mira", entries, 45).map(match => match.id)).toEqual([2, 3, 1]);
  expect(matchLoreEntries("Mira", entries, 44).map(match => match.id)).toEqual([2, 3]);
});

test("pins exceeding the budget suppress every optional match, including small ones", () => {
  const entries = [
    entry({ id: 1, name: "M", visible_text: "", keywords: ["Mira"] }),
    entry({ id: 2, pinned: true }),
    entry({ id: 3, pinned: true }),
  ];
  expect(matchLoreEntries("Mira", entries, 20).map(match => match.id)).toEqual([2, 3]);
});

test("counts UTF-16 code units in names and visible text only", () => {
  const lore = entry({ name: "🐉", visible_text: "é" });
  expect(matchLoreEntries("🐉", [lore], 3)).toHaveLength(1);
  expect(matchLoreEntries("🐉", [lore], 2)).toEqual([]);
});

test("rejects invalid character budgets", () => {
  for (const budget of [-1, 1.5, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1]) {
    expect(() => matchLoreEntries("Mira", [entry()], budget)).toThrow(RangeError);
  }
});

test("excludes hidden notes from the public result type and runtime payload", () => {
  const excludesHiddenNotes: "hidden_notes" extends keyof LoreMatch ? false : true = true;
  const excludesHiddenInput: "hidden_notes" extends keyof LoreMatchEntry ? false : true = true;
  const lore = { ...entry(), hidden_notes: "A secret." };
  expect(excludesHiddenNotes).toBe(true);
  expect(excludesHiddenInput).toBe(true);
  expect(matchLoreEntries("Mira", [lore], 100)[0]).not.toHaveProperty("hidden_notes");
  expect(matchLoreEntries("A secret.", [lore], 100)).toEqual([]);
});

test("carries all levels through unchanged without filtering", () => {
  for (const level of ["include", "known_dont_reveal", "leave_out"] as const) {
    expect(matchLoreEntries("Mira", [entry({ level })], 100)[0]?.level).toBe(level);
  }
});

test("matches only the transmitted Continue slice", () => {
  const document = "Mira left. Nobody returned.";
  const window = calculateDocumentWindow({ documentLength: document.length, windowSize: 15 });
  expect(matchLoreEntries(document.slice(window.start, window.end), [entry()], 100)).toEqual([]);
});

test("does not mutate inputs and returns fresh result objects", () => {
  const lore = Object.freeze(entry({ aliases: Object.freeze(["Red Fox"]), keywords: Object.freeze(["stone"]) }));
  const entries = Object.freeze([lore]);
  const first = matchLoreEntries("Mira", entries, 100);
  expect(matchLoreEntries("Mira", entries, 100)).toEqual(first);
  expect(first[0]).not.toBe(lore);
});
