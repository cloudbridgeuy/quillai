import type { EditorScope } from "../manuscripts/contract";
import { scriptGroupId } from "./routes";

type Activity = {
  pieceId: number; universeId: number; title: string; label: string; format: "prose" | "script";
  parentId: number | null; scriptTitle: string | null; changedAt: string; chapter: { seriesId: number; bookId: number; chapterId: number } | null;
};
type ActivityRow = { title: string; changedAt: string; scope: EditorScope };

function id(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function parseActivity(value: unknown): Activity[] {
  if (!Array.isArray(value)) throw new Error("Expected activity list");
  return value.flatMap(row => {
    if (!record(row) || !id(row.pieceId) || !id(row.universeId) || typeof row.title !== "string"
      || typeof row.label !== "string" || (row.format !== "prose" && row.format !== "script")
      || (row.scriptTitle !== null && typeof row.scriptTitle !== "string")
      || (row.parentId !== null && !id(row.parentId)) || typeof row.changedAt !== "string"
      || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/.test(row.changedAt)
      || !Number.isFinite(Date.parse(row.changedAt))
      || new Date(row.changedAt).toISOString().slice(0, 19) !== row.changedAt.slice(0, 19)) return [];
    let chapter: Activity["chapter"] = null;
    if (row.chapter !== null) {
      const value = row.chapter;
      if (!record(value) || !id(value.seriesId) || !id(value.bookId) || !id(value.chapterId)) return [];
      chapter = { seriesId: value.seriesId, bookId: value.bookId, chapterId: value.chapterId };
    }
    return [{ pieceId: row.pieceId, universeId: row.universeId, title: row.title, label: row.label,
      format: row.format, parentId: row.parentId, scriptTitle: row.scriptTitle, changedAt: row.changedAt,
      chapter }];
  });
}

export function activityGroups(rows: Activity[], now: Date): { label: string; rows: ActivityRow[] }[] {
  const unique = new Map<string, ActivityRow>();
  for (const row of rows) {
    const groupId = scriptGroupId({ id: row.pieceId, format: row.format, shape: "leaf", parentId: row.parentId });
    const scope: EditorScope = row.chapter ?? { universeId: row.universeId, pieceId: groupId ?? row.pieceId };
    const key = "pieceId" in scope ? `p:${scope.universeId}:${scope.pieceId}` : `c:${scope.seriesId}:${scope.bookId}:${scope.chapterId}`;
    const previous = unique.get(key);
    if (!previous || Date.parse(row.changedAt) > Date.parse(previous.changedAt)) {
      unique.set(key, { title: groupId ? row.scriptTitle ?? row.title : row.title, changedAt: row.changedAt, scope });
    }
  }
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1).getTime();
  const groups = ["Today", "Yesterday", "Earlier"].map(label => ({ label, rows: [] as ActivityRow[] }));
  for (const row of [...unique.values()].sort((a, b) => Date.parse(b.changedAt) - Date.parse(a.changedAt))) {
    const date = new Date(row.changedAt);
    const day = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
    groups[day === today ? 0 : day === yesterday ? 1 : 2]!.rows.push(row);
  }
  return groups.filter(group => group.rows.length > 0);
}
