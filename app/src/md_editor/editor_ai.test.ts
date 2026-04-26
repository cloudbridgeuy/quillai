import { expect, test } from "bun:test";

import {
  getStatusPresentation,
  isTextOnlyDeltaDocument,
  rememberCursorRange,
  rememberEditSelection,
  resolveContinueSelection,
  resolveEditSelection,
  shouldClearFailedRunState,
  shouldPreservePendingEditSelection,
  shouldRestoreCancelledEditSelection,
  shouldRollbackFailedEditSelection,
} from "./editor_ai";

test("rememberEditSelection preserves the last non-empty selection across blur", () => {
  expect(rememberEditSelection(null, { index: 12, length: 5 })).toEqual({
    index: 12,
    length: 5,
  });
  expect(rememberEditSelection({ index: 12, length: 5 }, null)).toEqual({
    index: 12,
    length: 5,
  });
});

test("rememberEditSelection clears the remembered selection on an explicit collapsed caret", () => {
  expect(
    rememberEditSelection({ index: 12, length: 5 }, { index: 18, length: 0 }),
  ).toBeNull();
});

test("rememberCursorRange preserves the last caret or selection across blur", () => {
  expect(rememberCursorRange(null, { index: 7, length: 0 })).toEqual({
    index: 7,
    length: 0,
  });
  expect(rememberCursorRange({ index: 7, length: 0 }, null)).toEqual({
    index: 7,
    length: 0,
  });
});

test("resolveContinueSelection falls back to the remembered caret when the editor blurs", () => {
  expect(resolveContinueSelection({ index: 4, length: 0 }, { index: 12, length: 0 })).toEqual({
    index: 4,
    length: 0,
  });
  expect(resolveContinueSelection(null, { index: 12, length: 0 })).toEqual({
    index: 12,
    length: 0,
  });
  expect(resolveContinueSelection({ index: 4, length: 3 }, null)).toEqual({
    index: 7,
    length: 0,
  });
});

test("resolveEditSelection prefers the live selection and falls back to the remembered one", () => {
  expect(resolveEditSelection({ index: 4, length: 2 }, { index: 12, length: 5 })).toEqual({
    index: 4,
    length: 2,
  });
  expect(resolveEditSelection({ index: 4, length: 0 }, { index: 12, length: 5 })).toEqual({
    index: 12,
    length: 5,
  });
  expect(resolveEditSelection(null, { index: 12, length: 5 })).toEqual({
    index: 12,
    length: 5,
  });
});

test("shouldRestoreCancelledEditSelection only restores untouched cancelled edits", () => {
  expect(shouldRestoreCancelledEditSelection("edit", true, 5, 0)).toBe(true);
  expect(shouldRestoreCancelledEditSelection("edit", false, 5, 0)).toBe(false);
  expect(shouldRestoreCancelledEditSelection("edit", true, 5, 2)).toBe(false);
  expect(shouldRestoreCancelledEditSelection("generate", true, 5, 0)).toBe(false);
  expect(shouldRestoreCancelledEditSelection("edit", true, 0, 0)).toBe(false);
});

test("shouldRollbackFailedEditSelection rolls failed edit replacements back to the original text", () => {
  expect(shouldRollbackFailedEditSelection("edit", true, 5)).toBe(true);
  expect(shouldRollbackFailedEditSelection("edit", false, 5)).toBe(false);
  expect(shouldRollbackFailedEditSelection("generate", true, 5)).toBe(false);
  expect(shouldRollbackFailedEditSelection("edit", true, 0)).toBe(false);
});

test("shouldPreservePendingEditSelection keeps the original highlight when nothing was replaced", () => {
  expect(shouldPreservePendingEditSelection("edit", false, 5)).toBe(true);
  expect(shouldPreservePendingEditSelection("edit", true, 5)).toBe(false);
  expect(shouldPreservePendingEditSelection("generate", false, 5)).toBe(false);
  expect(shouldPreservePendingEditSelection("edit", false, 0)).toBe(false);
});

test("shouldClearFailedRunState only clears failed state after user edits", () => {
  expect(shouldClearFailedRunState({ status: "failed", action: "edit", error: "boom" }, "user")).toBe(true);
  expect(shouldClearFailedRunState({ status: "failed", action: "edit", error: "boom" }, "api")).toBe(false);
  expect(shouldClearFailedRunState({ status: "idle" }, "user")).toBe(false);
});

test("isTextOnlyDeltaDocument accepts text inserts and rejects embeds", () => {
  expect(isTextOnlyDeltaDocument({ ops: [{ insert: "Hello" }, { insert: " world" }] })).toBe(true);
  expect(isTextOnlyDeltaDocument({ ops: [{ insert: { image: "x" } }] })).toBe(false);
});

test("getStatusPresentation maps run states to compact UI status values", () => {
  expect(getStatusPresentation({ status: "idle" })).toEqual({
    tone: "idle",
    label: "Idle",
  });
  expect(getStatusPresentation({ status: "continuing", action: "generate" })).toEqual({
    tone: "running",
    label: "Continuing",
  });
  expect(getStatusPresentation({ status: "finished", action: "edit", outcome: "done" })).toEqual({
    tone: "success",
    label: "Done",
  });
  expect(getStatusPresentation({ status: "finished", action: "edit", outcome: "cancelled" })).toEqual({
    tone: "warning",
    label: "Cancelled",
  });
  expect(getStatusPresentation({ status: "failed", action: "edit", error: "boom" })).toEqual({
    tone: "error",
    label: "boom",
  });
});
