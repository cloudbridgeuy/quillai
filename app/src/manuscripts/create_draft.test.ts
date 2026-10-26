import { expect, test } from "bun:test";
import type { Piece } from "./contract";
import {
  addChapter, addChild, addContainer, addRoot, addUniverse, createTitle, draftTransition, labelOptions, nextPosition,
  pieceShape, targetPosition, withBooks, type ManuscriptTreeData,
} from "./create_draft";

const piece = (id: number, overrides: Partial<Piece> = {}): Piece => ({
  id, universeId: 5, containerId: null, parentId: null, title: `Piece ${id}`, position: 0,
  label: "piece", format: "prose", shape: "leaf", ...overrides,
});
const tree: ManuscriptTreeData = {
  series: [{ id: 1, title: "Series", position: 5, books: [
    { id: 2, series_id: 1, title: "Book", position: 3, chapters: [
      { id: 3, title: "Chapter", position: 7, kind: "chapter", head_rev: 0 },
    ] },
  ] }],
  universes: [{ id: 5, title: "World", position: 4, containers: [
    { id: 1, universeId: 5, title: "Series", position: 2, kind: "book_series", pieces: [{ ...piece(20, { containerId: 1, position: 6 }), children: [] }] },
  ], loose: [{ ...piece(9, { shape: "group", label: "screenplay", format: "script", position: 3 }), children: [piece(10, { parentId: 9, position: 4 })] }] }],
};

test("titles reject blank input and trim accepted input", () => {
  for (const value of ["", " \t\n", " "]) expect(createTitle(value)).toBeNull();
  expect(createTitle("  Chapter 😀  ")).toBe("Chapter 😀");
});

test("positions append after the maximum of the target's siblings", () => {
  expect(nextPosition([])).toBe(0);
  expect(nextPosition([{ position: 8 }, { position: 2 }])).toBe(9);
  expect(targetPosition(tree, { kind: "universe" })).toBe(5);
  expect(targetPosition(tree, { kind: "container", universeId: 5 })).toBe(3);
  expect(targetPosition(tree, { kind: "piece", universeId: 5, container: null })).toBe(4);
  expect(targetPosition(tree, { kind: "piece", universeId: 5, container: { id: 1, kind: "book_series" } })).toBe(7);
  expect(targetPosition(tree, { kind: "child", universeId: 5, pieceId: 9, format: "script" })).toBe(5);
  expect(targetPosition(tree, { kind: "chapter", seriesId: 1, bookId: 2 })).toBe(8);
  expect(() => targetPosition(tree, { kind: "container", universeId: 9 })).toThrow();
  expect(() => targetPosition(tree, { kind: "piece", universeId: 5, container: { id: 9, kind: "collection" } })).toThrow();
  expect(() => targetPosition(tree, { kind: "child", universeId: 5, pieceId: 99, format: "prose" })).toThrow();
  expect(() => targetPosition(tree, { kind: "chapter", seriesId: 1, bookId: 9 })).toThrow();
});

test("labels follow container membership and group format", () => {
  expect(labelOptions({ kind: "piece", universeId: 5, container: { id: 1, kind: "book_series" } })).toEqual(["novel", "novella"]);
  expect(labelOptions({ kind: "piece", universeId: 5, container: { id: 1, kind: "tv_series" } })).toEqual(["episode", "tv_script"]);
  expect(labelOptions({ kind: "piece", universeId: 5, container: { id: 1, kind: "collection" } })).toContain("novel");
  expect(labelOptions({ kind: "child", universeId: 5, pieceId: 9, format: "script" })).toEqual(["scene"]);
  expect(labelOptions({ kind: "chapter", seriesId: 1, bookId: 2 })).toEqual(["chapter", "prologue", "epilogue"]);
  expect(labelOptions({ kind: "universe" })).toEqual([]);
  expect((["novel", "novella", "screenplay", "episode", "piece"] as const).map(label => pieceShape(label)))
    .toEqual(["group", "group", "group", "leaf", "leaf"]);
});

test("draft defaults, label-driven format, blanks, locks and cancel", () => {
  const started = draftTransition(null, { type: "start", target: { kind: "piece", universeId: 5, container: { id: 3, kind: "tv_series" } } });
  expect(started?.fields).toMatchObject({ label: "episode", format: "script" });
  expect(draftTransition(started, { type: "edit", fields: { format: "prose" } })?.fields.format).toBe("prose");
  expect(draftTransition(started, { type: "edit", fields: { label: "tv_script" } })?.fields.format).toBe("script");
  const collection = draftTransition(null, { type: "start", target: { kind: "piece", universeId: 5, container: null } });
  expect(draftTransition(collection, { type: "edit", fields: { label: "screenplay" } })?.fields.format).toBe("script");
  expect(draftTransition(started, { type: "submit" })).toMatchObject({ pending: false, error: "Enter a title." });
  const titled = draftTransition(started, { type: "edit", fields: { title: "New" } });
  const pending = draftTransition(titled, { type: "submit" });
  expect(pending?.pending).toBe(true);
  for (const action of [{ type: "cancel" }, { type: "submit" }, { type: "edit", fields: { title: "Other" } },
    { type: "start", target: { kind: "universe" } }] as const) expect(draftTransition(pending, action)).toBe(pending);
  const failed = draftTransition(pending, { type: "failed", message: "Offline" });
  expect(failed).toMatchObject({ fields: { title: "New" }, pending: false, error: "Offline" });
  expect(draftTransition(failed, { type: "edit", fields: { title: "Retry" } })?.error).toBeNull();
  expect(draftTransition(failed, { type: "cancel" })).toBeNull();
  expect(draftTransition(pending, { type: "created" })).toBeNull();
  expect(draftTransition(null, { type: "submit" })).toBeNull();
});

test("created items land under their parents", () => {
  const universe = addUniverse(tree, { id: 6, title: "New", position: 5 });
  expect(universe.universes[1]).toEqual({ id: 6, title: "New", position: 5, containers: [], loose: [] });
  const series = addContainer(tree, { id: 7, universeId: 5, title: "Saga", position: 3, kind: "book_series" });
  expect(series.universes[0]!.containers.map(item => item.id)).toEqual([1, 7]);
  expect(series.series.at(-1)).toEqual({ id: 7, title: "Saga", position: 3, books: [] });
  expect(addContainer(tree, { id: 8, universeId: 5, title: "C", position: 3, kind: "collection" }).series).toBe(tree.series);
  expect(addRoot(tree, piece(30)).universes[0]!.loose.map(item => item.id)).toEqual([9, 30]);
  expect(addRoot(tree, piece(31, { containerId: 1 })).universes[0]!.containers[0]!.pieces.map(item => item.id)).toEqual([20, 31]);
  expect(addChild(tree, piece(32, { parentId: 9 })).universes[0]!.loose[0]!.children.map(item => item.id)).toEqual([10, 32]);
  const chapter = { id: 4, title: "Next", position: 8, kind: "chapter" as const, head_rev: 0 };
  expect(addChapter(tree, { seriesId: 1, bookId: 2 }, chapter).series[0]!.books[0]!.chapters.map(item => item.id)).toEqual([3, 4]);
  const books = withBooks(tree, 1, [{ id: 2, series_id: 1, title: "Book", position: 3 }, { id: 5, series_id: 1, title: "Novel", position: 4 }]);
  expect(books.series[0]!.books.map(item => [item.id, item.chapters.length])).toEqual([[2, 1], [5, 0]]);
});
