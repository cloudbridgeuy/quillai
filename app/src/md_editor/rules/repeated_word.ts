import { FUNCTION_WORDS } from "./function_words";
import type { Rule } from "./engine";

export const repeatedWord: Rule = {
  id: "repeated-word",
  labelKey: "rules.repeatedWord.label",
  languages: ["en"],
  defaultEnabled: true,
  params: [{ key: "window", kind: "int", labelKey: "rules.repeatedWord.window", default: 20, min: 1, max: 100 }],
  defaultScope: "narration",
  check: (doc, config) => {
    const limit = Number(config.params.window);
    const lastSeen = new Map<string, number>();
    return doc.words.flatMap((word, index) => {
      const key = word.lower.replaceAll("’", "'");
      const previous = lastSeen.get(key);
      lastSeen.set(key, index);
      if (FUNCTION_WORDS.has(key) || previous === undefined || index - previous > limit) return [];
      return [{ ruleId: "repeated-word", start: word.start, length: word.end - word.start, messageKey: "hints.repeatedWord" as const, messageArgs: { word: word.text } }];
    });
  },
};
