import { ManuscriptApiError, type createManuscriptClient } from "../manuscripts/client";
import type { ChapterScope, PieceScope, SeriesScope, Universe } from "../manuscripts/contract";
import type { LastEdited } from "./routes";

export type ResumeClient = Pick<ReturnType<typeof createManuscriptClient>,
  "getPiece" | "getChapter" | "getSeries" | "getBook" | "listUniverses" | "listContainers"> & {
  seriesUniverse: (scope: SeriesScope) => Promise<number>;
};

export type ResumeState =
  | { status: "empty" }
  | { status: "checking"; stored: LastEdited }
  | { status: "ready"; stored: LastEdited; title: string; path: string[]; universeId: number }
  | { status: "missing"; stored: LastEdited }
  | { status: "failed"; stored: LastEdited; message: string };

type Found = { title: string; path: string[]; universeId: number };

function universeTitle(universes: Universe[], id: number): string | null {
  return universes.find(universe => universe.id === id)?.title ?? null;
}

function present(parts: (string | null)[]): string[] {
  return parts.filter((part): part is string => part !== null);
}

async function pieceTarget({ pieceId }: PieceScope, client: ResumeClient): Promise<Found> {
  const piece = await client.getPiece({ pieceId });
  const parent = piece.parentId === null ? null : await client.getPiece({ pieceId: piece.parentId });
  const containerId = parent?.containerId ?? piece.containerId;
  const [universes, containers] = await Promise.all([
    client.listUniverses(),
    containerId === null ? [] : client.listContainers({ universeId: piece.universeId }),
  ]);
  const container = containers.find(item => item.id === containerId)?.title ?? null;
  return {
    title: piece.title,
    universeId: piece.universeId,
    path: present([universeTitle(universes, piece.universeId), container ?? (parent ? null : "Loose piece"), parent?.title ?? null]),
  };
}

async function chapterTarget(scope: ChapterScope, client: ResumeClient): Promise<Found> {
  const [chapter, series, book, universeId] = await Promise.all([
    client.getChapter(scope), client.getSeries(scope), client.getBook(scope), client.seriesUniverse(scope),
  ]);
  const universes = await client.listUniverses();
  return { title: chapter.title, universeId, path: present([universeTitle(universes, universeId), series.title, book.title]) };
}

export async function resolveResume(stored: LastEdited, client: ResumeClient): Promise<ResumeState> {
  try {
    const found = "pieceId" in stored.scope ? await pieceTarget(stored.scope, client) : await chapterTarget(stored.scope, client);
    return { status: "ready", stored, ...found };
  } catch (error) {
    if (error instanceof ManuscriptApiError && error.status === 404) return { status: "missing", stored };
    return { status: "failed", stored, message: error instanceof Error ? error.message : "Could not check the last edited piece." };
  }
}
