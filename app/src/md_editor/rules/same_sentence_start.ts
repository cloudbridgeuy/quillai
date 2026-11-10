import type { Rule } from "./engine";

export const sameSentenceStart: Rule = {
  id: "same-sentence-start",
  labelKey: "rules.sameStart.label",
  languages: ["en"],
  defaultEnabled: true,
  params: [{ key: "count", kind: "int", labelKey: "rules.sameStart.count", default: 3, min: 2, max: 10 }],
  defaultScope: "narration",
  check: (doc, config) => {
    const count = Number(config.params.count);
    let runKey: string | undefined;
    let run = 0;
    return doc.sentences.flatMap((sentence) => {
      const first = doc.words.find((word) => word.start >= sentence.start && word.start < sentence.end);
      const key = first?.lower;
      run = key !== undefined && key === runKey ? run + 1 : 1;
      runKey = key;
      if (!first || run < count) return [];
      return [{ ruleId: "same-sentence-start", start: first.start, length: first.end - first.start, messageKey: "hints.sameStart" as const, messageArgs: { count: run, word: first.text } }];
    });
  },
};
