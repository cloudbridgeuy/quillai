import type Delta from "quill-delta";
import { scriptLines, type BlockType, type ScriptLine } from "./script_blocks";

type Element = { text: string; joinsPrevious: boolean };

const HEADING = /^(INT|EXT|EST|INT\.?\/EXT|I\/E)[. ]/i;
const SPECIAL_START = /^[.!@#>~=[]/;
const TRANSITION = /\bTO:$/;
const DIALOGUE_FLOW: ReadonlySet<BlockType> = new Set(["character", "parenthetical", "dialogue"]);

function isUppercaseLine(text: string): boolean {
  return /[A-Z]/.test(text) && text === text.toUpperCase();
}

function render({ type, text }: ScriptLine): string {
  const upper = text.toUpperCase();
  switch (type) {
    case "scene-heading":
      return HEADING.test(upper) ? upper : `.${upper}`;
    case "character":
      return /[A-Z]/.test(upper) && !SPECIAL_START.test(upper) ? upper : `@${upper}`;
    case "parenthetical":
      return text.startsWith("(") && text.endsWith(")") ? text : `(${text})`;
    case "transition":
      return TRANSITION.test(upper) ? upper : `>${upper}`;
    case "dialogue":
      return text;
    case "action":
      return HEADING.test(text) || SPECIAL_START.test(text) || isUppercaseLine(text) ? `!${text}` : text;
  }
}

export function toFountain(scenes: readonly Delta[]): string {
  const elements: Element[] = [];
  let previous: BlockType | null = null;
  for (const line of scenes.flatMap(scriptLines)) {
    if (line.text === "") continue;
    const joinsPrevious = previous !== null && DIALOGUE_FLOW.has(previous) && (line.type === "dialogue" || line.type === "parenthetical");
    elements.push({ text: render(line), joinsPrevious });
    previous = line.type;
  }
  const body = elements.reduce((out, { text, joinsPrevious }, position) => out + (position === 0 ? "" : joinsPrevious ? "\n" : "\n\n") + text, "");
  return body === "" ? "" : `${body}\n`;
}
