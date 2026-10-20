import { expect, test } from "bun:test";
import { supportsLoreHighlights } from "./LoreWindow";

test("lore highlights require both the registry and constructor", () => {
  const css = { highlights: new Map() };
  class Highlight {}
  expect(supportsLoreHighlights(css, Highlight)).toBe(true);
  expect(supportsLoreHighlights(undefined, Highlight)).toBe(false);
  expect(supportsLoreHighlights({}, Highlight)).toBe(false);
  expect(supportsLoreHighlights({ highlights: null }, Highlight)).toBe(false);
  expect(supportsLoreHighlights(css, undefined)).toBe(false);
  expect(supportsLoreHighlights(css, {})).toBe(false);
  expect(supportsLoreHighlights(undefined, undefined)).toBe(false);
});
