import { ENGLISH_BOOKISMS } from "./english_bookisms";
import type { Rule } from "./engine";
import { SPANISH_BOOKISMS } from "./spanish_bookisms";

const WINDOW = 3;

export const saidBookisms: Rule = {
  id: "said-bookisms",
  labelKey: "rules.bookisms.label",
  languages: ["en", "es"],
  defaultEnabled: false,
  params: [],
  defaultScope: "narration",
  check: (doc) => {
    const near = new Set(
      doc.dialogue.flatMap((span) => [
        ...doc.words.filter((word) => word.start >= span.end).slice(0, WINDOW),
        ...doc.words.filter((word) => word.end <= span.start).slice(-WINDOW),
      ]),
    );
    return doc.words
      .filter((word) => near.has(word) && word.zone === "narration")
      .filter((word) => ENGLISH_BOOKISMS.has(word.lower) || SPANISH_BOOKISMS.has(word.lower))
      .map((word) => ({
        ruleId: "said-bookisms",
        start: word.start,
        length: word.end - word.start,
        messageKey: "hints.bookism" as const,
        messageArgs: { word: word.text },
      }));
  },
};
