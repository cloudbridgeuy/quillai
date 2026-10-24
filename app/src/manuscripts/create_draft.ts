import type { Book, BookScope, ChapterSummary, Series } from "./contract";

export type ManuscriptSeries = Series & { books: (Book & { chapters: ChapterSummary[] })[] };
export type CreateTarget = { kind: "series" } | { kind: "book"; seriesId: number } | ({ kind: "chapter" } & BookScope);
export type CreateDraft = { target: CreateTarget; title: string; error: string | null; pending: boolean } | null;
export type DraftAction = { type: "start"; target: CreateTarget } | { type: "title"; title: string } | { type: "cancel" } | { type: "created" } | { type: "submit" } | { type: "failed"; message: string };

export function createTitle(value: string): string | null {
  return value.trim() || null;
}

export function nextPosition(items: readonly { position: number }[]): number {
  return items.reduce((maximum, item) => Math.max(maximum, item.position), -1) + 1;
}

export function draftTransition(draft: CreateDraft, action: DraftAction): CreateDraft {
  if (action.type === "created") return null;
  if (draft?.pending && action.type !== "failed") return draft;
  if (action.type === "start") return { target: action.target, title: "", error: null, pending: false };
  if (action.type === "cancel") return null;
  if (!draft) return draft;
  if (action.type === "title") return { ...draft, title: action.title, error: null };
  if (action.type === "failed") return { ...draft, pending: false, error: action.message };
  return createTitle(draft.title) ? { ...draft, pending: true, error: null } : { ...draft, error: "Enter a title." };
}

export function targetPosition(series: ManuscriptSeries[], target: CreateTarget): number {
  if (target.kind === "series") return nextPosition(series);
  const parent = series.find(item => item.id === target.seriesId);
  if (!parent) throw new Error("Series is no longer available.");
  if (target.kind === "book") return nextPosition(parent.books);
  const book = parent.books.find(item => item.id === target.bookId);
  if (!book) throw new Error("Book is no longer available.");
  return nextPosition(book.chapters);
}
