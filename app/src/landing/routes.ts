import type { EditorScope } from "../manuscripts/contract";

export const LAST_EDITED_KEY = "quillai.landing.lastEdited.v1";
const editorKeys = ["seriesId", "bookId", "chapterId", "universeId", "pieceId", "editor"];

export type LastEdited = { scope: EditorScope; title: string };

export function showsLanding(search: string): boolean {
  const params = new URLSearchParams(search);
  return !editorKeys.some(key => params.has(key));
}

export function landingUrl(href: string): URL {
  const url = new URL(href);
  for (const key of editorKeys) url.searchParams.delete(key);
  return url;
}

export function editorUrl(href: string, scope: EditorScope): URL {
  const url = landingUrl(href);
  for (const [key, value] of Object.entries(scope)) url.searchParams.set(key, String(value));
  return url;
}

function positiveId(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}

export function parseLastEdited(raw: string | null): LastEdited | null {
  try {
    const value: unknown = raw === null ? null : JSON.parse(raw);
    if (typeof value !== "object" || value === null || !("scope" in value) || !("title" in value)) return null;
    const { scope, title } = value;
    if (typeof title !== "string" || typeof scope !== "object" || scope === null) return null;
    if ("pieceId" in scope && "universeId" in scope && positiveId(scope.pieceId) && positiveId(scope.universeId)) {
      return { scope: { universeId: scope.universeId, pieceId: scope.pieceId }, title };
    }
    if ("seriesId" in scope && "bookId" in scope && "chapterId" in scope
      && positiveId(scope.seriesId) && positiveId(scope.bookId) && positiveId(scope.chapterId)) {
      return { scope: { seriesId: scope.seriesId, bookId: scope.bookId, chapterId: scope.chapterId }, title };
    }
    return null;
  } catch {
    return null;
  }
}
