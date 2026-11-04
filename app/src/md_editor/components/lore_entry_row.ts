import type { LoreEntry } from "../../manuscripts/lore_client";

export function loreEntryLabel(entry: Pick<LoreEntry, "name" | "kind" | "pinned">): string {
  return `${entry.name} · ${entry.kind}${entry.pinned ? " · pinned" : ""}`;
}

export function loreEntryPreview(text: string): string {
  const line = text.split("\n").map(line => line.trim()).find(Boolean) ?? "";
  const characters = Array.from(line);
  return characters.length > 60 ? `${characters.slice(0, 60).join("")}…` : line;
}

export function loreLevel(level: string) {
  return level === "known_dont_reveal" ? "Known, don't reveal" : level === "leave_out" ? "Leave out" : "Include";
}
