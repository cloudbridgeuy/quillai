import type { Hint } from "./engine";

export type Rect = { top: number; left: number; width: number; height: number };
export type Bounds = (Rect & { bottom: number; right: number }) | null;
export type Visible = { top: number; bottom: number };
export type Measure = (index: number, length: number) => Bounds;

const SAME_LINE_PX = 1;

export function hintSegments({ start, length }: Pick<Hint, "start" | "length">, text: string, measure: Measure, visible: Visible): Rect[] {
  const first = measure(start, 1);
  const last = measure(start + length - 1, 1);
  if (!first || !last || last.bottom < visible.top || first.top > visible.bottom) return [];
  const segments: Rect[] = [];
  for (const word of text.slice(start, start + length).matchAll(/\S+/g)) {
    const bounds = measure(start + word.index, word[0].length);
    if (!bounds) continue;
    const line = segments.at(-1);
    if (line && Math.abs(line.top - bounds.top) < SAME_LINE_PX) line.width = bounds.left + bounds.width - line.left;
    else segments.push({ top: bounds.top, left: bounds.left, width: bounds.width, height: bounds.height });
  }
  return segments;
}
