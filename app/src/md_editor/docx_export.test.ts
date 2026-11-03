import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { bookDocx, toDocx } from "./docx_export";

function files(bytes: Uint8Array): Record<string, string> {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const decoder = new TextDecoder();
  const result: Record<string, string> = {};
  let offset = 0;
  while (view.getUint32(offset, true) === 0x04034b50) {
    const size = view.getUint32(offset + 18, true);
    const nameLength = view.getUint16(offset + 26, true);
    const name = decoder.decode(bytes.slice(offset + 30, offset + 30 + nameLength));
    const start = offset + 30 + nameLength + view.getUint16(offset + 28, true);
    result[name] = decoder.decode(bytes.slice(start, start + size));
    offset = start + size;
  }
  return result;
}

test("chapter preserves lines, supported styles and text", () => {
  const delta = new Delta([{ insert: "A & <", attributes: { bold: true, italic: true, underline: true, strike: true, color: "red" } }, { insert: "B>😀\u0001\ud800'\"", attributes: { link: "https://example.com" } }, { insert: "\n", attributes: { header: 2 } }, { insert: "\nquote" }, { insert: "\n", attributes: { blockquote: true } }, { insert: "item\n", attributes: { list: "bullet" } }, { insert: "ordered\n", attributes: { list: "ordered" } }]);
  const packageFiles = files(toDocx("Title &", delta));
  expect(Object.keys(packageFiles)).toEqual(["[Content_Types].xml", "_rels/.rels", "word/document.xml", "word/styles.xml", "word/_rels/document.xml.rels"]);
  const xml = packageFiles["word/document.xml"]!;
  expect(xml).toContain('<w:pStyle w:val="Title"/>');
  expect(xml).toContain('<w:pStyle w:val="Heading2"/>');
  expect(xml).toContain('<w:pStyle w:val="Quote"/>');
  expect(xml).toContain("<w:b/><w:i/><w:u w:val=\"single\"/><w:strike/>");
  expect(xml).toContain("A &amp; &lt;");
  expect(xml).toContain("B&gt;😀&apos;&quot;");
  expect(xml).toContain("- ");
  expect(xml).toContain("1. ");
  expect(xml).toContain("<w:p></w:p>");
  expect(xml).not.toContain("color");
  expect(packageFiles["word/styles.xml"]).toContain('w:styleId="Heading3"');
  expect(packageFiles["word/_rels/document.xml.rels"]).toContain('Target="styles.xml"');
  expect(packageFiles["[Content_Types].xml"]).toContain("word/styles.xml");
});

test("ordered lists count consecutive items and restart after other lines", () => {
  for (const attributes of [{}, { header: 1 }, { list: "bullet" }, { blockquote: true }]) {
    const delta = new Delta().insert("one\ntwo\nthree\n", { list: "ordered" }).insert("break\n", attributes).insert("again\n", { list: "ordered" });
    const xml = files(toDocx("Title", delta))["word/document.xml"]!;
    expect(Array.from(xml.matchAll(/<w:t xml:space="preserve">(\d+\. )<\/w:t>/g), match => match[1])).toEqual(["1. ", "2. ", "3. ", "1. "]);
  }
});

test("book emits title and every chapter in order", () => {
  const xml = files(bookDocx("Book", [{ title: "First", delta: new Delta().insert("one\n") }, { title: "Empty", delta: new Delta().insert("\n") }]))["word/document.xml"]!;
  expect(xml.indexOf("Book")).toBeLessThan(xml.indexOf("First"));
  expect(xml.indexOf("First")).toBeLessThan(xml.indexOf("Empty"));
  expect(xml.match(/w:val="Heading1"/g)?.length).toBe(2);
  expect(xml).toContain("one");
  const empty = files(bookDocx("Empty book", []))["word/document.xml"]!;
  expect(empty).toContain("Empty book");
  const headings = files(toDocx("T", new Delta([1, 2, 3, 1.5].map(header => ({ insert: "x\n", attributes: { header } })))))["word/document.xml"]!;
  for (const level of [1, 2, 3]) expect(headings).toContain(`w:val="Heading${level}"`);
  expect(headings).not.toContain("Heading1.5");
});
