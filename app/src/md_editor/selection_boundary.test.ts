import { expect, test } from "bun:test";
import { selectionBoundary } from "./selection_boundary";

test("plain text indices stay unchanged", () => {
  expect(selectionBoundary("plain", 2)).toBe(2);
});

test("the index before an emoji stays unchanged", () => {
  expect(selectionBoundary("a😀b", 1)).toBe(1);
});

test("the index after an emoji stays unchanged", () => {
  expect(selectionBoundary("a😀b", 3)).toBe(3);
});

test("the index between surrogate halves moves to the emoji start", () => {
  expect(selectionBoundary("a😀b", 2)).toBe(1);
});

test("only a selection end inside an emoji moves", () => {
  const text = "ab😀c";
  const index = selectionBoundary(text, 1);
  const end = selectionBoundary(text, 3);
  expect({ index, length: Math.max(0, end - index) }).toEqual({ index: 1, length: 1 });
});

test("adjacent emojis with a shared high surrogate keep separate boundaries", () => {
  expect([0, 1, 2, 3, 4].map((index) => selectionBoundary("😀😃", index))).toEqual([0, 0, 2, 2, 4]);
});
