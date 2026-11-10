import type { Rule } from "./engine";

const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

export const watchWords: Rule = {
  id: "watch-words",
  labelKey: "rules.watch.label",
  languages: ["en", "es"],
  defaultEnabled: false,
  params: [{ key: "words", kind: "words", labelKey: "rules.watch.words", default: [] }],
  defaultScope: "both",
  check: (doc, config) => {
    const listed = (Array.isArray(config.params.words) ? config.params.words : [])
      .map((word) => word.trim())
      .filter((word) => word !== "")
      .sort((a, b) => b.length - a.length)
      .map((word) => escape(word).replace(/\s+/g, "\\s+"));
    if (listed.length === 0) return [];
    const pattern = new RegExp(`(?<![\\p{L}\\p{N}])(?:${listed.join("|")})(?![\\p{L}\\p{N}])`, "giu");
    return [...doc.text.matchAll(pattern)].map((match) => ({
      ruleId: "watch-words",
      start: match.index,
      length: match[0].length,
      messageKey: "hints.watch" as const,
      messageArgs: { word: match[0] },
    }));
  },
};
