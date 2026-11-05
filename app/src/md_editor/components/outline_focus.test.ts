import { expect, test } from "bun:test";
import { outlineFocusTarget } from "./outline_focus";

const base = { focusWasInOutline: true, activeElementIsBody: true, hasPreviousElement: true, hasPressedRow: true };

test("restores previous element, else pressed row, else region, only when focus fell out", () => {
  expect(outlineFocusTarget(base)).toBe("previous");
  expect(outlineFocusTarget({ ...base, hasPreviousElement: false })).toBe("row");
  expect(outlineFocusTarget({ ...base, hasPreviousElement: false, hasPressedRow: false })).toBe("region");
  expect(outlineFocusTarget({ ...base, focusWasInOutline: false })).toBe("none");
  expect(outlineFocusTarget({ ...base, activeElementIsBody: false })).toBe("none");
});
