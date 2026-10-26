import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { composeScenes, fromScenePosition, placeRemote, SCENE_BREAK, splitChange, toScenePosition, type ScriptLayout } from "./script_layout";

const stepCount = 400;
const replaySeed = process.env.QUILLAI_LAYOUT_SEED;
const seeds = replaySeed === undefined
  ? [0, 1, 2, 3, 7, 42, 729, 982, 20261026, 0x12345678, 0x80000000, 0xffffffff]
  : [Number(replaySeed)];

if (seeds.some((seed) => !Number.isSafeInteger(seed) || seed < 0 || seed > 0xffffffff) || replaySeed?.trim() === "") {
  throw new Error("QUILLAI_LAYOUT_SEED must be an unsigned 32-bit integer");
}

const tokens = ["a", "bc", " ", "é", "中", "😀", "🚀", "👩‍💻", "\n", "😀😀"];
const formats = [{ bold: true }, { italic: null }, { header: 1 }, { header: null }];

function seededRandom(seed: number) {
  let state = seed;
  return (limit: number) => {
    state = (state + 0x6d2b79f5) >>> 0;
    let value = Math.imul(state ^ (state >>> 15), state | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return Math.floor(((value ^ (value >>> 14)) >>> 0) / 0x100000000 * limit);
  };
}

function plainText(document: Delta): string {
  return document.ops.map((op) => typeof op.insert === "string" ? op.insert : "").join("");
}

function codepointBoundaries(text: string): number[] {
  const boundaries = [0];
  for (const character of Array.from(text)) boundaries.push(boundaries[boundaries.length - 1]! + character.length);
  return boundaries;
}

function randomRange(text: string, random: (limit: number) => number) {
  const boundaries = codepointBoundaries(text);
  const start = random(boundaries.length);
  const end = Math.min(boundaries.length - 1, start + random(4));
  return { index: boundaries[start]!, length: boundaries[end]! - boundaries[start]! };
}

function randomChange(text: string, random: (limit: number) => number, keepLastNewline: boolean) {
  const { index, length } = randomRange(keepLastNewline ? text.slice(0, -1) : text, random);
  const change = new Delta().retain(index);
  const kind = length === 0 ? 0 : random(4);
  const inserted = kind === 0 || kind === 3;
  if (inserted) change.insert(tokens[random(tokens.length)]!, random(3) === 0 ? formats[random(2)] : undefined);
  if (kind === 1 || kind === 3) change.delete(length);
  if (kind === 2) change.retain(length, formats[random(formats.length)]);
  return { change, index, length: kind === 0 ? 0 : length, inserted, deleted: kind === 1 || kind === 3 };
}

function expectedCrossing(layout: ScriptLayout, total: number, edit: ReturnType<typeof randomChange>): boolean {
  const breaks = layout.map((span) => span.start - 1);
  const lastNewlines = layout.map((span) => span.start + span.length - 1);
  const covers = (positions: number[]) => positions.some((at) => at >= edit.index && at < edit.index + edit.length);
  if (edit.inserted && (breaks.includes(edit.index) || edit.index === total)) return true;
  return covers(breaks) || (edit.deleted && covers(lastNewlines));
}

function expectConsistent(surface: Delta, layout: ScriptLayout, scenes: Map<number, Delta>) {
  const composed = composeScenes([...scenes].map(([id, content]) => ({ id, content })));
  expect(composed.document).toEqual(surface);
  expect(composed.layout).toEqual(layout);
}

for (const seed of seeds) {
  test(`surface and scenes stay one view after ${stepCount} seeded local and remote edits; seed=${seed}`, () => {
    const random = seededRandom(seed);
    const scenes = new Map<number, Delta>();
    const sceneCount = 2 + random(4);
    for (let id = 1; id <= sceneCount; id += 1) {
      let content = new Delta();
      const pieces = random(5);
      for (let piece = 0; piece < pieces; piece += 1) content = content.insert(tokens[random(tokens.length)]!);
      scenes.set(id, content);
    }
    let { document: surface, layout } = composeScenes([...scenes].map(([id, content]) => ({ id, content })));
    for (const span of layout) scenes.set(span.sceneId, surface.slice(span.start, span.start + span.length));
    let crossings = 0;
    let remotes = 0;

    for (let step = 0; step < stepCount; step += 1) {
      if (random(3) > 0) {
        const total = surface.length();
        const edit = randomChange(plainText(surface), random, false);
        const result = splitChange(edit.change, layout);
        expect(result.kind === "crossesBreak").toBe(expectedCrossing(layout, total, edit));
        if (result.kind === "crossesBreak") {
          crossings += 1;
          continue;
        }
        for (const [id, change] of result.changes) {
          expect(change.ops.some((op) => op.attributes?.[SCENE_BREAK] !== undefined)).toBe(false);
          scenes.set(id, scenes.get(id)!.compose(change));
        }
        surface = surface.compose(edit.change);
        layout = result.layout;
      } else {
        const span = layout[random(layout.length)]!;
        const others = layout.filter((candidate) => candidate.sceneId !== span.sceneId);
        const caretSpan = others[random(others.length)]!;
        const caretOffset = random(caretSpan.length);
        const caret = fromScenePosition(caretSpan.sceneId, caretOffset, layout)!;
        const { change } = randomChange(plainText(scenes.get(span.sceneId)!), random, true);
        const placed = placeRemote(span.sceneId, change, layout);
        scenes.set(span.sceneId, scenes.get(span.sceneId)!.compose(change));
        surface = surface.compose(placed.change);
        layout = placed.layout;
        expect(toScenePosition(placed.change.transformPosition(caret, true), layout)).toEqual({ kind: "scene", sceneId: caretSpan.sceneId, index: caretOffset });
        remotes += 1;
      }
      expectConsistent(surface, layout, scenes);
    }
    expect(crossings).toBeGreaterThan(0);
    expect(remotes).toBeGreaterThan(0);
  });
}
