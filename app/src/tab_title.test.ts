import { expect, test } from "bun:test";
import { getTabTitle } from "./tab_title";

test("tab title falls back when no chapter is open or ready", () => {
  expect(getTabTitle(null, false)).toBe("QuillAI");
  expect(getTabTitle(null, true)).toBe("QuillAI");
  expect(getTabTitle("Previous chapter", false)).toBe("QuillAI");
});

test("tab title follows chapter selection and title changes without altering the title", () => {
  expect(getTabTitle("First light", true)).toBe("First light — QuillAI");
  expect(getTabTitle("Nightfall", true)).toBe("Nightfall — QuillAI");
  expect(getTabTitle("夜 — <Renamed> & 😀", true)).toBe("夜 — <Renamed> & 😀 — QuillAI");
});
