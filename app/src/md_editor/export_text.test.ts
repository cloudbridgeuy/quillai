import { describe, expect, test } from "bun:test";
import Delta from "quill-delta";
import { toMarkdown, toPlainText } from "./export_text";

describe("toMarkdown", () => {
  test("writes a title, paragraphs, and inline emphasis with trailing spaces outside markers", () => {
    const delta = new Delta().insert("bold ", { bold: true, color: "red" }).insert("words ", { bold: true, color: "blue" }).insert("italic ", { italic: true })
      .insert("both ", { bold: true, italic: true }).insert("end\nNext paragraph\n");
    expect(toMarkdown("Chapter", delta)).toBe("# Chapter\n\n**bold words** *italic* ***both*** end\n\nNext paragraph\n");
  });

  test("writes headings at every level", () => {
    const delta = [1, 2, 3, 4, 5, 6].reduce((d, level) => d.insert(`H${level}`).insert("\n", { header: level }), new Delta());
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n# H1\n\n## H2\n\n### H3\n\n#### H4\n\n##### H5\n\n###### H6\n");
  });

  test("writes blockquotes", () => {
    expect(toMarkdown("Title", new Delta().insert("Quote").insert("\n", { blockquote: true }))).toBe("# Title\n\n> Quote\n");
  });

  test("writes fenced code blocks without escaping their contents", () => {
    const delta = new Delta().insert("const x = '*';").insert("\n\n", { "code-block": "javascript" })
      .insert("done();").insert("\n", { "code-block": "javascript" });
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n```javascript\nconst x = '*';\n\ndone();\n```\n");
    expect(toPlainText("Title", delta)).toBe("Title\n\nconst x = '*';\n\ndone();\n");
    expect(toMarkdown("Title", new Delta().insert("x").insert("\n", { "code-block": "js`\r\n" }))).toBe("# Title\n\n```js\nx\n```\n");
  });

  test("writes lists and their nesting", () => {
    const delta = new Delta().insert("One").insert("\n", { list: "ordered" })
      .insert("Nested").insert("\n", { list: "ordered", indent: 1 })
      .insert("Bullet").insert("\n", { list: "bullet" })
      .insert("Nested bullet").insert("\n", { list: "bullet", indent: 2 })
      .insert("Done").insert("\n", { list: "checked" })
      .insert("Todo").insert("\n", { list: "unchecked" });
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n1. One\n   1. Nested\n- Bullet\n    - Nested bullet\n- [x] Done\n- [ ] Todo\n");
  });

  test("writes links", () => {
    const delta = new Delta().insert("link", { link: "https://example.com" }).insert("\n");
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n[link](https://example.com)\n");
    expect(toPlainText("Title", delta)).toBe("Title\n\nlink\n");
  });

  test("writes inline code with backticks and whitespace", () => {
    const delta = new Delta().insert("a`b", { code: true }).insert(" ").insert(" a ", { code: true }).insert("\n");
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n``a`b`` `  a  `\n");
    expect(toPlainText("Title", delta)).toBe("Title\n\na`b  a \n");
  });

  test("writes strike", () => {
    const delta = new Delta().insert("gone", { strike: true }).insert("\n");
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n~~gone~~\n");
    expect(toPlainText("Title", delta)).toBe("Title\n\ngone\n");
  });

  test("drops unsupported and lore attributes while keeping text", () => {
    const delta = new Delta().insert("Words", { underline: true, font: "serif", size: "large", color: "red", background: "blue", script: "sub", "code-token": "keyword", lore: "id" })
      .insert(" more", { script: "super" }).insert("\n", { align: "center", direction: "rtl", indent: 1 });
    expect(toMarkdown("Title", delta)).toBe("# Title\n\nWords more\n");
  });

  test("escapes Markdown punctuation in title and text", () => {
    expect(toMarkdown("A *title*", new Delta().insert("[link] \\ # _ * `\n"))).toBe("# A \\*title\\*\n\n\\[link\\] \\\\ # \\_ \\* \\`\n");
  });

  test("keeps ordinary prose punctuation and midline list markers literal", () => {
    const text = "She stopped. It was well-known (to her)! See 1. item - item # > + = |.";
    expect(toMarkdown("Title", new Delta().insert(`${text}\n`))).toBe(`# Title\n\n${text}\n`);
  });

  test("escapes block markers only at the start after leading spaces", () => {
    for (const spaces of ["", "  "]) {
      for (const marker of ["#", ">", "+", "-", "=", "|"]) {
        expect(toMarkdown("Title", new Delta().insert(`${spaces}${marker} item\n`))).toBe(`# Title\n\n${spaces}\\${marker} item\n`);
      }
      for (const marker of ["1.", "12)"]) {
        expect(toMarkdown("Title", new Delta().insert(`${spaces}${marker} item\n`))).toBe(`# Title\n\n${spaces}${marker.slice(0, -1)}\\${marker.slice(-1)} item\n`);
      }
    }
  });

  test("escapes inline syntax anywhere and preserves single tildes", () => {
    const delta = new Delta().insert("text * _ [x] \\ ` < ~~strike~~ ~ single\n");
    expect(toMarkdown("Title", delta)).toBe("# Title\n\ntext \\* \\_ \\[x\\] \\\\ \\` \\< \\~\\~strike\\~\\~ ~ single\n");
  });

  test("keeps a midline hyphen literal across differently formatted runs", () => {
    const delta = new Delta().insert("prefix ", { bold: true }).insert("- item\n");
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n**prefix** - item\n");
  });

  test("writes a scene break", () => {
    const delta = new Delta().insert("Before\n").insert({ "scene-break": true }).insert("\nAfter\n");
    expect(toMarkdown("Title", delta)).toBe("# Title\n\nBefore\n\n***\n\nAfter\n");
    const persisted = new Delta().insert("Before\n").insert("\n", { "scene-break": 0 }).insert("After\n");
    expect(toMarkdown("Title", persisted)).toBe("# Title\n\nBefore\n\n***\n\nAfter\n");
    expect(toPlainText("Title", persisted)).toBe("Title\n\nBefore\n\n***\n\nAfter\n");
  });

  test("writes safe images and only the alt text for other sources", () => {
    const delta = new Delta().insert({ image: "https://example.com/a.png" }, { alt: "Cat" }).insert("\n")
      .insert({ image: "data:image/png;base64,abc" }, { alt: "Dog" }).insert("\n");
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n![Cat](https://example.com/a.png)\n\nDog\n");
    expect(toPlainText("Title", delta)).toBe("Title\n\nCat\n\nDog\n");
  });

  test("writes only safe video links", () => {
    const delta = new Delta().insert({ video: "https://example.com/v" }).insert("\n")
      .insert({ video: "javascript:alert(1)" }).insert("\n");
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n[video](https://example.com/v)\n");
    expect(toPlainText("Title", delta)).toBe("Title\n");
  });

  test("writes formula as inline code", () => {
    const delta = new Delta().insert({ formula: "x^2" }).insert("\n");
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n`x^2`\n");
    expect(toPlainText("Title", delta)).toBe("Title\n\nx^2\n");
  });

  test("writes table rows as joined cells", () => {
    const delta = new Delta().insert("A").insert("\n", { table: "row-1" }).insert("B").insert("\n", { table: "row-1" })
      .insert("\n", { table: "row-1" }).insert("\n", { table: "row-2" }).insert("D").insert("\n", { table: "row-2" });
    expect(toMarkdown("Title", delta)).toBe("# Title\n\nA | B | \n | D\n");
    expect(toPlainText("Title", delta)).toBe("Title\n\nA | B | \n | D\n");
  });

  test("escapes literal table pipes while keeping inserted cell separators", () => {
    const delta = new Delta().insert("| A | B").insert("\n", { table: "row-1" }).insert("C").insert("\n", { table: "row-1" });
    expect(toMarkdown("Title", delta)).toBe("# Title\n\n\\| A \\| B | C\n");
    expect(toPlainText("Title", delta)).toBe("Title\n\n| A | B | C\n");
  });

  test("writes script blocks as paragraphs", () => {
    const delta = new Delta().insert("INT. HALL").insert("\n", { "script-block": "scene-heading" })
      .insert("Hello").insert("\n", { "script-block": "dialogue" });
    expect(toMarkdown("Title", delta)).toBe("# Title\n\nINT. HALL\n\nHello\n");
  });

  test("writes an empty chapter", () => {
    expect(toMarkdown("Title", new Delta().insert("\n"))).toBe("# Title\n");
  });
});

describe("toPlainText", () => {
  test("keeps all punctuation without escaping", () => {
    const text = "1. - # > + = | * _ [x] \\ ` < ~~ ~ (word)!";
    expect(toPlainText(text, new Delta().insert(`${text}\n`))).toBe(`${text}\n\n${text}\n`);
  });

  test("writes title, text, paragraphs and list text without formatting", () => {
    const delta = new Delta().insert("Bold", { bold: true }).insert("\n")
      .insert("Paragraph\n")
      .insert("Item").insert("\n", { list: "bullet" });
    expect(toPlainText("Title", delta)).toBe("Title\n\nBold\n\nParagraph\n\nItem\n");
  });

  test("writes an empty chapter", () => {
    expect(toPlainText("Title", new Delta().insert("\n"))).toBe("Title\n");
  });
});
