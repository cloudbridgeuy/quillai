import type Delta from "quill-delta";
import { zipStore } from "./zip_store";

function escapeXml(value: string): string {
  return Array.from(value).filter(char => {
    const code = char.codePointAt(0)!;
    return code === 9 || code === 10 || code === 13 || (code >= 0x20 && code <= 0xd7ff) || (code >= 0xe000 && code <= 0xfffd) || (code >= 0x10000 && code <= 0x10ffff);
  }).join("").replace(/[&<>"']/g, char => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&apos;" })[char]!);
}

function run(text: string, attrs: Record<string, unknown> = {}): string {
  const props = `${attrs.bold ? "<w:b/>" : ""}${attrs.italic ? "<w:i/>" : ""}${attrs.underline ? '<w:u w:val="single"/>' : ""}${attrs.strike ? "<w:strike/>" : ""}`;
  return `<w:r>${props ? `<w:rPr>${props}</w:rPr>` : ""}<w:t xml:space="preserve">${escapeXml(text)}</w:t></w:r>`;
}

function paragraph(content: string, style = ""): string {
  return `<w:p>${style ? `<w:pPr><w:pStyle w:val="${style}"/></w:pPr>` : ""}${content}</w:p>`;
}

function body(delta: Delta): string {
  let result = "";
  let runs = "";
  let ordered = 0;
  for (const op of delta.ops) {
    if (typeof op.insert !== "string") continue;
    const attrs = op.attributes ?? {};
    const parts = op.insert.split("\n");
    for (let i = 0; i < parts.length; i++) {
      if (parts[i]) runs += run(parts[i]!, attrs);
      if (i < parts.length - 1) {
        const style = typeof attrs.header === "number" && [1, 2, 3].includes(attrs.header) ? `Heading${attrs.header}` : attrs.blockquote ? "Quote" : "";
        ordered = attrs.list === "ordered" ? ordered + 1 : 0;
        const marker = attrs.list ? run(attrs.list === "ordered" ? `${ordered}. ` : "- ") : "";
        result += paragraph(marker + runs, style);
        runs = "";
      }
    }
  }
  return result + (runs ? paragraph(runs) : "");
}

const contentTypes = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>`;
const rootRels = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>`;
const documentRels = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>`;
const styles = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">${[["Title", 36], ["Heading1", 32], ["Heading2", 28], ["Heading3", 24], ["Quote", 0]].map(([style, size]) => `<w:style w:type="paragraph" w:styleId="${style}"><w:name w:val="${style}"/><w:rPr>${size ? `<w:b/><w:sz w:val="${size}"/>` : "<w:i/>"}</w:rPr></w:style>`).join("")}</w:styles>`;

function packageDocx(content: string): Uint8Array {
  return zipStore([
    { name: "[Content_Types].xml", data: contentTypes },
    { name: "_rels/.rels", data: rootRels },
    { name: "word/document.xml", data: `<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>${content}<w:sectPr/></w:body></w:document>` },
    { name: "word/styles.xml", data: styles },
    { name: "word/_rels/document.xml.rels", data: documentRels },
  ]);
}

export function toDocx(title: string, delta: Delta): Uint8Array {
  return packageDocx(paragraph(run(title), "Title") + body(delta));
}

export function bookDocx(title: string, chapters: { title: string; delta: Delta }[]): Uint8Array {
  return packageDocx(paragraph(run(title), "Title") + chapters.map(chapter => paragraph(run(chapter.title), "Heading1") + body(chapter.delta)).join(""));
}
