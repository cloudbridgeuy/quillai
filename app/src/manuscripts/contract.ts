export type Series = { id: number; title: string; position: number };
export type Book = Series & { series_id: number };
export type ChapterKind = "chapter" | "prologue" | "epilogue";
export type DeltaOperation = (
  | { insert: string | Record<string, unknown>; retain?: never; delete?: never }
  | { retain: number | Record<string, unknown>; insert?: never; delete?: never }
  | { delete: number; insert?: never; retain?: never }
) & { attributes?: Record<string, unknown> | null };
export type ManuscriptDelta = { ops: DeltaOperation[] };
export type Chapter = Series & {
  book_id: number;
  kind: ChapterKind;
  content: ManuscriptDelta;
  head_rev: number;
};
export type Revision = { rev: number; delta: ManuscriptDelta };
export type CreateSeries = Pick<Series, "title" | "position">;
export type UpdateSeries = { title?: string | null; position?: number | null };
export type CreateBook = CreateSeries;
export type UpdateBook = UpdateSeries;
export type CreateChapter = CreateSeries & { kind: ChapterKind; content?: ManuscriptDelta };
export type UpdateChapter = UpdateSeries & { kind?: ChapterKind | null };
export type SubmitDelta = { baseRev: number; delta: ManuscriptDelta };
export type SeriesScope = { seriesId: number };
export type BookScope = SeriesScope & { bookId: number };
export type ChapterScope = BookScope & { chapterId: number };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function record(value: unknown): Record<string, unknown> {
  if (!isRecord(value)) throw new Error("Expected an object");
  return value;
}

function integer(value: unknown, minimum = 0): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < minimum) {
    throw new Error("Expected a safe integer");
  }
  return value;
}

function title(value: unknown): string {
  if (typeof value !== "string" || !value.trim()) throw new Error("Expected a nonblank title");
  return value;
}

function chapterKind(value: unknown): ChapterKind {
  if (value !== "chapter" && value !== "prologue" && value !== "epilogue") {
    throw new Error("Invalid chapter kind");
  }
  return value;
}

export function parseSeries(value: unknown): Series {
  const row = record(value);
  return { id: integer(row.id, 1), title: title(row.title), position: integer(row.position) };
}

export function parseBook(value: unknown): Book {
  const row = record(value);
  return { ...parseSeries(row), series_id: integer(row.series_id, 1) };
}

export function parseChapter(value: unknown): Chapter {
  const row = record(value);
  return {
    ...parseSeries(row),
    book_id: integer(row.book_id, 1),
    kind: chapterKind(row.kind),
    content: parseDelta(row.content),
    head_rev: integer(row.head_rev),
  };
}

export function parseDelta(value: unknown): ManuscriptDelta {
  let length = 0;
  const ops = parseList(record(value).ops, (value): DeltaOperation => {
    const op = record(value);
    const actions = ["insert", "retain", "delete"];
    if (actions.filter((key) => Object.hasOwn(op, key)).length !== 1 ||
      Object.keys(op).some((key) => !actions.includes(key) && key !== "attributes")) {
      throw new Error("Invalid delta operation");
    }
    const attributes = Object.hasOwn(op, "attributes")
      ? { attributes: op.attributes === null ? null : record(op.attributes) }
      : {};
    let result: DeltaOperation;
    let count: number;
    if (Object.hasOwn(op, "insert")) {
      const insert = typeof op.insert === "string" ? op.insert : record(op.insert);
      count = typeof insert === "string" ? insert.length : 1;
      result = { insert, ...attributes };
    } else if (Object.hasOwn(op, "retain")) {
      const retain = typeof op.retain === "number" ? integer(op.retain) : record(op.retain);
      count = typeof retain === "number" ? retain : 1;
      result = { retain, ...attributes };
    } else {
      count = integer(op.delete);
      result = { delete: count, ...attributes };
    }
    length = integer(length + count);
    return result;
  });
  return { ops };
}

export function parseRevision(value: unknown): Revision {
  const row = record(value);
  return { rev: integer(row.rev, 1), delta: parseDelta(row.delta) };
}

export function parseList<T>(value: unknown, parse: (value: unknown) => T): T[] {
  if (!Array.isArray(value)) throw new Error("Expected an array");
  return value.map(parse);
}

export function parseApiError(value: unknown): { error: string; code: string } {
  const row = record(value);
  if (typeof row.error !== "string" || typeof row.code !== "string") {
    throw new Error("Expected an API error");
  }
  return { error: row.error, code: row.code };
}
