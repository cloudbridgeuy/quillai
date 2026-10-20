import { expect, test } from "bun:test";
import { canApplyEdit, getSavePresentation } from "./editor_ai";

test("Enter and Apply require a selection, an available chapter, and no active run", () => {
  expect(canApplyEdit(false, true, true, false)).toBe(true);
  expect(canApplyEdit(false, false, true, false)).toBe(false);
  expect(canApplyEdit(true, true, true, false)).toBe(false);
  expect(canApplyEdit(false, true, false, false)).toBe(false);
  expect(canApplyEdit(false, true, true, true)).toBe(false);
});

test("save label removes only the chapter prefix and keeps failures prominent", () => {
  expect(getSavePresentation("A chapter · Saved", "A chapter", false)).toEqual({ label: "Saved", failed: false });
  expect(getSavePresentation("A chapter · Unsaved changes", "A chapter", false)).toEqual({ label: "Unsaved changes", failed: false });
  const failure = "Chapter switch failed: Save stopped: unavailable. Current chapter retained.";
  expect(getSavePresentation(failure, "A chapter", false)).toEqual({ label: "Not saved", failed: true, detail: failure });
  expect(getSavePresentation("Save stopped: export local work", "A chapter", true)).toEqual({
    label: "Not saved", failed: true, detail: "Save stopped: export local work",
  });
  expect(getSavePresentation("A chapter · Connection lost", "A chapter", true)).toEqual({
    label: "Not saved", failed: true, detail: "Connection lost",
  });
  expect(getSavePresentation("A chapter · Saved · Recovery storage unavailable", "A chapter", false)).toEqual({
    label: "Not saved", failed: true, detail: "Saved · Recovery storage unavailable",
  });
  expect(getSavePresentation("Loading chapter…", null, false)).toEqual({ label: "Loading chapter…", failed: false });
});
