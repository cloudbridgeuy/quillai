import { expect, test } from "bun:test";
import type { Container, Piece, PieceDetail } from "../../manuscripts/contract";
import { resolvePieceSeries } from "./piece_series";

const piece = (id: number, containerId: number | null, parentId: number | null, shape: "leaf" | "group"): PieceDetail => {
  const base: Piece = { id, title: `p${id}`, position: 0, universeId: 1, containerId, parentId, label: "piece", format: "prose", shape };
  return shape === "leaf" ? { ...base, shape: "leaf", content: { ops: [] }, headRev: 0 } : { ...base, shape: "group", children: [] };
};
const container = (id: number, kind: Container["kind"]): Container => ({ id, title: `c${id}`, position: 0, universeId: 1, kind });

function setup(pieces: PieceDetail[], containers: Container[]) {
  const calls: string[] = [];
  const client = {
    getPiece: async ({ pieceId }: { pieceId: number }) => { calls.push(`getPiece:${pieceId}`); return pieces.find((candidate) => candidate.id === pieceId)!; },
    listContainers: async () => { calls.push("listContainers"); return containers; },
  };
  return { client, calls };
}

test("a leaf piece in a book_series resolves to that series", async () => {
  const { client, calls } = setup([piece(10, 5, null, "leaf")], [container(5, "book_series")]);
  expect(await resolvePieceSeries(client, { universeId: 1, pieceId: 10 })).toBe(5);
  expect(calls).toEqual(["getPiece:10", "listContainers"]);
});

test("a child of a group resolves through the group's container", async () => {
  const { client, calls } = setup([piece(11, null, 20, "leaf"), piece(20, 6, null, "group")], [container(6, "book_series")]);
  expect(await resolvePieceSeries(client, { universeId: 1, pieceId: 11 })).toBe(6);
  expect(calls).toEqual(["getPiece:11", "getPiece:20", "listContainers"]);
});

test("a container that is not a book_series resolves to null", async () => {
  const { client } = setup([piece(12, 7, null, "leaf")], [container(7, "tv_series")]);
  expect(await resolvePieceSeries(client, { universeId: 1, pieceId: 12 })).toBeNull();
});

test("a loose piece resolves to null without listing containers", async () => {
  const { client, calls } = setup([piece(13, null, null, "leaf")], [container(5, "book_series")]);
  expect(await resolvePieceSeries(client, { universeId: 1, pieceId: 13 })).toBeNull();
  expect(calls).toEqual(["getPiece:13"]);
});
