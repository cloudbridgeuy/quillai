import { expect, test } from "bun:test";
import { panelShortcut, transitionPanel } from "./editor_panels";

test("panels toggle, switch sides, and close without simultaneous drawers", () => {
  expect(transitionPanel(null, "manuscript")).toBe("manuscript");
  expect(transitionPanel("manuscript", "context")).toBe("context");
  expect(transitionPanel("context", "context")).toBeNull();
  expect(transitionPanel("manuscript", "manuscript")).toBeNull();
  for (const state of [null, "manuscript", "context"] as const) {
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
