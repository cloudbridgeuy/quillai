import { expect, test } from "bun:test";
import { offBreak, scriptKeyAction } from "./script_keys";

const layout = [{ sceneId: 1, start: 1, length: 5 }, { sceneId: 2, start: 7, length: 4 }];
const key = (name: string, modifiers: Partial<Record<"shiftKey" | "altKey" | "ctrlKey" | "metaKey", boolean>> = {}) => ({
  key: name, shiftKey: false, altKey: false, ctrlKey: false, metaKey: false, ...modifiers,
});

test("arrows step over the break line", () => {
  expect(scriptKeyAction(key("ArrowRight"), { index: 5, length: 0 }, layout)).toEqual({ kind: "move", index: 7 });
  expect(scriptKeyAction(key("ArrowLeft"), { index: 7, length: 0 }, layout)).toEqual({ kind: "move", index: 5 });
  expect(scriptKeyAction(key("ArrowLeft"), { index: 1, length: 0 }, layout)).toEqual({ kind: "move", index: 1 });
  expect(scriptKeyAction(key("ArrowRight", { shiftKey: true }), { index: 5, length: 0 }, layout)).toEqual({ kind: "pass" });
  expect(scriptKeyAction(key("ArrowRight"), { index: 4, length: 0 }, layout)).toEqual({ kind: "pass" });
});

test("merging keys at scene edges are blocked", () => {
  expect(scriptKeyAction(key("Backspace"), { index: 7, length: 0 }, layout)).toEqual({ kind: "block" });
  expect(scriptKeyAction(key("Backspace"), { index: 1, length: 0 }, layout)).toEqual({ kind: "block" });
  expect(scriptKeyAction(key("Delete"), { index: 5, length: 0 }, layout)).toEqual({ kind: "block" });
  expect(scriptKeyAction(key("Enter"), { index: 5, length: 0 }, layout)).toEqual({ kind: "pass" });
  expect(scriptKeyAction(key("Backspace"), { index: 8, length: 0 }, layout)).toEqual({ kind: "pass" });
});

test("editing keys over a selection across a break are blocked", () => {
  const across = { index: 4, length: 4 };
  for (const name of ["Delete", "Backspace", "Enter", "x"]) expect(scriptKeyAction(key(name), across, layout)).toEqual({ kind: "block" });
  expect(scriptKeyAction(key("x", { metaKey: true }), across, layout)).toEqual({ kind: "block" });
  expect(scriptKeyAction(key("c", { metaKey: true }), across, layout)).toEqual({ kind: "pass" });
  expect(scriptKeyAction(key("ArrowDown", { shiftKey: true }), across, layout)).toEqual({ kind: "pass" });
  expect(scriptKeyAction(key("Delete"), { index: 2, length: 3 }, layout)).toEqual({ kind: "pass" });
});

test("a caret on a break moves into the scene it came toward", () => {
  expect(offBreak(6, 5, layout)).toBe(7);
  expect(offBreak(6, 8, layout)).toBe(5);
  expect(offBreak(0, null, layout)).toBe(1);
  expect(offBreak(3, null, layout)).toBeNull();
});
