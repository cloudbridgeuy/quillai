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
  const labels = [[counts.words, "word"], [counts.characters, "character"], [counts.lines, "line"]] as const;
  const prefix = selectedWords === null ? "" : `${format.format(selectedWords)} of `;
  return prefix + labels.map(([count, unit]) => `${format.format(count)} ${unit}${count === 1 ? "" : "s"}`).join(" · ");
}
