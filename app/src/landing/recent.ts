import type { EditorScope } from "../manuscripts/contract";
import { LAST_EDITED_KEY, parseLastEdited, type LastEdited } from "./routes";

export const RECENT_KEY = "quillai.landing.recent.v1";

function sameScope(a: EditorScope, b: EditorScope): boolean {
  return "pieceId" in a && "pieceId" in b ? a.universeId === b.universeId && a.pieceId === b.pieceId
    : "chapterId" in a && "chapterId" in b && a.seriesId === b.seriesId && a.bookId === b.bookId && a.chapterId === b.chapterId;
}

export function parseRecent(raw: string | null, legacy: string | null): LastEdited[] {
  if (raw === null) {
    const item = parseLastEdited(legacy);
    return item ? [item] : [];
  }
  try {
    const value: unknown = JSON.parse(raw);
    if (!Array.isArray(value)) return [];
    return value.map(item => parseLastEdited(JSON.stringify(item))).filter((item): item is LastEdited => item !== null).slice(0, 5);
  } catch { return []; }
}

export function forgetRecent(items: LastEdited[], scope: EditorScope): LastEdited[] {
  return items.filter(item => !sameScope(item.scope, scope));
}

export function pushRecent(items: LastEdited[], item: LastEdited): LastEdited[] {
  return [item, ...forgetRecent(items, item.scope)].slice(0, 5);
}

export function readRecent(): LastEdited[] {
  try {
    const raw = window.localStorage.getItem(RECENT_KEY);
    const items = parseRecent(raw, raw === null ? window.localStorage.getItem(LAST_EDITED_KEY) : null);
    if (raw === null) saveRecent(items);
    return items;
  } catch { return []; }
}

export function saveRecent(items: LastEdited[]): void {
  try { window.localStorage.setItem(RECENT_KEY, JSON.stringify(items)); } catch {}
}

export function rememberEdited(raw: string): void {
  const item = parseLastEdited(raw);
  if (item) saveRecent(pushRecent(readRecent(), item));
}
