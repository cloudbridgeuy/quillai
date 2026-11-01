import { expect, test } from "bun:test";
import { aiShortcut, aiShortcutFocusAllowed, panelShortcut, transitionPanel } from "./editor_panels";

test("panels toggle, switch sides, and close without simultaneous drawers", () => {
  expect(transitionPanel(null, "manuscript")).toBe("manuscript");
  expect(transitionPanel("manuscript", "context")).toBe("context");
  expect(transitionPanel("context", "context")).toBeNull();
  expect(transitionPanel("manuscript", "manuscript")).toBeNull();
  expect(transitionPanel("context", "settings")).toBe("settings");
  expect(transitionPanel("settings", "settings")).toBeNull();
  expect(transitionPanel("settings", "manuscript")).toBe("manuscript");
  for (const state of [null, "manuscript", "context", "settings"] as const) {
    expect(transitionPanel(state, "close")).toBeNull();
  }
});

test("panel shortcuts accept Ctrl or Cmd brackets and Escape, not typing or composition", () => {
  const event = {
    key: "]", ctrlKey: false, metaKey: false, altKey: false,
    shiftKey: false, repeat: false, isComposing: false,
  };
  for (const modifier of [{ ctrlKey: true }, { metaKey: true }]) {
    expect(panelShortcut({ ...event, ...modifier })).toBe("context");
    expect(panelShortcut({ ...event, ...modifier, key: "[" })).toBe("manuscript");
  }
  expect(panelShortcut({ ...event, key: "Escape" })).toBe("close");
  expect(panelShortcut(event)).toBeNull();
  for (const ignored of [{ altKey: true }, { shiftKey: true }, { repeat: true }, { isComposing: true }]) {
    expect(panelShortcut({ ...event, ctrlKey: true, ...ignored })).toBeNull();
  }
  expect(panelShortcut({ ...event, ctrlKey: true, key: "a" })).toBeNull();
});

test("AI shortcuts accept Ctrl or Cmd and ignore extra modifiers, repeats, and composition", () => {
  const event = {
    key: "Enter", ctrlKey: false, metaKey: false, altKey: false,
    shiftKey: false, repeat: false, isComposing: false,
  };
  for (const modifier of [{ ctrlKey: true }, { metaKey: true }]) {
    expect(aiShortcut({ ...event, ...modifier })).toBe("continue");
    expect(aiShortcut({ ...event, ...modifier, key: "k" })).toBe("edit");
    expect(aiShortcut({ ...event, ...modifier, key: "K" })).toBe("edit");
    expect(aiShortcut({ ...event, ...modifier, key: "a" })).toBeNull();
    for (const ignored of [{ altKey: true }, { shiftKey: true }, { repeat: true }, { isComposing: true }]) {
      expect(aiShortcut({ ...event, ...modifier, ...ignored })).toBeNull();
      expect(aiShortcut({ ...event, ...modifier, ...ignored, key: "k" })).toBeNull();
    }
  }
  expect(aiShortcut(event)).toBeNull();
  expect(aiShortcut({ ...event, key: "k" })).toBeNull();
});

test("AI shortcut focus accepts the manuscript and non-text controls, excluding other editors", () => {
  expect(aiShortcutFocusAllowed(true, true, false)).toBe(true);
  expect(aiShortcutFocusAllowed(false, false, false)).toBe(true);
  expect(aiShortcutFocusAllowed(false, true, false)).toBe(false);
});

test("AI shortcuts leave dialog and popover controls to their own handlers", () => {
  expect(aiShortcutFocusAllowed(false, false, true)).toBe(false);
  expect(aiShortcutFocusAllowed(false, true, true)).toBe(false);
  expect(aiShortcutFocusAllowed(true, true, true)).toBe(false);
});
