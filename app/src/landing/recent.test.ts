import { expect, test } from "bun:test";
import { LAST_EDITED_KEY } from "./routes";
import { forgetRecent, forgetStoredIds, parseRecent, pushRecent, RECENT_KEY } from "./recent";

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

test("a stale piece URL drops that recent row and the legacy key", () => {
  const chapter = { scope: { seriesId: 4, bookId: 5, chapterId: 6 }, title: "Chapter" };
  window.localStorage.setItem(RECENT_KEY, JSON.stringify([item(9), chapter]));
  window.localStorage.setItem(LAST_EDITED_KEY, JSON.stringify(item(9)));
  forgetStoredIds("?universeId=1&pieceId=9");
  expect(JSON.parse(window.localStorage.getItem(RECENT_KEY)!)).toEqual([chapter]);
  expect(window.localStorage.getItem(LAST_EDITED_KEY)).toBeNull();
  window.localStorage.clear();
});

test("migrates legacy only when the recent key is absent", () => {
  expect(parseRecent(null, JSON.stringify(item(1)))).toEqual([item(1)]);
  expect(parseRecent("[]", JSON.stringify(item(1)))).toEqual([]);
});
