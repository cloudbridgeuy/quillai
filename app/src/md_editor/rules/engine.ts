import { dialoguePunctuation } from "./dialogue_punctuation";
import { redundantPhrases } from "./redundant_phrases";
import { repeatedWord } from "./repeated_word";
import { sameSentenceStart } from "./same_sentence_start";
import { analyzeParagraph, splitParagraphs, type ParagraphAnalysis } from "./shared_pass";
import type { FormatValues, MessageKey } from "../../i18n/messages";

export type Scope = "narration" | "dialogue" | "both";
export type Language = "en" | "es";
export type ParamValue = number | string[];

export type ParamDef =
  | { key: string; kind: "int"; labelKey: MessageKey; default: number; min: number; max: number }
  | { key: string; kind: "words"; labelKey: MessageKey; default: string[] };

export type Hint = { ruleId: string; start: number; length: number; messageKey: MessageKey; messageArgs?: FormatValues };

export type RuleConfig = { enabled: boolean; scope: Scope; params: Record<string, ParamValue> };

export type Rule = {
  id: string;
  labelKey: MessageKey;
  languages: Language[];
  defaultEnabled: boolean;
  params: ParamDef[];
  defaultScope: Scope;
  check(doc: ParagraphAnalysis, config: RuleConfig): Hint[];
};

export type StoredRuleConfig = { enabled?: boolean; scope?: Scope; params?: Record<string, ParamValue> };
export type StoredRules = Record<string, StoredRuleConfig>;

export const RULES: readonly Rule[] = [dialoguePunctuation, redundantPhrases, repeatedWord, sameSentenceStart];

export function resolveConfig(rule: Rule, stored: StoredRuleConfig | undefined): RuleConfig {
  const params = Object.fromEntries(rule.params.map((def) => [def.key, stored?.params?.[def.key] ?? def.default]));
  return { enabled: stored?.enabled ?? rule.defaultEnabled, scope: stored?.scope ?? rule.defaultScope, params };
}

export function createEngine(rules: readonly Rule[] = RULES, capacity = 2000) {
  const analyses = lru<ParagraphAnalysis>(capacity);
  const hints = lru<Hint[]>(capacity);

  return {
    run(text: string, language: Language, stored: StoredRules): Hint[] {
      const active = rules
        .filter((rule) => rule.languages.includes(language))
        .map((rule) => ({ rule, config: resolveConfig(rule, stored[rule.id]) }))
        .filter(({ config }) => config.enabled);

      return splitParagraphs(text).flatMap((paragraph) => {
        const doc = analyses(paragraph.text, () => analyzeParagraph(paragraph.text));
        return active.flatMap(({ rule, config }) => {
          const key = JSON.stringify([rule.id, language, config, paragraph.text]);
          return hints(key, () => rule.check(doc, config).filter((hint) => inScope(doc, hint.start, config.scope)));
        }).map((hint) => ({ ...hint, start: hint.start + paragraph.start }));
      });
    },
  };
}

function inScope(doc: ParagraphAnalysis, offset: number, scope: Scope): boolean {
  if (scope === "both") return true;
  const dialogue = doc.dialogue.some((span) => offset >= span.start && offset < span.end);
  return scope === (dialogue ? "dialogue" : "narration");
}

function lru<V>(capacity: number) {
  const entries = new Map<string, V>();
  return (key: string, make: () => V): V => {
    const value = entries.get(key) ?? make();
    entries.delete(key);
    entries.set(key, value);
    const oldest = entries.keys().next();
    if (entries.size > capacity && !oldest.done) entries.delete(oldest.value);
    return value;
  };
}
