import { ADVERB_EXCEPTIONS } from "./adverb_exceptions";
import type { Rule } from "./engine";

const isLyAdverb = (lower: string) => lower.length > 4 && lower.endsWith("ly") && !ADVERB_EXCEPTIONS.has(lower);

export const adverbs: Rule = {
  id: "adverbs",
  labelKey: "rules.adverbs.label",
  languages: ["en"],
  defaultEnabled: true,
  params: [{ key: "max", kind: "int", labelKey: "rules.adverbs.max", default: 1, min: 0, max: 9 }],
  defaultScope: "both",
  check: (doc, config) => {
    const max = Number(config.params.max);
    return doc.sentences.flatMap((sentence) => {
      const found = doc.words.filter((word) => word.start >= sentence.start && word.start < sentence.end && isLyAdverb(word.lower));
      if (found.length <= max) return [];
      return found.map((word) => ({ ruleId: "adverbs", start: word.start, length: word.end - word.start, messageKey: "hints.adverb" as const, messageArgs: { word: word.text } }));
    });
  },
};
