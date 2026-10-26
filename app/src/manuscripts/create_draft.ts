import type { Book, BookScope, ChapterSummary, Container, ContainerKind, Format, Label, Piece, Series, Shape, Universe } from "./contract";
import type { StoryRoot, StoryUniverse } from "./story_tree";

export type ManuscriptSeries = Series & { books: (Book & { chapters: ChapterSummary[] })[] };
export type ManuscriptTreeData = { series: ManuscriptSeries[]; universes: StoryUniverse[] };
export type CreateTarget =
  | { kind: "universe" }
  | { kind: "container"; universeId: number }
  | { kind: "piece"; universeId: number; container: { id: number; kind: ContainerKind } | null }
  | { kind: "child"; universeId: number; pieceId: number; format: Format }
  | ({ kind: "chapter" } & BookScope);
export type DraftFields = { title: string; containerKind: ContainerKind; label: Label; format: Format };
export type CreateDraft = { target: CreateTarget; fields: DraftFields; error: string | null; pending: boolean } | null;
export type DraftAction =
  | { type: "start"; target: CreateTarget }
  | { type: "edit"; fields: Partial<DraftFields> }
  | { type: "cancel" } | { type: "created" } | { type: "submit" } | { type: "failed"; message: string };

const rootLabels = ["piece", "short_story", "novella", "novel", "screenplay", "tv_script", "episode"] as const;
const memberLabels: Record<ContainerKind, readonly Label[]> = {
  book_series: ["novel", "novella"],
  tv_series: ["episode", "tv_script"],
  collection: rootLabels,
};
const scriptLabels: readonly Label[] = ["screenplay", "tv_script", "episode"];
const groupLabels: readonly Label[] = ["novel", "novella", "screenplay"];

export const containerKinds: readonly ContainerKind[] = ["book_series", "tv_series", "collection"];
export const formats: readonly Format[] = ["prose", "script"];
export const labelNames: Record<Label, string> = {
  piece: "Piece", short_story: "Short story", novella: "Novella", novel: "Novel", screenplay: "Screenplay",
  tv_script: "TV script", episode: "Episode", prologue: "Prologue", chapter: "Chapter", epilogue: "Epilogue", scene: "Scene",
};

export function labelOptions(target: CreateTarget): readonly Label[] {
  if (target.kind === "piece") return target.container ? memberLabels[target.container.kind] : rootLabels;
  if (target.kind === "child" && target.format === "script") return ["scene"];
  if (target.kind === "child" || target.kind === "chapter") return ["chapter", "prologue", "epilogue"];
  return [];
}

export function defaultFormat(label: Label): Format {
  return scriptLabels.includes(label) ? "script" : "prose";
}

export function pieceShape(label: Label): Shape {
  return groupLabels.includes(label) ? "group" : "leaf";
}

export function childName(format: Format): string {
  return format === "script" ? "Scene" : "Chapter";
}

export function createTitle(value: string): string | null {
  return value.trim() || null;
}

export function nextPosition(items: readonly { position: number }[]): number {
  return items.reduce((maximum, item) => Math.max(maximum, item.position), -1) + 1;
}

function startFields(target: CreateTarget): DraftFields {
  const label = labelOptions(target)[0] ?? "piece";
  return { title: "", containerKind: "book_series", label, format: defaultFormat(label) };
}

export function draftTransition(draft: CreateDraft, action: DraftAction): CreateDraft {
  if (action.type === "created") return null;
  if (draft?.pending && action.type !== "failed") return draft;
  if (action.type === "start") return { target: action.target, fields: startFields(action.target), error: null, pending: false };
  if (action.type === "cancel") return null;
  if (!draft) return draft;
  if (action.type === "edit") {
    const label = action.fields.label;
    const format = label && !action.fields.format ? { format: defaultFormat(label) } : {};
    return { ...draft, fields: { ...draft.fields, ...action.fields, ...format }, error: null };
  }
  if (action.type === "failed") return { ...draft, pending: false, error: action.message };
  return createTitle(draft.fields.title) ? { ...draft, pending: true, error: null } : { ...draft, error: "Enter a title." };
}

