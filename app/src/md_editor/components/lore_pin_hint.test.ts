import { expect, test } from "bun:test";
import { PIN_HINT, TERMS_HINT } from "./lore_pin_hint";

test("pin hint explains AI request inclusion", () => {
  expect(PIN_HINT).toBe("Sent with every AI request unless set to Leave out, even when the text doesn't mention it, and never dropped for space.");
});

test("terms hint explains lore matching", () => {
  expect(TERMS_HINT).toBe("Whole words or phrases, any case. When the name, an alias or a keyword appears in the text, this entry is sent to the AI.");
});
