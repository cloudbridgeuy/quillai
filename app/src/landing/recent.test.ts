import { expect, test } from "bun:test";
import { forgetRecent, parseRecent, pushRecent, RECENT_KEY } from "./recent";

const item = (id: number, title = `Title ${id}`) => ({ scope: { universeId: 1, pieceId: id }, title });

test("parses valid entries and drops invalid entries", () => {
  expect(RECENT_KEY).toBe("quillai.landing.recent.v1");
  expect(parseRecent(JSON.stringify([item(1), {}, item(2)]), null)).toEqual([item(1), item(2)]);
  expect(parseRecent("{", JSON.stringify(item(1)))).toEqual([]);
});

test("pushes newest, refreshes duplicates, limits to five and forgets by scope", () => {
  const entries = [1, 2, 3, 4, 5].map(id => item(id));
  expect(pushRecent(entries, item(3, "Updated"))).toEqual([item(3, "Updated"), item(1), item(2), item(4), item(5)]);
  expect(pushRecent(entries, item(6))).toEqual([item(6), ...entries.slice(0, 4)]);
  expect(forgetRecent(entries, item(3).scope)).toEqual([item(1), item(2), item(4), item(5)]);
});

test("migrates legacy only when the recent key is absent", () => {
  expect(parseRecent(null, JSON.stringify(item(1)))).toEqual([item(1)]);
  expect(parseRecent("[]", JSON.stringify(item(1)))).toEqual([]);
});
