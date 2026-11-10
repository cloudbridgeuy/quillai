import { dashTags } from "./dash_tags";
import type { Rule } from "./engine";

const SENTENCE_END = /[.!?]/u;
const TEXT_AFTER = /^\s*[^\s—]/u;

export const dashTagUnclosed: Rule = {
  id: "dash-tag-unclosed",
  labelKey: "rules.dashTagUnclosed.label",
  languages: ["es"],
  defaultEnabled: false,
  params: [],
  defaultScope: "both",
  check: (doc) =>
    dashTags(doc).flatMap((tag) => {
      const inner = doc.text.slice(tag.open + 1, tag.end);
      const end = SENTENCE_END.exec(inner)?.index;
      const followed = end !== undefined && TEXT_AFTER.test(doc.text.slice(tag.open + 2 + end));
      return followed ? [{ ruleId: "dash-tag-unclosed", start: tag.open + 1 + (end ?? 0), length: 1, messageKey: "hints.dashTagUnclosed" as const }] : [];
    }),
};
