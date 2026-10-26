import { describe, expect, test } from "bun:test";
import { ManuscriptApiError } from "../manuscripts/client";
import type { Container, Piece, PieceDetail } from "../manuscripts/contract";
import { resolveResume, type ResumeClient } from "./resume";

const universe = { id: 1, title: "Harbor", position: 0 };
const shelf: Container = { id: 5, universeId: 1, title: "Shelf", position: 0, kind: "collection" };
const base: Piece = { id: 0, universeId: 1, containerId: null, parentId: null, title: "", position: 0, label: "short_story", format: "prose", shape: "leaf" };

function leaf(piece: Partial<Piece>): PieceDetail {
  return { ...base, ...piece, shape: "leaf", content: { ops: [] }, headRev: 0 };
}

function client(pieces: PieceDetail[], failure?: Error): ResumeClient {
  const fail = () => Promise.reject(failure ?? new Error("unused"));
  return {
    getPiece: async ({ pieceId }) => {
      if (failure) throw failure;
      const piece = pieces.find(item => item.id === pieceId);
      if (!piece) throw new ManuscriptApiError("not found", 404, "NOT_FOUND", "http");
      return piece;
    },
    getChapter: fail, getSeries: fail, getBook: fail, seriesUniverse: fail,
    listUniverses: async () => [universe],
    listContainers: async () => [shelf],
  };
}

const stored = (pieceId: number) => ({ scope: { universeId: 1, pieceId }, title: "Stored" });

describe("resolveResume", () => {
  test("loose piece names its universe and loose placement", async () => {
    expect(await resolveResume(stored(2), client([leaf({ id: 2, title: "Fresh" })])))
      .toEqual({ status: "ready", stored: stored(2), title: "Fresh", universeId: 1, path: ["Harbor", "Loose piece"] });
  });

  test("child piece names the container of its group and the group", async () => {
    const pieces = [leaf({ id: 3, title: "Book", containerId: 5 }), leaf({ id: 4, title: "One", parentId: 3 })];
    expect(await resolveResume(stored(4), client(pieces)))
      .toEqual({ status: "ready", stored: stored(4), title: "One", universeId: 1, path: ["Harbor", "Shelf", "Book"] });
  });

  test("deleted piece is missing and other errors fail", async () => {
    expect(await resolveResume(stored(9), client([]))).toEqual({ status: "missing", stored: stored(9) });
    expect(await resolveResume(stored(9), client([], new Error("offline"))))
      .toEqual({ status: "failed", stored: stored(9), message: "offline" });
  });
});
