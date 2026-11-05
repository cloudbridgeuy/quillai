import { expect, test } from "bun:test";
import { nextOccurrence } from "./lore_occurrence";

test("first click goes to the first match", () => {
  expect(nextOccurrence([4, 10, 20], null)).toBe(4);
});

test("later clicks advance to the next match", () => {
  expect(nextOccurrence([4, 10, 20], 4)).toBe(10);
  expect(nextOccurrence([4, 10, 20], 10)).toBe(20);
});

test("wraps to the first match after the last", () => {
  expect(nextOccurrence([4, 10, 20], 20)).toBe(4);
});

test("a single match stays on itself", () => {
  expect(nextOccurrence([7], null)).toBe(7);
  expect(nextOccurrence([7], 7)).toBe(7);
});

test("falls back to the first match when the current one is gone", () => {
  expect(nextOccurrence([4, 10, 20], 11)).toBe(4);
});

test("sorts unordered input and handles no matches", () => {
  expect(nextOccurrence([20, 4, 10], 4)).toBe(10);
  expect(nextOccurrence([], null)).toBeUndefined();
});
