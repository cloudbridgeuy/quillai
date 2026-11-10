import type { BlockType, ScriptLine } from "./script_blocks";

export type ScriptExchangeState = { pending: string; previous: BlockType | null };
export type ScriptExchangeFeed = { lines: ScriptLine[]; state: ScriptExchangeState };

export const SCRIPT_EXCHANGE_START: ScriptExchangeState = { pending: "", previous: null };

export function feedScriptExchange(state: ScriptExchangeState, chunk: string): ScriptExchangeFeed {
  const parts = (state.pending + chunk).split("\n");
  const pending = parts.pop() ?? "";
  const lines: ScriptLine[] = [];
  let previous = state.previous;
  for (const part of parts) {
    const line = classifyLine(part, previous);
    if (line) {
      lines.push(line);
      previous = line.type;
    }
  }
  return { lines, state: { pending, previous } };
}

export function finishScriptExchange(state: ScriptExchangeState): ScriptLine[] {
  const line = classifyLine(state.pending, state.previous);
  return line ? [line] : [];
}

function classifyLine(raw: string, previous: BlockType | null): ScriptLine | null {
  const text = raw.trim();
  if (!text) return null;
  if (text.startsWith("(") && text.endsWith(")")) return { type: "parenthetical", text };
  if (previous === "character" || previous === "parenthetical") return { type: "dialogue", text };
  const isCue = text.toUpperCase() === text && text.toLowerCase() !== text;
  return { type: isCue ? "character" : "dialogue", text };
}
