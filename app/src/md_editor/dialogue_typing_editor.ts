import Quill from "quill";
import type { Delta as DeltaT, EmitterSource } from "quill";
import { Delta } from "quill/core";
import { dialogueReplacement } from "./dialogue_typing";

type TypedCharacter = { index: number; character: string; attributes: DeltaT["ops"][number]["attributes"] };

function typedCharacter(delta: DeltaT): TypedCharacter | null {
  let index = 0;
  let typed: TypedCharacter | null = null;
  for (const op of delta.ops) {
    if (typeof op.retain === "number" && typed === null) index += op.retain;
    else if (op.insert !== undefined) {
      if (typed !== null || typeof op.insert !== "string" || op.insert.length !== 1) return null;
      typed = { index, character: op.insert, attributes: op.attributes };
    } else if (op.delete === undefined && typed === null) return null;
  }
  return typed;
}

export function dialogueTyping(quill: Quill, dashesBlocked: () => boolean): (delta: DeltaT, source: EmitterSource) => void {
  let ruleQuoteIndex: number | null = null;
  let committing = false;
  quill.on(Quill.events.COMPOSITION_BEFORE_END, () => { committing = true; });
  quill.on(Quill.events.COMPOSITION_END, () => { committing = false; });
  return (delta, source) => {
    const previousRuleQuote = ruleQuoteIndex;
    ruleQuoteIndex = null;
    if (source !== Quill.sources.USER || quill.history.ignoreChange) return;
    if (quill.composition.isComposing && !committing) return;
    const typed = typedCharacter(delta);
    if (typed === null || (typed.character === "-" && dashesBlocked())) return;
    const [, offset] = quill.getLine(typed.index);
    const result = dialogueReplacement(quill.getText(typed.index - offset, offset), typed.character, previousRuleQuote === typed.index - 1);
    if (result === null) return;
    const start = typed.index + 1 - result.remove;
    quill.history.cutoff();
    quill.updateContents(new Delta().retain(start).delete(result.remove).insert(result.insert, typed.attributes), Quill.sources.USER);
    quill.history.cutoff();
    if (result.insert === "‘") ruleQuoteIndex = start;
  };
}
