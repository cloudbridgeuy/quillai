import { useEffect, useState } from "react";
import type { EditorScope } from "./contract";
import { createLoreClient, type LoreEntry } from "./lore_client";

type LoreState = { key: string | null; universeId: number | null; entries: LoreEntry[]; status: "loading" | "ready" | "failed"; error: string };

export function useLore(scope: EditorScope | null) {
  const [client] = useState(createLoreClient);
  const [attempt, setAttempt] = useState(0);
  const pieceUniverseId = scope && "pieceId" in scope ? scope.universeId : null;
  const seriesId = scope && !("pieceId" in scope) ? scope.seriesId : null;
  const key = pieceUniverseId !== null ? `universe:${pieceUniverseId}` : seriesId !== null ? `series:${seriesId}` : null;
  const [state, setState] = useState<LoreState>({ key: null, universeId: null, entries: [], status: "loading", error: "" });
  useEffect(() => {
    let active = true;
    setState({ key, universeId: null, entries: [], status: "loading", error: "" });
    async function load(universeId: number | null, seriesId: number | null) {
      const resolved = universeId ?? (seriesId === null ? null : await client.seriesUniverse({ seriesId }));
      if (resolved === null) return;
      const entries = await client.list({ universeId: resolved });
      if (active) setState({ key, universeId: resolved, entries, status: "ready", error: "" });
    }
    load(pieceUniverseId, seriesId).catch((error: unknown) => {
      if (active) setState({ key, universeId: null, entries: [], status: "failed", error: error instanceof Error ? error.message : "Could not load lore." });
    });
    return () => { active = false; };
  }, [key, pieceUniverseId, seriesId, client, attempt]);
  const current = state.key === key;
  return {
    client, universeId: current ? state.universeId : null, entries: current ? state.entries : [],
    status: current ? state.status : "loading", error: state.error,
    retry: () => setAttempt(value => value + 1),
    change: (entries: LoreEntry[]) => setState(value => value.key === key ? { ...value, entries } : value),
  };
}
