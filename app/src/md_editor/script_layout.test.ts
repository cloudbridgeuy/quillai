import { expect, test } from "bun:test";
import Delta from "quill-delta";
import { composeScenes, fromScenePosition, placeRemote, SCENE_BREAK, splitChange, toScenePosition } from "./script_layout";

const scenes = [
  { id: 11, content: new Delta().insert("INT. 😀\n") },
  { id: 12, content: new Delta() },
  { id: 13, content: new Delta().insert("Hi", { bold: true }).insert("\n", { header: 1 }) },
];

function surface() {
  return composeScenes(scenes);
}

function split(change: Delta) {
  const result = splitChange(change, surface().layout);
  if (result.kind !== "scenes") throw new Error("expected scene changes");
  return result;
}

test("compose puts one break line before every scene and gives empty scenes a newline", () => {
  const { document, layout } = surface();
  expect(document.ops).toEqual([
    { insert: "\n", attributes: { [SCENE_BREAK]: 11 } },
    { insert: "INT. 😀\n" },
    { insert: "\n", attributes: { [SCENE_BREAK]: 12 } },
    { insert: "\n" },
    { insert: "\n", attributes: { [SCENE_BREAK]: 13 } },
    { insert: "Hi", attributes: { bold: true } },
    { insert: "\n", attributes: { header: 1 } },
  ]);
  expect(layout).toEqual([
    { sceneId: 11, start: 1, length: 8 },
    { sceneId: 12, start: 10, length: 1 },
    { sceneId: 13, start: 12, length: 3 },
  ]);
  expect(composeScenes([{ id: 1, content: new Delta().insert("no newline") }]).document.ops[1]).toEqual({ insert: "no newline\n" });
});

test("compose rejects scene content that carries a break attribute", () => {
  expect(() => composeScenes([{ id: 1, content: new Delta().insert("\n", { [SCENE_BREAK]: 1 }) }])).toThrow();
});

test("typing in one scene changes only that scene", () => {
  const result = split(new Delta().retain(13).insert("ey"));
  expect([...result.changes]).toEqual([[13, new Delta().retain(1).insert("ey")]]);
  expect(result.layout).toEqual([
    { sceneId: 11, start: 1, length: 8 },
    { sceneId: 12, start: 10, length: 1 },
    { sceneId: 13, start: 12, length: 5 },
  ]);
});

test("edits in two scenes that leave the break alone split per scene", () => {
  const result = split(new Delta().retain(1).delete(4).retain(5).insert("x").retain(2).retain(2, { italic: true }));
  expect([...result.changes]).toEqual([
    [11, new Delta().delete(4)],
    [12, new Delta().insert("x")],
    [13, new Delta().retain(2, { italic: true })],
  ]);
  expect(result.layout.map((span) => span.length)).toEqual([4, 2, 3]);
});

test("line formats on a scene's own last newline pass through", () => {
  expect([...split(new Delta().retain(14).retain(1, { header: null })).changes]).toEqual([[13, new Delta().retain(2).retain(1, { header: null })]]);
});

test("changes that touch a break line report crossing", () => {
  const { layout } = surface();
  const crossing = [
    new Delta().retain(5).delete(6),
    new Delta().retain(9).delete(1),
    new Delta().retain(8).delete(1),
    new Delta().delete(1),
    new Delta().retain(9).retain(1, { bold: true }),
    new Delta().retain(9).insert("x"),
    new Delta().insert("x"),
    new Delta().retain(15).insert("x"),
    new Delta().retain(2).insert("\n", { [SCENE_BREAK]: 99 }),
    new Delta().retain(2).retain(1, { [SCENE_BREAK]: null }),
  ];
  for (const change of crossing) expect(splitChange(change, layout)).toEqual({ kind: "crossesBreak" });
});

test("a change longer than the surface is a programming error", () => {
  expect(() => splitChange(new Delta().retain(15).retain(1, { bold: true }), surface().layout)).toThrow(RangeError);
});

test("a remote change lands in its scene and keeps a caret in another scene on its text", () => {
  const { document, layout } = surface();
  const caret = fromScenePosition(13, 1, layout)!;
  const placed = placeRemote(11, new Delta().insert("🚀 "), layout);
  const next = document.compose(placed.change);
  const moved = placed.change.transformPosition(caret, true);
  expect(toScenePosition(moved, placed.layout)).toEqual({ kind: "scene", sceneId: 13, index: 1 });
  expect(next.slice(placed.layout[2]!.start, placed.layout[2]!.start + placed.layout[2]!.length)).toEqual(scenes[2]!.content);
  expect(composeScenes([{ id: 11, content: new Delta().insert("🚀 INT. 😀\n") }, scenes[1]!, scenes[2]!])).toEqual({ document: next, layout: placed.layout });
});

test("a remote change cannot carry a break or target a missing scene", () => {
  const { layout } = surface();
  expect(() => placeRemote(11, new Delta().insert("\n", { [SCENE_BREAK]: 11 }), layout)).toThrow();
  expect(() => placeRemote(99, new Delta().insert("x"), layout)).toThrow(RangeError);
});

test("surface offsets map to scene offsets and back", () => {
  const { layout } = surface();
  expect(toScenePosition(0, layout)).toEqual({ kind: "break", sceneId: 11 });
  expect(toScenePosition(1, layout)).toEqual({ kind: "scene", sceneId: 11, index: 0 });
  expect(toScenePosition(8, layout)).toEqual({ kind: "scene", sceneId: 11, index: 7 });
  expect(toScenePosition(9, layout)).toEqual({ kind: "break", sceneId: 12 });
  expect(toScenePosition(14, layout)).toEqual({ kind: "scene", sceneId: 13, index: 2 });
  expect(toScenePosition(15, layout)).toBeNull();
  expect(toScenePosition(-1, layout)).toBeNull();
  for (let index = 0; index < 15; index += 1) {
    const position = toScenePosition(index, layout)!;
    if (position.kind === "scene") expect(fromScenePosition(position.sceneId, position.index, layout)).toBe(index);
  }
  expect(fromScenePosition(12, 1, layout)).toBeNull();
  expect(fromScenePosition(99, 0, layout)).toBeNull();
});
