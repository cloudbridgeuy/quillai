import { expect, test } from "bun:test";
import type { Container, Piece } from "./contract";
import { facadeBook, openUniverse, pieceRoots, pieceScope, sameScope, treeUniverses, type StoryUniverse } from "./story_tree";

const piece = (id: number, shape: Piece["shape"] = "leaf"): Piece => ({
  id, universeId: 5, containerId: null, parentId: null, title: `Piece ${id}`, position: 0,
  label: "short_story", format: "prose", shape,
});
const container = (id: number, kind: Container["kind"]) => ({ id, universeId: 5, title: `C${id}`, position: 0, kind, pieces: [] });
const universe: StoryUniverse = {
  id: 5, title: "World", position: 0,
  containers: [container(7, "collection"), container(4, "book_series"), container(8, "book_series")],
  loose: [{ ...piece(9), children: [] }],
};
const empty: StoryUniverse = { id: 6, title: "Empty", position: 1, containers: [container(2, "book_series")], loose: [] };

test("open universe follows a chapter's series or a piece's universe", () => {
  expect(openUniverse([empty, universe], { seriesId: 8, bookId: 1, chapterId: 1 })?.id).toBe(5);
  expect(openUniverse([empty, universe], { seriesId: 7, bookId: 1, chapterId: 1 })).toBeNull();
  expect(openUniverse([empty, universe], { universeId: 5, pieceId: 9 })?.id).toBe(5);
  expect(openUniverse([universe], null)).toBeNull();
});

test("the open universe is listed last and expanded, others collapsed; no scope expands all", () => {
  const third = { ...empty, id: 7 };
  expect(treeUniverses([universe, empty, third], { universeId: 5, pieceId: 9 }).map(({ universe, open }) => [universe.id, open]))
    .toEqual([[6, false], [7, false], [5, true]]);
  expect(treeUniverses([universe, empty], null).map(({ open }) => open)).toEqual([true, true]);
});

test("book series hide façade novels, other containers show every root", () => {
  const novel = { ...piece(11, "group"), label: "novel", children: [] };
  const novella = { ...piece(12, "group"), label: "novella", children: [] };
  const script = { ...novel, id: 13, format: "script" as const };
  expect(facadeBook(novel)).toBe(true);
  expect([novella, script, { ...novel, shape: "leaf" as const }].map(facadeBook)).toEqual([false, false, false]);
  expect(pieceRoots({ ...container(4, "book_series"), pieces: [novel, novella, script] })).toEqual([novella, script]);
  expect(pieceRoots({ ...container(7, "collection"), pieces: [novel, novella] })).toEqual([novel, novella]);
});

test("every leaf gets a piece scope in its universe, with or without a book series", () => {
  expect(pieceScope(universe, piece(9))).toEqual({ universeId: 5, pieceId: 9 });
  expect(pieceScope(universe, piece(10, "group"))).toBeNull();
  expect(pieceScope({ ...universe, containers: [container(7, "collection")] }, piece(9))).toEqual({ universeId: 5, pieceId: 9 });
});

test("scopes compare by kind and ids", () => {
  const chapter = { seriesId: 4, bookId: 1, chapterId: 9 };
  const loose = { universeId: 5, pieceId: 9 };
  expect(sameScope(chapter, { ...chapter })).toBe(true);
  expect(sameScope(chapter, { ...chapter, bookId: 2 })).toBe(false);
  expect(sameScope(loose, { ...loose, universeId: 8 })).toBe(true);
  expect(sameScope(chapter, loose)).toBe(false);
  expect(sameScope(null, loose)).toBe(false);
});
