import { useEffect, useMemo, useState } from "react";
import type { EditorScope } from "./contract";
import { effectivePoint, resolveLoreEntry, type PieceRef, type ResolvedLore, type TimelinePoint } from "../md_editor/lore_states";
import { createLoreClient, type LoreEntry } from "./lore_client";
import type { NamedPoint } from "./timeline_panel";

type LoreState = { key: string | null; universeId: number | null; entries: LoreEntry[]; status: "loading" | "ready" | "failed"; error: string };

const EMPTY_POINTS: NamedPoint[] = [];
const EMPTY_ENTRIES: LoreEntry[] = [];

export function useLore(scope: EditorScope | null) {
  const [client] = useState(createLoreClient);
  const [attempt, setAttempt] = useState(0);
  const pieceId = scope && "pieceId" in scope ? scope.pieceId : null;
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
  const universeId = current ? state.universeId : null;
  // Timeline points and the piece's point are optional extras: a failure only means base text.
  const [timelineAttempt, setTimelineAttempt] = useState(0);
  const [timeline, setTimeline] = useState<{ universeId: number; points: NamedPoint[] } | null>(null);
  useEffect(() => {
    if (universeId === null) return;
    let active = true;
    client.timeline({ universeId }).catch((): NamedPoint[] => []).then(points => { if (active) setTimeline({ universeId, points }); });
    return () => { active = false; };
  }, [universeId, client, timelineAttempt]);
  const [pieceRef, setPieceRef] = useState<{ pieceId: number; piece: PieceRef | null } | null>(null);
  useEffect(() => {
    if (pieceId === null) return;
    let active = true;
    client.pieceRef({ pieceId }).catch(() => null).then(piece => { if (active) setPieceRef({ pieceId, piece }); });
    return () => { active = false; };
  }, [pieceId, client, timelineAttempt]);
  const points = timeline && timeline.universeId === universeId ? timeline.points : EMPTY_POINTS;
  const piece = pieceRef && pieceRef.pieceId === pieceId ? pieceRef.piece : null;
  const entries = current ? state.entries : EMPTY_ENTRIES;
  const resolved = useMemo<ResolvedLore<LoreEntry>[]>(() => {
    const byId = new Map(points.map(point => [point.id, point]));
    const point = effectivePoint(piece, byId);
    return entries.map(entry => resolveLoreEntry(entry, entry.states ?? [], point, byId));
  }, [entries, points, piece]);
  return {
    client, resolved, universeId, entries, points, piece,
    status: current ? state.status : "loading", error: state.error,
    /** Reload timeline points and the piece's point after the author changed them. */
    refreshTimeline: () => setTimelineAttempt(value => value + 1),
    retry: () => setAttempt(value => value + 1),
    change: (entries: LoreEntry[]) => setState(value => value.key === key ? { ...value, entries } : value),
  };
}
