import type Delta from "quill-delta";
import { zipStore } from "./zip_store";

function escapeXml(value: string): string {
  return Array.from(value).filter(char => {
    const code = char.codePointAt(0)!;
    return code === 9 || code === 10 || code === 13 || (code >= 0x20 && code <= 0xd7ff) || (code >= 0xe000 && code <= 0xfffd) || (code >= 0x10000 && code <= 0x10ffff);
  }).join("").replace(/[&<>"']/g, char => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&apos;" })[char]!);
}

function body(delta: Delta): string {
  let result = "";
  let runs = "";
  let list = "";
  const line = (attrs: Record<string, unknown>) => {
    const nextList = attrs.list === "ordered" ? "ol" : attrs.list === "bullet" ? "ul" : "";
    if (list !== nextList) {
      if (list) result += `</${list}>`;
      if (nextList) result += `<${nextList}>`;
      list = nextList;
    }
    const tag = list ? "li" : typeof attrs.header === "number" && [1, 2, 3].includes(attrs.header) ? `h${attrs.header + 1}` : attrs.blockquote ? "blockquote" : "p";
    result += `<${tag}>${runs}</${tag}>`;
    runs = "";
  };
  for (const op of delta.ops) {
    if (typeof op.insert !== "string") continue;
    const attrs = op.attributes ?? {};
    const parts = op.insert.split("\n");
    for (let i = 0; i < parts.length; i++) {
      if (parts[i]) {
        let text = escapeXml(parts[i]!);
        for (const [key, tag] of [["bold", "strong"], ["italic", "em"], ["underline", "u"], ["strike", "s"]]) if (attrs[key!]) text = `<${tag}>${text}</${tag}>`;
        runs += text;
      }
      if (i < parts.length - 1) line(attrs);
    }
  }
  if (runs) line({});
  return result + (list ? `</${list}>` : "");
}

function xhtml(title: string, lang: string, body: string): string {
  return `<?xml version="1.0" encoding="UTF-8"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="${escapeXml(lang)}" xml:lang="${escapeXml(lang)}"><head><title>${escapeXml(title)}</title></head><body>${body}</body></html>`;
}

export function bookEpub(title: string, chapters: { title: string; delta: Delta }[], metadata: { id: string; modified: string; lang: string }): Uint8Array {
  const { id, modified, lang } = metadata;
  const chapterName = (index: number) => `chapter-${index + 1}.xhtml`;
  const manifest = chapters.map((_, i) => `<item id="chapter-${i + 1}" href="${chapterName(i)}" media-type="application/xhtml+xml"/>`).join("");
  const spine = chapters.map((_, i) => `<itemref idref="chapter-${i + 1}"/>`).join("");
  const nav = chapters.map((chapter, i) => `<li><a href="${chapterName(i)}">${escapeXml(chapter.title)}</a></li>`).join("");
  return zipStore([
    { name: "mimetype", data: "application/epub+zip" },
    { name: "META-INF/container.xml", data: `<?xml version="1.0" encoding="UTF-8"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>` },
    { name: "OEBPS/content.opf", data: `<?xml version="1.0" encoding="UTF-8"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="book-id"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="book-id">${escapeXml(id)}</dc:identifier><dc:title>${escapeXml(title)}</dc:title><dc:language>${escapeXml(lang)}</dc:language><meta property="dcterms:modified">${escapeXml(modified)}</meta></metadata><manifest><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>${manifest}</manifest><spine>${spine}</spine></package>` },
    { name: "OEBPS/nav.xhtml", data: xhtml(title, lang, `<nav epub:type="toc"><h1>${escapeXml(title)}</h1><ol>${nav}</ol></nav>`) },
    ...chapters.map((chapter, i) => ({ name: `OEBPS/${chapterName(i)}`, data: xhtml(chapter.title, lang, `<h1>${escapeXml(chapter.title)}</h1>${body(chapter.delta)}`) })),
  ]);
}
