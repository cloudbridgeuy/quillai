export type Series = { id: number; title: string; position: number };
export type Book = Series & { series_id: number };
export type ChapterKind = "chapter" | "prologue" | "epilogue";
export type DeltaOperation = (
  | { insert: string | Record<string, unknown>; retain?: never; delete?: never }
  | { retain: number | Record<string, unknown>; insert?: never; delete?: never }
  | { delete: number; insert?: never; retain?: never }
) & { attributes?: Record<string, unknown> | null };
export type ManuscriptDelta = { ops: DeltaOperation[] };
export type ChapterSummary = Series & {
  kind: ChapterKind;
  head_rev: number;
};
export type Chapter = ChapterSummary & { book_id: number; content: ManuscriptDelta };
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
export type UniverseScope = { universeId: number };
export type PieceScope = UniverseScope & { pieceId: number };
export type EditorScope = ChapterScope | PieceScope;
export type ContainerScope = { containerId: number };
export type Universe = Series;
export type ContainerKind = "book_series" | "tv_series" | "collection";
export type Container = Series & { universeId: number; kind: ContainerKind };
export type Label = "piece" | "short_story" | "novella" | "novel" | "screenplay" | "tv_script"
  | "episode" | "prologue" | "chapter" | "epilogue" | "scene";
export type Format = "prose" | "script";
export type Shape = "leaf" | "group";
export type Piece = Series & {
  universeId: number;
  containerId: number | null;
  parentId: number | null;
  label: string;
  format: Format;
  shape: Shape;
};
export type CreateUniverse = CreateSeries;
export type CreateContainer = CreateSeries & { kind: ContainerKind };
export type CreatePiece = CreateSeries & { label: Label; shape: Shape; format: Format };
export type UpdatePiece = { position: number };
export type CreateChild = CreateSeries & { label: Label };
export type PieceDetail =
  | Piece & { shape: "leaf"; content: ManuscriptDelta; headRev: number }
  | Piece & { shape: "group"; children: Piece[] };
export type Scene = Piece & { shape: "leaf"; content: ManuscriptDelta; headRev: number };
export type ScriptGroup = Piece & { scenes: Scene[] };
export type SceneDeltas = { revisions: Revision[] } | { conflict: string };
export type ScenePoll = { children: number[]; scenes: Map<number, SceneDeltas> };

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

export function parseChapterSummary(value: unknown): ChapterSummary {
  const row = record(value);
  return {
    ...parseSeries(row),
    kind: chapterKind(row.kind),
    head_rev: integer(row.head_rev),
  };
}

export function parseChapter(value: unknown): Chapter {
  const row = record(value);
  return { ...parseChapterSummary(row), book_id: integer(row.book_id, 1), content: parseDelta(row.content) };
}

function nullableId(value: unknown): number | null {
  return value === null ? null : integer(value, 1);
}

function oneOf<T extends string>(value: unknown, options: readonly T[]): T {
  if (!options.includes(value as T)) throw new Error(`Expected one of ${options.join(", ")}`);
  return value as T;
}

export function parseContainer(value: unknown): Container {
  const row = record(value);
  return {
    ...parseSeries(row), universeId: integer(row.universeId, 1),
    kind: oneOf(row.kind, ["book_series", "tv_series", "collection"] as const),
  };
}

export function parsePiece(value: unknown): Piece {
  const row = record(value);
  if (typeof row.label !== "string") throw new Error("Expected a piece label");
  return {
    ...parseSeries(row), universeId: integer(row.universeId, 1), containerId: nullableId(row.containerId),
    parentId: nullableId(row.parentId), label: row.label, format: oneOf(row.format, ["prose", "script"] as const),
    shape: oneOf(row.shape, ["leaf", "group"] as const),
  };
}

export function parsePieceDetail(value: unknown): PieceDetail {
  const row = record(value);
  const piece = parsePiece(row);
  return piece.shape === "leaf"
    ? { ...piece, shape: "leaf", content: parseDelta(row.content), headRev: integer(row.headRev) }
    : { ...piece, shape: "group", children: parseList(row.children, parsePiece) };
}

export function parseScriptGroup(value: unknown): ScriptGroup {
  const row = record(value);
  return {
    ...parsePiece(row),
    scenes: parseList(row.scenes, (scene) => {
      const detail = parsePieceDetail(scene);
      if (detail.shape !== "leaf") throw new Error("Expected a scene leaf");
      return detail;
    }),
  };
}

export function parseScenePoll(value: unknown): ScenePoll {
  const row = record(value);
  const scenes = new Map(Object.entries(record(row.scenes)).map(([id, entry]): [number, SceneDeltas] => {
    const scene = record(entry);
    return [integer(Number(id), 1), typeof scene.conflict === "string"
      ? { conflict: scene.conflict }
      : { revisions: parseList(scene.revisions, parseRevision) }];
  }));
  return { children: parseList(row.children, (id) => integer(id, 1)), scenes };
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
