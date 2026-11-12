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

function storedId(params: URLSearchParams, name: string): number | null {
  const values = params.getAll(name);
  if (values.length !== 1 || !/^[0-9]+$/.test(values[0] ?? "")) return null;
  const value = Number(values[0]);
  return Number.isSafeInteger(value) && value > 0 ? value : null;
}

function matchesGone(item: LastEdited, params: URLSearchParams): boolean {
  const scope = item.scope;
  const pieceId = storedId(params, "pieceId");
  if (pieceId !== null && "pieceId" in scope && scope.pieceId === pieceId) {
    const universeId = storedId(params, "universeId");
    return universeId === null || scope.universeId === universeId;
  }
  const seriesId = storedId(params, "seriesId");
  const bookId = storedId(params, "bookId");
  const chapterId = storedId(params, "chapterId");
  return seriesId !== null && bookId !== null && chapterId !== null && "chapterId" in scope
    && scope.seriesId === seriesId && scope.bookId === bookId && scope.chapterId === chapterId;
}

export function forgetStoredIds(search: string): void {
  const params = new URLSearchParams(search);
  try {
    saveRecent(readRecent().filter(item => !matchesGone(item, params)));
    const legacy = parseLastEdited(window.localStorage.getItem(LAST_EDITED_KEY));
    if (legacy && matchesGone(legacy, params)) window.localStorage.removeItem(LAST_EDITED_KEY);
  } catch {}
}

export function rememberEdited(raw: string): void {
  const item = parseLastEdited(raw);
  if (item) saveRecent(pushRecent(readRecent(), item));
}
