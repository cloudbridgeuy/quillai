import { apiErrorMessage } from "../api_error";
import { getApiBaseUrl } from "../api_config";
import type { LoreMatchEntry } from "../md_editor/lore_match";
import type { LoreStateRow, PieceRef, TimelinePoint } from "../md_editor/lore_states";
import { ManuscriptApiError } from "./client";
import { parseApiError, parseContainer, parseList, type SeriesScope, type UniverseScope } from "./contract";

export type LoreEntry = LoreMatchEntry & { universe_id: number; hidden_notes: string; states?: readonly LoreStateRow[] };
export type LoreInput = Omit<LoreEntry, "id" | "universe_id" | "states">;
export type LoreScope = UniverseScope & { loreId: number };

export function parseLoreEntry(value: unknown): LoreEntry {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("Expected lore object");
  const row = value as Record<string, unknown>;
  function id(value: unknown): number {
    if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 1) throw new Error("Expected lore ID");
    return value;
  }
  function text(value: unknown): string {
    if (typeof value !== "string") throw new Error("Expected lore text");
    return value;
  }
  function term(value: unknown): string {
    const result = text(value);
    if (!result.trim()) throw new Error("Expected nonblank lore term");
    return result;
  }
  const { kind, level, pinned } = row;
  if (kind !== "character" && kind !== "place" && kind !== "term" && kind !== "note") throw new Error("Invalid lore kind");
  if (level !== "include" && level !== "known_dont_reveal" && level !== "leave_out") throw new Error("Invalid lore level");
  if (typeof pinned !== "boolean") throw new Error("Expected lore pin boolean");
  return {
    id: id(row.id), universe_id: id(row.universe_id), kind, level, pinned, name: term(row.name),
    keywords: parseList(row.keywords, term), aliases: parseList(row.aliases, term),
    visible_text: text(row.visible_text), hidden_notes: text(row.hidden_notes),
    ...(row.states === undefined ? {} : { states: parseList(row.states, parseLoreState) }),
  };
}

function parseLoreState(value: unknown): LoreStateRow {
  if (typeof value !== "object" || value === null) throw new Error("Expected lore state object");
  const row = value as Record<string, unknown>;
  const { id, pointId, name, visibleText, hiddenNotes } = row;
  if (typeof id !== "number" || typeof pointId !== "number" || typeof name !== "string"
    || typeof visibleText !== "string" || typeof hiddenNotes !== "string") throw new Error("Invalid lore state");
  return { id, name, pointId, visibleText, hiddenNotes };
}

export function parseTimelinePoint(value: unknown): TimelinePoint {
  const row = (typeof value === "object" && value !== null ? value : {}) as Record<string, unknown>;
  if (typeof row.id !== "number" || typeof row.position !== "number") throw new Error("Invalid timeline point");
  return { id: row.id, position: row.position };
}

function parsePieceFields(value: unknown): { parentId: number | null; timelinePointId: number | null } {
  const row = (typeof value === "object" && value !== null ? value : {}) as Record<string, unknown>;
  const optional = (field: unknown) => typeof field === "number" ? field : null;
  return { parentId: optional(row.parentId), timelinePointId: optional(row.timelinePointId) };
}

export function createLoreClient({ baseUrl = getApiBaseUrl(), fetch: fetchRequest = globalThis.fetch }: {
  baseUrl?: string; fetch?: (url: string, init?: RequestInit) => Promise<Response>;
} = {}) {
  function lorePath(scope: UniverseScope | LoreScope): string {
    return `/universes/${encodeURIComponent(scope.universeId)}/lore${"loreId" in scope ? `/${encodeURIComponent(scope.loreId)}` : ""}`;
  }
  async function request<T>(path: string, method: string, status: number, parse: (value: unknown) => T, body?: unknown): Promise<T> {
    const response = await fetchRequest(`${baseUrl.replace(/\/+$/, "")}${path}`, {
      method, headers: { Accept: "application/json", ...(body === undefined ? {} : { "Content-Type": "application/json" }) },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    if (!response.ok) {
      let error: { error: string; code: string };
      try { error = parseApiError(await response.json()); } catch {
        throw new ManuscriptApiError(`HTTP ${response.status}: invalid error response`, response.status, "INVALID_RESPONSE", "invalid_response");
      }
      throw new ManuscriptApiError(apiErrorMessage(error.code, error.error), response.status, error.code, "http", error.error);
    }
    try {
      if (response.status !== status) throw new Error(`Expected HTTP ${status}`);
      return parse(status === 204 ? await response.text() : await response.json());
    } catch (error) {
      throw new ManuscriptApiError(error instanceof Error ? error.message : "Invalid response", response.status, "INVALID_RESPONSE", "invalid_response");
    }
  }
  return {
    seriesUniverse: ({ seriesId }: SeriesScope) => request(`/containers/${encodeURIComponent(seriesId)}`, "GET", 200,
      value => parseContainer(value).universeId),
    timeline: (scope: UniverseScope) => request(`/universes/${encodeURIComponent(scope.universeId)}/timeline`, "GET", 200,
      value => parseList(value, parseTimelinePoint)),
    /** Piece plus its parent (groups do not nest), each with its own timeline point. */
    pieceRef: async ({ pieceId }: { pieceId: number }): Promise<PieceRef> => {
      const fields = (id: number) => request(`/pieces/${encodeURIComponent(id)}`, "GET", 200, parsePieceFields);
      const own = await fields(pieceId);
      const parent = own.parentId === null ? null : await fields(own.parentId);
      return { timelinePointId: own.timelinePointId, parent: parent && { timelinePointId: parent.timelinePointId, parent: null } };
    },
    list: (scope: UniverseScope) => request(lorePath(scope), "GET", 200, value => parseList(value, parseLoreEntry)),
    get: (scope: LoreScope) => request(lorePath(scope), "GET", 200, parseLoreEntry),
    create: (scope: UniverseScope, input: LoreInput) => request(lorePath(scope), "POST", 201, parseLoreEntry, input),
    update: (scope: LoreScope, input: Partial<LoreInput>) => request(lorePath(scope), "PUT", 200, parseLoreEntry, input),
    delete: (scope: LoreScope) => request(lorePath(scope), "DELETE", 204, value => {
      if (value !== "") throw new Error("Expected empty delete response");
    }),
  };
}
