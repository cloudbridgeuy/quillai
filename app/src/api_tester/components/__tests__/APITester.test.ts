import { describe, test, expect } from "bun:test";
import { buttonLabel } from "../APITester";

describe("buttonLabel", () => {
  test("returns 'Sending...' when loading", () => {
    expect(buttonLabel(true)).toBe("Sending...");
  });

  test("returns 'Send' when not loading", () => {
    expect(buttonLabel(false)).toBe("Send");
  });
});
