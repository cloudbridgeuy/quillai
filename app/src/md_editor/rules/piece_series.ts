import { useEffect, useState } from "react";
import type { createManuscriptClient } from "../../manuscripts/client";
import type { PieceScope } from "../../manuscripts/contract";

type PieceClient = Pick<ReturnType<typeof createManuscriptClient>, "getPiece" | "listContainers">;

export async function resolvePieceSeries(client: PieceClient, { universeId, pieceId }: PieceScope): Promise<number | null> {
  const piece = await client.getPiece({ pieceId });
  const containerId = piece.containerId ?? (piece.parentId === null ? null : (await client.getPiece({ pieceId: piece.parentId })).containerId);
  if (containerId === null) return null;
  const containers = await client.listContainers({ universeId });
  return containers.find((container) => container.id === containerId && container.kind === "book_series")?.id ?? null;
}

export function usePieceSeries(client: PieceClient, scope: PieceScope | null): number | null {
  const [seriesId, setSeriesId] = useState<number | null>(null);
  const universeId = scope?.universeId;
  const pieceId = scope?.pieceId;
  useEffect(() => {
    setSeriesId(null);
    if (universeId === undefined || pieceId === undefined) return;
    let active = true;
    resolvePieceSeries(client, { universeId, pieceId }).then((id) => active && setSeriesId(id)).catch(() => active && setSeriesId(null));
    return () => { active = false; };
  }, [client, universeId, pieceId]);
  return seriesId;
}
