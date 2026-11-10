import type { ParagraphAnalysis } from "./shared_pass";
import { SPANISH_TAG_VERBS } from "./spanish_tag_verbs";

export type DashTag = { open: number; end: number };

const DASH_PARAGRAPH = /^\s*—/u;

export function dashTags(doc: ParagraphAnalysis): DashTag[] {
  if (!DASH_PARAGRAPH.test(doc.text)) return [];
  const dashes = Array.from(doc.text.matchAll(/—/gu), (match) => match.index).slice(1);
  return dashes
    .filter((_, i) => i % 2 === 0)
    .map((open, i) => ({ open, end: dashes[i * 2 + 1] ?? doc.text.length }))
    .filter((tag) => doc.words.filter((word) => word.start > tag.open && word.end <= tag.end).slice(0, 2).some((word) => SPANISH_TAG_VERBS.has(word.lower)));
}
