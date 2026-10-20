import { useEffect, useState } from "react";
import { createLoreClient, type LoreEntry } from "./lore_client";

export function useLore(seriesId: number | null) {
  const [client] = useState(createLoreClient);
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<{ seriesId: number | null; entries: LoreEntry[]; status: "loading" | "ready" | "failed"; error: string }>({ seriesId: null, entries: [], status: "loading", error: "" });
  useEffect(() => {
    let active = true;
    setState({ seriesId, entries: [], status: "loading", error: "" });
    if (seriesId !== null) client.list({ seriesId }).then(entries => {
      if (active) setState({ seriesId, entries, status: "ready", error: "" });
    }).catch((error: unknown) => {
      if (active) setState({ seriesId, entries: [], status: "failed", error: error instanceof Error ? error.message : "Could not load lore." });
    });
    return () => { active = false; };
  }, [seriesId, client, attempt]);
  return {
    client, entries: state.seriesId === seriesId ? state.entries : [],
    status: state.seriesId === seriesId ? state.status : "loading", error: state.error,
    retry: () => setAttempt(value => value + 1),
    change: (entries: LoreEntry[]) => setState(current => current.seriesId === seriesId ? { ...current, entries } : current),
  };
}
