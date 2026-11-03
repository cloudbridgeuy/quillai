import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { bookEpub } from "./epub_export";

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

const metadata = { id: "urn:uuid:123", modified: "2026-01-02T03:04:05Z", lang: "en" };

test("packages EPUB metadata, navigation and chapters in order", () => {
  const bytes = bookEpub("Book & more", [{ title: "First", delta: new Delta().insert("one\n") }, { title: "Empty", delta: new Delta().insert("\n") }, { title: "Empty Delta", delta: new Delta() }], metadata);
  const entries = files(bytes);
  expect(Object.keys(entries)).toEqual(["mimetype", "META-INF/container.xml", "OEBPS/content.opf", "OEBPS/nav.xhtml", "OEBPS/chapter-1.xhtml", "OEBPS/chapter-2.xhtml", "OEBPS/chapter-3.xhtml"]);
  const header = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  expect(header.getUint16(8, true)).toBe(0);
  expect(header.getUint16(26, true)).toBe(8);
  expect(header.getUint16(28, true)).toBe(0);
  expect(entries.mimetype).toBe("application/epub+zip");
  expect(new TextDecoder().decode(bytes.slice(38, 58))).toBe("application/epub+zip");
  expect(entries["META-INF/container.xml"]).toContain('<rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>');
  const opf = entries["OEBPS/content.opf"]!;
  expect(opf).toContain('unique-identifier="book-id"');
  expect(opf).toContain(`<dc:identifier id="book-id">${metadata.id}</dc:identifier>`);
  expect(opf).toContain("<dc:title>Book &amp; more</dc:title>");
  expect(opf).toContain(`<dc:language>${metadata.lang}</dc:language>`);
  expect(opf).toContain(`<meta property="dcterms:modified">${metadata.modified}</meta>`);
  expect(opf.match(/<manifest>(.*?)<\/manifest>/)?.[1]).toBe('<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/><item id="chapter-1" href="chapter-1.xhtml" media-type="application/xhtml+xml"/><item id="chapter-2" href="chapter-2.xhtml" media-type="application/xhtml+xml"/><item id="chapter-3" href="chapter-3.xhtml" media-type="application/xhtml+xml"/>');
  expect(opf.match(/<spine>(.*?)<\/spine>/)?.[1]).toBe('<itemref idref="chapter-1"/><itemref idref="chapter-2"/><itemref idref="chapter-3"/>');
  expect(entries["OEBPS/nav.xhtml"]).toContain('<ol><li><a href="chapter-1.xhtml">First</a></li><li><a href="chapter-2.xhtml">Empty</a></li><li><a href="chapter-3.xhtml">Empty Delta</a></li></ol>');
  expect(entries["OEBPS/chapter-1.xhtml"]).toContain("<body><h1>First</h1><p>one</p></body>");
  expect(entries["OEBPS/chapter-2.xhtml"]).toContain("<body><h1>Empty</h1><p></p></body>");
  expect(entries["OEBPS/chapter-3.xhtml"]).toContain("<body><h1>Empty Delta</h1></body>");
});

test("renders chapter text, blocks, lists and supported inline styles", () => {
  const delta = new Delta([{ insert: "A & <\u0001\ud800😀", attributes: { bold: true, italic: true, underline: true, strike: true, color: "red" } }, { insert: " plain\n", attributes: { link: "https://example.com" } }, ...[1, 2, 3].map(header => ({ insert: `Heading ${header}\n`, attributes: { header } })), { insert: "quote\n", attributes: { blockquote: true } }, { insert: "one\ntwo\n", attributes: { list: "ordered" } }, { insert: "bullet\nnext\n", attributes: { list: "bullet" } }, { insert: "last\n", attributes: { code: true } }]);
  const xml = files(bookEpub("Book", [{ title: "Title &", delta }], metadata))["OEBPS/chapter-1.xhtml"]!;
  for (const value of ["<h1>Title &amp;</h1>", "<h2>Heading 1</h2>", "<h3>Heading 2</h3>", "<h4>Heading 3</h4>", "<blockquote>quote</blockquote>", "<ol><li>one</li><li>two</li></ol>", "<ul><li>bullet</li><li>next</li></ul>", "<p>last</p>", "<strong>", "<em>", "<u>", "<s>", "A &amp; &lt;😀", " plain"]) expect(xml).toContain(value);
  expect(xml).not.toContain("color");
  expect(xml).not.toContain("https://example.com");
  expect(xml).not.toContain("\u0001");
});

test("uses newline block attributes and closes lists before empty paragraphs and final text", () => {
  const delta = new Delta()
    .insert("heading", { bold: true }).insert("\n", { header: 1 })
    .insert("quote").insert("\n", { blockquote: true })
    .insert("one").insert("\n", { list: "ordered" })
    .insert("two").insert("\n", { list: "ordered" })
    .insert("\n")
    .insert("again").insert("\n", { list: "ordered" })
    .insert("tail", { italic: true });
  const xml = files(bookEpub("Book", [{ title: "Title", delta }], metadata))["OEBPS/chapter-1.xhtml"]!;
  expect(xml).toContain("<body><h1>Title</h1><h2><strong>heading</strong></h2><blockquote>quote</blockquote><ol><li>one</li><li>two</li></ol><p></p><ol><li>again</li></ol><p><em>tail</em></p></body>");
});

test("escapes metadata and titles and removes invalid XML characters while preserving emoji", () => {
  const raw = "&<>\"'\u0000\u0001\ud800x\udc00\ufffe\uffff😀";
  const escaped = "&amp;&lt;&gt;&quot;&apos;x😀";
  const entries = files(bookEpub(raw, [{ title: raw, delta: new Delta().insert(`${raw}\n`) }], { ...metadata, id: raw, lang: raw }));
  const opf = entries["OEBPS/content.opf"]!;
  expect(opf).toContain(`<dc:identifier id="book-id">${escaped}</dc:identifier>`);
  expect(opf).toContain(`<dc:title>${escaped}</dc:title>`);
  expect(opf).toContain(`<dc:language>${escaped}</dc:language>`);
  expect(entries["OEBPS/nav.xhtml"]).toContain(`<h1>${escaped}</h1><ol><li><a href="chapter-1.xhtml">${escaped}</a></li></ol>`);
  const chapter = entries["OEBPS/chapter-1.xhtml"]!;
  expect(chapter).toContain(`lang="${escaped}" xml:lang="${escaped}"`);
  expect(chapter).toContain(`<title>${escaped}</title>`);
  expect(chapter).toContain(`<body><h1>${escaped}</h1><p>${escaped}</p></body>`);
});
