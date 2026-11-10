import type { Rule } from "./engine";

export const REDUNDANT_PHRASES = [
  "nodded his head",
  "nodded her head",
  "nodded their heads",
  "nodded my head",
  "shrugged his shoulders",
  "shrugged her shoulders",
  "shrugged their shoulders",
  "shrugged my shoulders",
  "blinked his eyes",
  "blinked her eyes",
  "blinked my eyes",
  "clapped his hands",
  "clapped her hands",
  "winked his eye",
  "winked her eye",
];

const PATTERN = new RegExp(`(?<![\\p{L}\\p{N}])(?:${REDUNDANT_PHRASES.map((phrase) => phrase.replaceAll(" ", "\\s+")).join("|")})(?![\\p{L}\\p{N}])`, "giu");

export const redundantPhrases: Rule = {
  id: "redundant-phrases",
  labelKey: "rules.redundantPhrases.label",
  languages: ["en"],
  defaultEnabled: true,
  params: [],
  defaultScope: "narration",
  check: (doc) =>
    [...doc.text.matchAll(PATTERN)].map((match) => ({
      ruleId: "redundant-phrases",
      start: match.index,
      length: match[0].length,
      messageKey: "hints.redundantPhrase" as const,
      messageArgs: { phrase: match[0].toLowerCase().replace(/\s+/g, " ") },
    })),
};
