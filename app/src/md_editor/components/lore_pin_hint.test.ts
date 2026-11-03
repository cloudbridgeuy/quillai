import { expect, test } from "bun:test";
import { PIN_HINT } from "./lore_pin_hint";

test("pin hint explains AI request inclusion", () => {
  expect(PIN_HINT).toBe("Sent with every AI request unless set to Leave out, even when the text doesn't mention it, and never dropped for space.");
});
