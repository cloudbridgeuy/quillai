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

export function treeUniverses(universes: StoryUniverse[], scope: EditorScope | null): { universe: StoryUniverse; open: boolean }[] {
  const current = openUniverse(universes, scope);
  if (!current) return universes.map(universe => ({ universe, open: true }));
  return [...universes.filter(universe => universe !== current).map(universe => ({ universe, open: false })), { universe: current, open: true }];
}

export function facadeBook(piece: Piece): boolean {
  return piece.shape === "group" && piece.label === "novel" && piece.format === "prose";
}

export function pieceRoots(container: StoryContainer): StoryRoot[] {
  return container.kind === "book_series" ? container.pieces.filter(piece => !facadeBook(piece)) : container.pieces;
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
