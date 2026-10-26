import { expect, test } from "bun:test";
import type { Container, Piece } from "./contract";
import { loreSeriesId, openUniverse, pieceScope, sameScope, universeRoots, type StoryUniverse } from "./story_tree";

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

test("lore scope is the first book series in the universe", () => {
  expect(loreSeriesId(universe.containers)).toBe(4);
  expect(loreSeriesId([container(7, "collection")])).toBeNull();
});

test("open universe follows a chapter's series or a piece's universe", () => {
  expect(openUniverse([empty, universe], { seriesId: 8, bookId: 1, chapterId: 1 })?.id).toBe(5);
  expect(openUniverse([empty, universe], { seriesId: 7, bookId: 1, chapterId: 1 })).toBeNull();
  expect(openUniverse([empty, universe], { seriesId: 2, universeId: 5, pieceId: 9 })?.id).toBe(5);
  expect(openUniverse([universe], null)).toBeNull();
});

test("universe roots omit book series and empty universes", () => {
  expect(universeRoots(universe)).toEqual({ containers: [universe.containers[0]!], loose: universe.loose });
  expect(universeRoots(empty)).toBeNull();
  expect(universeRoots(null)).toBeNull();
});

test("only leaves in a universe with a book series get a piece scope", () => {
  expect(pieceScope(universe, piece(9))).toEqual({ seriesId: 4, universeId: 5, pieceId: 9 });
  expect(pieceScope(universe, piece(10, "group"))).toBeNull();
  expect(pieceScope({ ...universe, containers: [container(7, "collection")] }, piece(9))).toBeNull();
});

test("scopes compare by kind and ids", () => {
  const chapter = { seriesId: 4, bookId: 1, chapterId: 9 };
  const loose = { seriesId: 4, universeId: 5, pieceId: 9 };
  expect(sameScope(chapter, { ...chapter })).toBe(true);
  expect(sameScope(chapter, { ...chapter, bookId: 2 })).toBe(false);
  expect(sameScope(loose, { ...loose, seriesId: 8 })).toBe(true);
  expect(sameScope(chapter, loose)).toBe(false);
  expect(sameScope(null, loose)).toBe(false);
});
