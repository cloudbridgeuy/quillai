import type { LoreEntry, LoreInput } from "./lore_client";

export function newLoreDraft(): LoreInput {
  return { kind: "character", name: "", keywords: [], aliases: [], visible_text: "", hidden_notes: "", level: "include", pinned: false };
}

export function parseLoreTerms(text: string): string[] {
  return text.split("\n").map(term => term.trim()).filter(Boolean);
}

export function replaceLoreEntry(entries: readonly LoreEntry[], saved: LoreEntry): LoreEntry[] {
  return [...entries.filter(entry => entry.id !== saved.id), saved].sort((a, b) => a.name.localeCompare(b.name) || a.id - b.id);
}
