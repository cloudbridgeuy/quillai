// Pure helpers for the timeline manager and the chapter point picker. No I/O.
import type { SelectOption } from "../ui/Select";

export type NamedPoint = { id: number; name: string; position: number };
export type PointMove = { id: number; position: number };

export function orderPoints<T extends { id: number; position: number }>(points: readonly T[]): T[] {
  return [...points].sort((left, right) => left.position - right.position || left.id - right.id);
}

export function nextPosition(points: readonly NamedPoint[]): number {
  return points.reduce((next, point) => Math.max(next, point.position + 1), 0);
}

/** Position updates that swap `id` with its neighbour; positions are renumbered 0..n-1 so ties resolve. */
export function movePoint(points: readonly NamedPoint[], id: number, direction: "up" | "down"): PointMove[] {
  const ordered = orderPoints(points);
  const from = ordered.findIndex(point => point.id === id);
  const to = direction === "up" ? from - 1 : from + 1;
  if (from < 0 || to < 0 || to >= ordered.length) return [];
  [ordered[from], ordered[to]] = [ordered[to]!, ordered[from]!];
  return ordered.flatMap((point, index) => point.position === index ? [] : [{ id: point.id, position: index }]);
}

/** Picker values are point ids as strings; "" means no point (base text). */
export function pickerOptions(points: readonly NamedPoint[], baseLabel: string): SelectOption[] {
  return [{ value: "", label: baseLabel }, ...orderPoints(points).map(point => ({ value: String(point.id), label: point.name }))];
}
