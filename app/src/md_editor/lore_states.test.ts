import { describe, expect, test } from "bun:test";
import { activeState, effectivePoint, resolveLoreEntry, type LoreStateRow, type PieceRef, type TimelinePoint } from "./lore_states";

const points = new Map<number, TimelinePoint>([
  [1, { id: 1, position: 10 }], [2, { id: 2, position: 20 }], [3, { id: 3, position: 20 }], [4, { id: 4, position: 30 }],
]);
const state = (id: number, pointId: number, visibleText = `v${id}`): LoreStateRow => ({ id, name: `s${id}`, pointId, visibleText, hiddenNotes: `h${id}` });

describe("effectivePoint", () => {
  test("uses own point, else parent's, else null", () => {
    expect(effectivePoint({ timelinePointId: 2, parent: { timelinePointId: 1, parent: null } }, points)?.id).toBe(2);
    expect(effectivePoint({ timelinePointId: null, parent: { timelinePointId: 1, parent: null } }, points)?.id).toBe(1);
    expect(effectivePoint({ timelinePointId: null, parent: null }, points)).toBeNull();
  });
  test("deleted or unknown own point falls back to parent then none", () => {
    const parent: PieceRef = { timelinePointId: 1, parent: null };
    expect(effectivePoint({ timelinePointId: 99, parent }, points)?.id).toBe(1);
    expect(effectivePoint({ timelinePointId: 99, parent: { timelinePointId: 98, parent: null } }, points)).toBeNull();
  });
});

describe("activeState", () => {
  test("no point means base", () => {
    expect(activeState(null, [state(1, 1)], points)).toBeNull();
  });
  test("latest state at or before the point wins; later states are ignored", () => {
    const states = [state(1, 1), state(2, 2), state(3, 4)];
    expect(activeState(points.get(2)!, states, points)?.id).toBe(2);
    expect(activeState(points.get(1)!, states, points)?.id).toBe(1);
    expect(activeState(points.get(4)!, states, points)?.id).toBe(3);
  });
  test("state only after the point yields base", () => {
    expect(activeState(points.get(1)!, [state(1, 4)], points)).toBeNull();
  });
  test("ties on position break by point id then state id", () => {
    expect(activeState(points.get(4)!, [state(5, 2), state(6, 3)], points)?.id).toBe(6);
    expect(activeState(points.get(4)!, [state(7, 2), state(6, 2)], points)?.id).toBe(7);
    expect(activeState(points.get(4)!, [state(6, 3), state(5, 2)], points)?.id).toBe(6);
  });
  test("states on deleted or unknown points are skipped", () => {
    expect(activeState(points.get(4)!, [state(1, 1), state(2, 99)], points)?.id).toBe(1);
  });
});

describe("resolveLoreEntry", () => {
  const entry = { id: 1, visible_text: "base", hidden_notes: "secret", name: "Ann" };
  test("overlays text and notes and names the state", () => {
    expect(resolveLoreEntry(entry, [state(1, 1, "young")], points.get(2)!, points)).toEqual({ ...entry, visible_text: "young", hidden_notes: "h1", stateName: "s1" });
  });
  test("base entry carries a null state name", () => {
    expect(resolveLoreEntry(entry, [state(1, 4)], null, points)).toEqual({ ...entry, stateName: null });
  });
});
