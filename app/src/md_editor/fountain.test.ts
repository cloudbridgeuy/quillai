import { describe, expect, test } from "bun:test";
import Delta from "quill-delta";
import { toFountain } from "./fountain";

function scene(...lines: [string, string?][]): Delta {
  return lines.reduce((delta, [text, block]) => delta.insert(text).insert("\n", block ? { "script-block": block } : undefined), new Delta());
}

describe("toFountain", () => {
  test("writes a dialogue block with scene heading, action and transition", () => {
    const out = toFountain([
      scene(["int. hall - day", "scene-heading"], ["Anna waits."], ["anna", "character"], ["quietly", "parenthetical"], ["Hello.", "dialogue"], ["cut to:", "transition"]),
      scene(["Rain falls."]),
    ]);
    expect(out).toBe("INT. HALL - DAY\n\nAnna waits.\n\nANNA\n(quietly)\nHello.\n\nCUT TO:\n\nRain falls.\n");
  });

  test("forces elements that would parse as something else", () => {
    const out = toFountain([scene(["the yard", "scene-heading"], ["fade out.", "transition"], ["BANG"], ["INT. looks like a heading"], ["1234", "character"])]);
    expect(out).toBe(".THE YARD\n\n>FADE OUT.\n\n!BANG\n\n!INT. looks like a heading\n\n@1234\n");
  });

  test("skips empty lines and embeds", () => {
    const delta = new Delta().insert("\n").insert({ image: "x" }).insert("Hi").insert("\n").insert("  \n");
    expect(toFountain([delta])).toBe("Hi\n");
    expect(toFountain([])).toBe("");
  });
});
