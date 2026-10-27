import Delta from "quill-delta";

export const SCRIPT_BLOCK = "script-block";

export const BLOCK_TYPES = ["scene-heading", "action", "character", "dialogue", "parenthetical", "transition"] as const;
export type BlockType = (typeof BLOCK_TYPES)[number];
export type ScriptLine = { type: BlockType; text: string };

const AFTER_ENTER: Record<BlockType, BlockType> = {
  "scene-heading": "action",
  action: "action",
  character: "dialogue",
  dialogue: "action",
  parenthetical: "dialogue",
  transition: "scene-heading",
};

export function blockLabel(type: BlockType): string {
  const words = type.replace("-", " ");
  return words[0]!.toUpperCase() + words.slice(1);
}

export function parseBlock(value: unknown): BlockType {
  return BLOCK_TYPES.find((type) => type === value) ?? "action";
}

export function blockAttribute(type: BlockType): BlockType | null {
  return type === "action" ? null : type;
}

export function cycleBlock(type: BlockType, step: 1 | -1): BlockType {
  return BLOCK_TYPES[(BLOCK_TYPES.indexOf(type) + step + BLOCK_TYPES.length) % BLOCK_TYPES.length]!;
}

export function enterBlock(type: BlockType): BlockType {
  return AFTER_ENTER[type];
}

export function splitLineDelta(index: number, toLineEnd: number, type: BlockType): Delta {
  return new Delta()
    .retain(index)
    .insert("\n", type === "action" ? undefined : { [SCRIPT_BLOCK]: type })
    .retain(toLineEnd)
    .retain(1, { [SCRIPT_BLOCK]: blockAttribute(enterBlock(type)) });
}

export function scriptLines(scene: Delta): ScriptLine[] {
  const lines: ScriptLine[] = [];
  let text = "";
  for (const op of scene.ops) {
    if (typeof op.insert !== "string") continue;
    const parts = op.insert.split("\n");
    parts.forEach((part, position) => {
      text += part;
      if (position === parts.length - 1) return;
      lines.push({ type: parseBlock(op.attributes?.[SCRIPT_BLOCK]), text: text.trim() });
      text = "";
    });
  }
  return lines;
}
