import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { diffDocuments } from "./document_diff";

test("Unicode-safe text differences preserve formatting, embeds, and separate edits", () => {
  const before = new Delta().insert("😀🚀", { bold: true }).insert({ image: "old" }).insert("\n");
  const after = new Delta().insert("😀😀a", { italic: true }).insert("🚀", { bold: true })
    .insert({ image: "new" }, { link: "image" }).insert("🌙\n");
  const change = diffDocuments(before, after);
  expect(change.ops.every((op) => typeof op.insert !== "string" || op.insert.isWellFormed())).toBe(true);
  expect(before.compose(change)).toEqual(after);
  expect(diffDocuments(after, after).ops).toEqual([]);
  expect(after.compose(diffDocuments(after, before))).toEqual(before);
});
