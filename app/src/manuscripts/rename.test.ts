import { expect, test } from "bun:test";
import type { ManuscriptTreeData } from "./create_draft";
import { parseTitle, renameTree, type RenameKind } from "./rename";

test("rename titles trim, reject blanks, and skip unchanged titles", () => {
  expect(parseTitle("  New 😀  ", "Old")).toEqual({ status: "valid", title: "New 😀" });
  for (const input of ["", " \t\n", " "]) expect(parseTitle(input, "Old").status).toBe("invalid");
  expect(parseTitle("  Same  ", "Same")).toEqual({ status: "unchanged" });
  expect(parseTitle("a".repeat(10000), "Old").status).toBe("valid");
});

const piece = { id: 1, title: "Piece", position: 2, universeId: 1, containerId: null, parentId: null, label: "piece", format: "prose" as const, shape: "leaf" as const };
const tree: ManuscriptTreeData = {
  series: [{ id: 1, title: "Series", position: 0, books: [{ id: 1, series_id: 1, title: "Book", position: 1,
    chapters: [{ id: 1, title: "Chapter", kind: "chapter", position: 2, head_rev: 7 }] }] }],
  universes: [{ id: 1, title: "Universe", position: 0, containers: [{ id: 1, title: "Series", position: 0, universeId: 1, kind: "book_series",
    pieces: [{ ...piece, id: 2, containerId: 1, children: [{ ...piece, id: 3, parentId: 2 }] }] }], loose: [{ ...piece, children: [] }] }],
};

for (const kind of ["series", "book", "chapter", "piece"] satisfies RenameKind[]) {
  test(`rename ${kind} changes only the matching title, including the series facade`, () => {
    const expected = structuredClone(tree);
    if (kind === "series") {
      expected.series[0]!.title = "New";
      expected.universes[0]!.containers[0]!.title = "New";
    } else if (kind === "book") expected.series[0]!.books[0]!.title = "New";
    else if (kind === "chapter") expected.series[0]!.books[0]!.chapters[0]!.title = "New";
    else expected.universes[0]!.loose[0]!.title = "New";
    const before = structuredClone(tree);
    expect(renameTree(tree, kind, 1, "New")).toEqual(expected);
    expect(tree).toEqual(before);
    expect(renameTree(tree, kind, 999, "New")).toEqual(tree);
  });
}

test("rename finds contained roots and children without changing siblings", () => {
  for (const id of [2, 3]) {
    const expected = structuredClone(tree);
    const root = expected.universes[0]!.containers[0]!.pieces[0]!;
    (id === 2 ? root : root.children[0]!).title = "Renamed";
    const renamed = renameTree(tree, "piece", id, "Renamed");
    expect(renamed).toEqual(expected);
    expect(renamed.series).toBe(tree.series);
    expect(renamed.universes[0]!.loose[0]).toBe(tree.universes[0]!.loose[0]);
  }
});
