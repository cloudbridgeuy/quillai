import { defaultLocale, formatMessage } from "../i18n/messages";

type TextOperation = { insert?: unknown };

const words = new Intl.Segmenter(undefined, { granularity: "word" });
const graphemes = new Intl.Segmenter(undefined, { granularity: "grapheme" });
const format = new Intl.NumberFormat("en-US");

export function countText(input: string | readonly TextOperation[], terminalNewline = true) {
  let text = typeof input === "string" ? input : input.map((op) => typeof op.insert === "string" ? op.insert : "").join("");
  if (terminalNewline && text.endsWith("\n")) text = text.slice(0, -1);
  return {
    words: Array.from(words.segment(text)).filter((part) => part.isWordLike).length,
    characters: Array.from(graphemes.segment(text)).filter((part) => !/[\r\n\u2028\u2029]/u.test(part.segment)).length,
    lines: text.split("\n").length,
  };
}

export function formatTextCounts(counts: ReturnType<typeof countText>, selectedWords: number | null = null) {
  const labels = [[counts.words, "textCounts.word", "draft.words"], [counts.characters, "textCounts.character", "textCounts.characters"], [counts.lines, "textCounts.line", "textCounts.lines"]] as const;
  const prefix = selectedWords === null ? "" : formatMessage(defaultLocale, "textCounts.selected", { count: format.format(selectedWords) });
  return prefix + labels.map(([count, one, other]) => formatMessage(defaultLocale, count === 1 ? one : other, { count: format.format(count) })).join(formatMessage(defaultLocale, "textCounts.separator"));
}
