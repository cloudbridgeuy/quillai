// Pure lore-state resolution (docs/lore-states.md section 3). No I/O.
export type TimelinePoint = { id: number; position: number };
export type PieceRef = { timelinePointId: number | null; parent: PieceRef | null };
export type LoreStateRow = { id: number; name: string; pointId: number; visibleText: string; hiddenNotes: string };
export type ResolvedLore<T> = T & { stateName: string | null };

/** Own point if known, else the parent's, else null. Deleted points are absent from `points`. */
export function effectivePoint(piece: PieceRef | null, points: ReadonlyMap<number, TimelinePoint>): TimelinePoint | null {
  for (let current = piece; current; current = current.parent) {
    const point = current.timelinePointId === null ? undefined : points.get(current.timelinePointId);
    if (point) return point;
  }
  return null;
}

/** Latest state whose point is at or before `point`; ties by point id then state id. */
export function activeState(point: TimelinePoint | null, states: readonly LoreStateRow[], points: ReadonlyMap<number, TimelinePoint>): LoreStateRow | null {
  if (!point) return null;
  let best: { state: LoreStateRow; point: TimelinePoint } | null = null;
  for (const state of states) {
    const at = points.get(state.pointId);
    if (!at || at.position > point.position) continue;
    if (!best || at.position > best.point.position || (at.position === best.point.position
      && (at.id > best.point.id || (at.id === best.point.id && state.id > best.state.id)))) best = { state, point: at };
  }
  return best?.state ?? null;
}

export function resolveLoreEntry<T extends { visible_text: string; hidden_notes: string }>(
  entry: T, states: readonly LoreStateRow[], point: TimelinePoint | null, points: ReadonlyMap<number, TimelinePoint>,
): ResolvedLore<T> {
  const state = activeState(point, states, points);
  return state
    ? { ...entry, visible_text: state.visibleText, hidden_notes: state.hiddenNotes, stateName: state.name }
    : { ...entry, stateName: null };
}
