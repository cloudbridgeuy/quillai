import type { Container, EditorScope, Piece, PieceScope, Universe } from "./contract";

export type StoryRoot = Piece & { children: Piece[] };
export type StoryContainer = Container & { pieces: StoryRoot[] };
export type StoryUniverse = Universe & { containers: StoryContainer[]; loose: StoryRoot[] };

export function openUniverse(universes: StoryUniverse[], scope: EditorScope | null): StoryUniverse | null {
  if (!scope) return null;
  return universes.find(universe => "pieceId" in scope
    ? universe.id === scope.universeId
    : universe.containers.some(container => container.kind === "book_series" && container.id === scope.seriesId)) ?? null;
}

export function universeRoots(universe: StoryUniverse | null): { containers: StoryContainer[]; loose: StoryRoot[] } | null {
  if (!universe) return null;
  const containers = universe.containers.filter(container => container.kind !== "book_series");
  return containers.length || universe.loose.length ? { containers, loose: universe.loose } : null;
}

export function pieceScope(universe: StoryUniverse, piece: Piece): PieceScope | null {
  return piece.shape === "leaf" ? { universeId: universe.id, pieceId: piece.id } : null;
}

export function sameScope(left: EditorScope | null, right: EditorScope | null): boolean {
  if (!left || !right) return false;
  if ("pieceId" in left || "pieceId" in right) return "pieceId" in left && "pieceId" in right && left.pieceId === right.pieceId;
  return left.seriesId === right.seriesId && left.bookId === right.bookId && left.chapterId === right.chapterId;
}

export const containerKindLabel: Record<Container["kind"], string> = {
  book_series: "Book series",
  tv_series: "TV series",
  collection: "Collection",
};
