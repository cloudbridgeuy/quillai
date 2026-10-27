import type { ScriptLayout } from "./script_layout";

export type ScriptKey = { key: string; shiftKey: boolean; altKey: boolean; ctrlKey: boolean; metaKey: boolean };
export type ScriptRange = { index: number; length: number };
export type ScriptKeyAction = { kind: "pass" } | { kind: "block" } | { kind: "move"; index: number };

const pass: ScriptKeyAction = { kind: "pass" };
const block: ScriptKeyAction = { kind: "block" };

function breaks(layout: ScriptLayout): number[] {
  return layout.map((span) => span.start - 1);
}

function editsText({ key, ctrlKey, metaKey }: ScriptKey): boolean {
  if (key === "Backspace" || key === "Delete" || key === "Enter") return true;
  if (ctrlKey || metaKey) return key.toLowerCase() === "x";
  return key.length === 1;
}

export function crossesBreak(range: ScriptRange, layout: ScriptLayout): boolean {
  return range.length > 0 && breaks(layout).some((index) => index >= range.index && index < range.index + range.length);
}

export function scriptKeyAction(event: ScriptKey, range: ScriptRange, layout: ScriptLayout): ScriptKeyAction {
  if (range.length > 0) return crossesBreak(range, layout) && editsText(event) ? block : pass;
  const lines = breaks(layout);
  const before = lines.includes(range.index - 1);
  const after = lines.includes(range.index + 1);
  if (event.key === "Backspace" && before) return block;
  if (event.key === "Delete" && after) return block;
  if (event.shiftKey || event.metaKey) return pass;
  if (event.key === "ArrowLeft" && before) return { kind: "move", index: range.index === 1 ? 1 : range.index - 2 };
  if (event.key === "ArrowRight" && after) return { kind: "move", index: range.index + 2 };
  return pass;
}

export function offBreak(index: number, previous: number | null, layout: ScriptLayout): number | null {
  if (!breaks(layout).includes(index)) return null;
  return index === 0 || (previous !== null && previous < index) ? index + 1 : index - 1;
}

export function startsList(textBeforeCaret: string): boolean {
  return /^\s*?(\d+\.|-|\*|\[ ?\]|\[x\])$/.test(textBeforeCaret);
}
