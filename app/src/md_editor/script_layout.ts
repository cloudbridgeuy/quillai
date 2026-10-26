import Delta, { Op } from "quill-delta";

export const SCENE_BREAK = "scene-break";

export type SceneId = number;
export type Scene = { id: SceneId; content: Delta };
export type SceneSpan = { sceneId: SceneId; start: number; length: number };
export type ScriptLayout = readonly SceneSpan[];
export type SplitChange =
  | { kind: "scenes"; changes: Map<SceneId, Delta>; layout: ScriptLayout }
  | { kind: "crossesBreak" };
export type SurfacePosition =
  | { kind: "scene"; sceneId: SceneId; index: number }
  | { kind: "break"; sceneId: SceneId };

const crossesBreak: SplitChange = { kind: "crossesBreak" };

function hasSceneBreak(delta: Delta): boolean {
  return delta.ops.some((op) => op.attributes?.[SCENE_BREAK] !== undefined);
}

function endsWithNewline(delta: Delta): boolean {
  const last = delta.ops[delta.ops.length - 1]?.insert;
  return typeof last === "string" && last.endsWith("\n");
}

function resize(layout: ScriptLayout, growth: (span: SceneSpan) => number): ScriptLayout {
  let start = 1;
  return layout.map((span) => {
    const next = { sceneId: span.sceneId, start, length: span.length + growth(span) };
    start += next.length + 1;
    return next;
  });
}

export function composeScenes(scenes: readonly Scene[]): { document: Delta; layout: ScriptLayout } {
  let document = new Delta();
  const layout: SceneSpan[] = [];
  for (const scene of scenes) {
    if (hasSceneBreak(scene.content)) throw new Error(`Scene ${scene.id} content carries a scene break.`);
    const content = endsWithNewline(scene.content) ? scene.content : scene.content.concat(new Delta().insert("\n"));
    document = document.insert("\n", { [SCENE_BREAK]: scene.id });
    layout.push({ sceneId: scene.id, start: document.length(), length: content.length() });
    document = document.concat(content);
  }
  return { document, layout };
}

function sceneAt(index: number, layout: ScriptLayout): number {
  return layout.findIndex((span) => index >= span.start && index < span.start + span.length);
}

export function splitChange(change: Delta, layout: ScriptLayout): SplitChange {
  const builders = layout.map(() => ({ delta: new Delta(), consumed: 0 }));
  const alignTo = (scene: number, index: number) => {
    const builder = builders[scene]!;
    const offset = index - layout[scene]!.start;
    builder.delta.retain(offset - builder.consumed);
    builder.consumed = offset;
    return builder;
  };
  let index = 0;
  for (const op of change.ops) {
    if (op.attributes?.[SCENE_BREAK] !== undefined) return crossesBreak;
    if (op.insert !== undefined) {
      const scene = sceneAt(index, layout);
      if (scene < 0) return crossesBreak;
      alignTo(scene, index).delta.insert(op.insert, op.attributes);
      continue;
    }
    const end = index + Op.length(op);
    while (index < end) {
      const scene = sceneAt(index, layout);
      if (scene < 0) {
        if (!layout.some((span) => span.start - 1 === index)) throw new RangeError("Change does not fit the script surface.");
        if (op.delete !== undefined || op.attributes !== undefined) return crossesBreak;
        index += 1;
        continue;
      }
      const span = layout[scene]!;
      const length = Math.min(end, span.start + span.length) - index;
      if (op.delete !== undefined) {
        if (index + length === span.start + span.length) return crossesBreak;
        const builder = alignTo(scene, index);
        builder.delta.delete(length);
        builder.consumed += length;
      } else if (op.attributes !== undefined) {
        const builder = alignTo(scene, index);
        builder.delta.retain(typeof op.retain === "object" ? op.retain : length, op.attributes);
        builder.consumed += length;
      }
      index += length;
    }
  }
  const changes = new Map<SceneId, Delta>();
  builders.forEach(({ delta }, scene) => {
    const chopped = delta.chop();
    if (chopped.ops.length > 0) changes.set(layout[scene]!.sceneId, chopped);
  });
  return {
    kind: "scenes",
    changes,
    layout: resize(layout, (span) => changes.get(span.sceneId)?.changeLength() ?? 0),
  };
}

export function placeRemote(sceneId: SceneId, sceneChange: Delta, layout: ScriptLayout): { change: Delta; layout: ScriptLayout } {
  const span = layout.find((candidate) => candidate.sceneId === sceneId);
  if (span === undefined) throw new RangeError(`Scene ${sceneId} is not on the script surface.`);
  if (hasSceneBreak(sceneChange)) throw new Error(`Remote change for scene ${sceneId} carries a scene break.`);
  return {
    change: new Delta().retain(span.start).concat(sceneChange),
    layout: resize(layout, (candidate) => candidate.sceneId === sceneId ? sceneChange.changeLength() : 0),
  };
}

export function toScenePosition(index: number, layout: ScriptLayout): SurfacePosition | null {
  const breakSpan = layout.find((span) => span.start - 1 === index);
  if (breakSpan !== undefined) return { kind: "break", sceneId: breakSpan.sceneId };
  const span = layout[sceneAt(index, layout)];
  return span === undefined ? null : { kind: "scene", sceneId: span.sceneId, index: index - span.start };
}

export function fromScenePosition(sceneId: SceneId, index: number, layout: ScriptLayout): number | null {
  const span = layout.find((candidate) => candidate.sceneId === sceneId);
  return span === undefined || !Number.isInteger(index) || index < 0 || index >= span.length ? null : span.start + index;
}
