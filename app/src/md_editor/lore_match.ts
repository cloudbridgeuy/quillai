export type LoreMatchEntry = {
  id: number;
  kind: "character" | "place" | "term" | "note";
  name: string;
  aliases: readonly string[];
  keywords: readonly string[];
  visible_text: string;
  pinned: boolean;
  level: "include" | "known_dont_reveal" | "leave_out";
};

export type LoreMatch = Pick<LoreMatchEntry, "id" | "kind" | "name" | "visible_text" | "level"> & {
  reason: "keyword" | "pinned";
};

export function matchLoreEntries(
  windowText: string,
  entries: readonly LoreMatchEntry[],
  maxCharacters: number,
): LoreMatch[] {
  if (!Number.isSafeInteger(maxCharacters) || maxCharacters < 0) {
    throw new RangeError("maxCharacters must be a nonnegative safe integer");
  }

  const text = windowText.normalize("NFC");
  const matches: LoreMatch[] = [];
  let remaining = maxCharacters;
  const ordered = [...entries.filter(entry => entry.pinned), ...entries.filter(entry => !entry.pinned)];

  for (const entry of ordered) {
    if (!entry.pinned && ![entry.name, ...entry.aliases, ...entry.keywords].some(term => matchesTerm(text, term))) {
      continue;
    }

    const cost = entry.name.length + entry.visible_text.length;
    if (!entry.pinned && cost > remaining) {
      continue;
    }

    remaining -= cost;
    matches.push({
      id: entry.id,
      kind: entry.kind,
      name: entry.name,
      visible_text: entry.visible_text,
      level: entry.level,
      reason: entry.pinned ? "pinned" : "keyword",
    });
  }

  return matches;
}

function matchesTerm(text: string, term: string): boolean {
  const normalized = term.normalize("NFC").trim();
  if (!normalized) {
    return false;
  }

  const escaped = normalized.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const wordCharacter = "[\\p{L}\\p{M}\\p{N}\\p{Pc}\\p{Emoji_Modifier}\\u200d]";
  return new RegExp(`(?<!${wordCharacter})${escaped}(?!${wordCharacter})`, "iu").test(text);
}
