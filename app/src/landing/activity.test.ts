import { expect, test } from "bun:test";
import { activityGroups, parseActivity } from "./activity";

const row = { pieceId: 1, universeId: 2, title: "Story", label: "short_story", format: "prose", parentId: null, changedAt: "2026-11-04T12:00:00Z", chapter: null };

test("parses valid rows and drops invalid entries", () => {
  expect(parseActivity([row, { ...row, pieceId: 0 }, { ...row, changedAt: "bad" }, { ...row, changedAt: "2026-02-30T12:00:00Z" }, { ...row, chapter: { seriesId: 1 } }, null])).toHaveLength(1);
  expect(() => parseActivity({})).toThrow();
});

test("folds script scenes and selects editor scopes", () => {
  const rows = parseActivity([
    { ...row, pieceId: 3, format: "script", parentId: 9, changedAt: "2026-11-03T12:00:00Z" },
    { ...row, pieceId: 4, format: "script", parentId: 9 },
    { ...row, pieceId: 10 },
    { ...row, pieceId: 5, chapter: { seriesId: 6, bookId: 7, chapterId: 8 } },
  ]);
  const groups = activityGroups(rows, new Date("2026-11-04T18:00:00Z"));
  const all = groups.flatMap(group => group.rows);
  expect(all).toHaveLength(3);
  expect(all.find(item => "pieceId" in item.scope)?.scope).toEqual({ universeId: 2, pieceId: 9 });
  expect(all.find(item => "pieceId" in item.scope)?.changedAt).toBe(row.changedAt);
  expect(all.find(item => "pieceId" in item.scope && item.scope.pieceId === 10)?.scope).toEqual({ universeId: 2, pieceId: 10 });
  expect(all.find(item => "chapterId" in item.scope)?.scope).toEqual({ seriesId: 6, bookId: 7, chapterId: 8 });
});

test("groups by local calendar day", () => {
  const now = new Date(2026, 10, 4, 12);
  const rows = [0, 1, 2].map(days => ({ ...row, pieceId: days + 1, changedAt: new Date(2026, 10, 4 - days, 12).toISOString() }));
  expect(activityGroups(parseActivity(rows), now).map(group => group.label)).toEqual(["Today", "Yesterday", "Earlier"]);
});
