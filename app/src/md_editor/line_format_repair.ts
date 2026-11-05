import type { Op } from "quill-delta";

export type ScriptBlockRepair = { index: number; value: string | null };

function scriptBlocks(ops: Op[]): Map<number, string> {
  const blocks = new Map<number, string>();
  let index = 0;
  for (const op of ops) {
    if (typeof op.insert !== "string") {
      index += 1;
      continue;
    }
    const value = op.attributes?.["script-block"];
    for (const char of op.insert) {
      if (char === "\n" && typeof value === "string") blocks.set(index, value);
      index += char.length;
    }
  }
  return blocks;
}

/** Newlines whose `script-block` differs between the expected and actual documents. */
export function scriptBlockRepairs(expected: Op[], actual: Op[]): ScriptBlockRepair[] {
  const want = scriptBlocks(expected);
  const have = scriptBlocks(actual);
  const repairs: ScriptBlockRepair[] = [];
  for (const index of [...new Set([...want.keys(), ...have.keys()])].sort((a, b) => a - b)) {
    const value = want.get(index) ?? null;
    if (value !== (have.get(index) ?? null)) repairs.push({ index, value });
  }
  return repairs;
}
