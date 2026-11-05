import type { LoreEntry } from "../../manuscripts/lore_client";
import { defaultLocale, formatMessage } from "../../i18n/messages";

export function loreEntryLabel(entry: Pick<LoreEntry, "name" | "kind" | "pinned">): string {
  return formatMessage(defaultLocale, "lore.entryLabel", { name: entry.name, kind: entry.kind, pinned: entry.pinned ? formatMessage(defaultLocale, "lore.pinned") : "" });
}

export function loreEntryPreview(text: string): string {
  const line = text.split("\n").map(line => line.trim()).find(Boolean) ?? "";
  const characters = Array.from(line);
  return characters.length > 60 ? `${characters.slice(0, 60).join("")}…` : line;
}

export function loreLevel(level: string) {
  return formatMessage(defaultLocale, level === "known_dont_reveal" ? "lore.known" : level === "leave_out" ? "lore.leaveOut" : "lore.include");
}
