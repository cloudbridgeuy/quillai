import { describe, expect, test } from "bun:test";
import Delta from "quill-delta";
import { toFdx } from "./fdx";

function scene(...lines: [string, string?][]): Delta {
  return lines.reduce((delta, [text, block]) => delta.insert(text).insert("\n", block ? { "script-block": block } : undefined), new Delta());
}

const HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="no" ?>\n<FinalDraft DocumentType="Script" Template="No" Version="5">\n<Content>\n';
const TAIL = "</Content>\n</FinalDraft>\n";

function paragraph(type: string, text: string): string {
  return `<Paragraph Type="${type}"><Text>${text}</Text></Paragraph>\n`;
}

describe("toFdx", () => {
  test("writes every block type across two scenes in order", () => {
    const out = toFdx([
      scene(["INT. HALL - DAY", "scene-heading"], ["Anna waits."], ["ANNA", "character"], ["quietly", "parenthetical"], ["Hello.", "dialogue"], ["(softly)", "parenthetical"], ["CUT TO:", "transition"]),
      scene(["Rain falls."]),
    ]);
    expect(out).toBe(
      HEAD +
        paragraph("Scene Heading", "INT. HALL - DAY") +
        paragraph("Action", "Anna waits.") +
        paragraph("Character", "ANNA") +
        paragraph("Parenthetical", "(quietly)") +
        paragraph("Dialogue", "Hello.") +
        paragraph("Parenthetical", "(softly)") +
        paragraph("Transition", "CUT TO:") +
        paragraph("Action", "Rain falls.") +
        TAIL,
    );
  });

  test("escapes markup, drops illegal control characters and keeps emoji and tabs", () => {
    const out = toFdx([scene(['Tom & "Jerry" <run> 😀\u0001\tgo'])]);
    expect(out).toBe(HEAD + paragraph("Action", "Tom &amp; &quot;Jerry&quot; &lt;run&gt; 😀\tgo") + TAIL);
  });

  test("skips empty lines and embeds, and an empty script has no paragraphs", () => {
    const delta = new Delta().insert("\n").insert({ image: "x" }).insert("Hi").insert("\n").insert("  \n");
    expect(toFdx([delta])).toBe(HEAD + paragraph("Action", "Hi") + TAIL);
    expect(toFdx([])).toBe(HEAD + TAIL);
  });
});
