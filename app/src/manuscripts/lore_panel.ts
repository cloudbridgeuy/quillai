import type { LoreStateRow } from "../md_editor/lore_states";
import type { LoreEntry, LoreInput } from "./lore_client";
import type { NamedPoint } from "./timeline_panel";

export function newLoreDraft(): LoreInput {
  return { kind: "character", name: "", keywords: [], aliases: [], visible_text: "", hidden_notes: "", level: "include", pinned: false };
}

export function parseLoreTerms(text: string): string[] {
  return text.split("\n").map(term => term.trim()).filter(Boolean);
}

export function replaceLoreEntry(entries: readonly LoreEntry[], saved: LoreEntry): LoreEntry[] {
  return [...entries.filter(entry => entry.id !== saved.id), saved].sort((a, b) => a.name.localeCompare(b.name) || a.id - b.id);
}

export function setEntryStates(entries: readonly LoreEntry[], loreId: number, points: readonly NamedPoint[],
  update: (states: readonly LoreStateRow[]) => readonly LoreStateRow[]): LoreEntry[] {
  const position = new Map(points.map(point => [point.id, point.position]));
  const at = (state: LoreStateRow) => position.get(state.pointId) ?? Number.MAX_SAFE_INTEGER;
  return entries.map(entry => entry.id !== loreId ? entry
    : { ...entry, states: [...update(entry.states ?? [])].sort((a, b) => at(a) - at(b) || a.pointId - b.pointId || a.id - b.id) });
}