function universeOf(tree: ManuscriptTreeData, universeId: number): StoryUniverse {
  const universe = tree.universes.find(item => item.id === universeId);
  if (!universe) throw new Error("Universe is no longer available.");
  return universe;
}

export function findRoot(universe: StoryUniverse, pieceId: number): StoryRoot | undefined {
  return [...universe.loose, ...universe.containers.flatMap(container => container.pieces)].find(piece => piece.id === pieceId);
}

export function targetPosition(tree: ManuscriptTreeData, target: CreateTarget): number {
  if (target.kind === "universe") return nextPosition(tree.universes);
  if (target.kind === "chapter") {
    const book = tree.series.find(item => item.id === target.seriesId)?.books.find(item => item.id === target.bookId);
    if (!book) throw new Error("Book is no longer available.");
    return nextPosition(book.chapters);
  }
  const universe = universeOf(tree, target.universeId);
  if (target.kind === "container") return nextPosition(universe.containers);
  if (target.kind === "child") {
    const root = findRoot(universe, target.pieceId);
    if (!root) throw new Error("Piece is no longer available.");
    return nextPosition(root.children);
  }
  if (!target.container) return nextPosition(universe.loose);
  const containerId = target.container.id;
  const container = universe.containers.find(item => item.id === containerId);
  if (!container) throw new Error("Container is no longer available.");
  return nextPosition(container.pieces);
}

function mapUniverse(tree: ManuscriptTreeData, universeId: number, update: (universe: StoryUniverse) => StoryUniverse): ManuscriptTreeData {
  return { ...tree, universes: tree.universes.map(universe => universe.id === universeId ? update(universe) : universe) };
}

export function addUniverse(tree: ManuscriptTreeData, universe: Universe): ManuscriptTreeData {
  return { ...tree, universes: [...tree.universes, { ...universe, containers: [], loose: [] }] };
}

export function addContainer(tree: ManuscriptTreeData, container: Container): ManuscriptTreeData {
  const added = mapUniverse(tree, container.universeId, universe => ({
    ...universe, containers: [...universe.containers, { ...container, pieces: [] }],
  }));
  if (container.kind !== "book_series") return added;
  const { id, title, position } = container;
  return { ...added, series: [...added.series, { id, title, position, books: [] }] };
}

export function addRoot(tree: ManuscriptTreeData, piece: Piece): ManuscriptTreeData {
  const root = { ...piece, children: [] };
  return mapUniverse(tree, piece.universeId, universe => piece.containerId === null
    ? { ...universe, loose: [...universe.loose, root] }
    : { ...universe, containers: universe.containers.map(container => container.id === piece.containerId
      ? { ...container, pieces: [...container.pieces, root] } : container) });
}

export function addChild(tree: ManuscriptTreeData, child: Piece): ManuscriptTreeData {
  const update = (root: StoryRoot) => root.id === child.parentId ? { ...root, children: [...root.children, child] } : root;
  return mapUniverse(tree, child.universeId, universe => ({
    ...universe,
    loose: universe.loose.map(update),
    containers: universe.containers.map(container => ({ ...container, pieces: container.pieces.map(update) })),
  }));
}

export function addChapter(tree: ManuscriptTreeData, scope: BookScope, chapter: ChapterSummary): ManuscriptTreeData {
  return { ...tree, series: tree.series.map(series => series.id === scope.seriesId
    ? { ...series, books: series.books.map(book => book.id === scope.bookId
      ? { ...book, chapters: [...book.chapters, chapter] } : book) } : series) };
}

export function withBooks(tree: ManuscriptTreeData, seriesId: number, books: Book[]): ManuscriptTreeData {
  return { ...tree, series: tree.series.map(series => series.id === seriesId
    ? { ...series, books: books.map(book => ({ ...book, chapters: series.books.find(item => item.id === book.id)?.chapters ?? [] })) }
    : series) };
}
