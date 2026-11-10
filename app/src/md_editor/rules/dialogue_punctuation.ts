import type { Rule } from "./engine";

const PERIOD_BEFORE_LOWERCASE_TAG = /^\.["”]$/u;
const LOWERCASE_START = /^\s+\p{Ll}/u;

export const dialoguePunctuation: Rule = {
  id: "dialogue-punctuation",
  labelKey: "rules.dialoguePunctuation.label",
  languages: ["en"],
  defaultEnabled: true,
  params: [],
  defaultScope: "both",
  check: (doc) =>
    doc.dialogue
      .filter((span) => span.end - span.start > 2)
      .filter((span) => PERIOD_BEFORE_LOWERCASE_TAG.test(doc.text.slice(span.end - 2, span.end)))
      .filter((span) => doc.text[span.end - 3] !== "." && LOWERCASE_START.test(doc.text.slice(span.end)))
      .map((span) => ({ ruleId: "dialogue-punctuation", start: span.end - 2, length: 1, messageKey: "hints.dialoguePunctuation" })),
};
