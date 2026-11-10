import { dashTags } from "./dash_tags";
import type { Rule } from "./engine";

export const dashTagPunctuation: Rule = {
  id: "dash-tag-punctuation",
  labelKey: "rules.dashTagPunctuation.label",
  languages: ["es"],
  defaultEnabled: true,
  params: [],
  defaultScope: "both",
  check: (doc) =>
    dashTags(doc)
      .map((tag) => doc.text.slice(0, tag.open).trimEnd().length - 1)
      .filter((start) => start >= 0 && /[.,]/.test(doc.text[start] ?? ""))
      .map((start) => ({ ruleId: "dash-tag-punctuation", start, length: 1, messageKey: "hints.dashTagPunctuation" })),
};
