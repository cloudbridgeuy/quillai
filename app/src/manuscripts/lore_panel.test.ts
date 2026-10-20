import { expect, test } from "bun:test";
import { newLoreDraft, parseLoreTerms, replaceLoreEntry } from "./lore_panel";

test("new lore defaults are independent and author notes stay separate", () => {
  const draft = newLoreDraft();
  expect(draft).toMatchObject({ kind: "character", pinned: false, level: "include", hidden_notes: "", visible_text: "" });
  expect(newLoreDraft()).not.toBe(draft);
});

test("terms trim lines, remove blanks, preserve commas and Unicode", () => {
  expect(parseLoreTerms(" Mira \n\n The captain, elder \r\n月 ")).toEqual(["Mira", "The captain, elder", "月"]);
});

test("saved lore replaces by ID, sorts and does not mutate the source", () => {
  const entry = { ...newLoreDraft(), id: 1, series_id: 2, name: "Zed" };
  const saved = { ...entry, name: "Ada", hidden_notes: "secret" };
  expect(replaceLoreEntry([entry], saved)).toEqual([saved]);
  expect(replaceLoreEntry([entry], { ...saved, id: 3 }).map(row => row.name)).toEqual(["Ada", "Zed"]);
  expect(entry.name).toBe("Zed");
});
