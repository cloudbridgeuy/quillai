import { getApiBaseUrl } from "../api_config";
import type { LoreMatchEntry } from "../md_editor/lore_match";
import { ManuscriptApiError } from "./client";
import { parseApiError, parseList, type SeriesScope } from "./contract";

export type LoreEntry = LoreMatchEntry & { series_id: number; hidden_notes: string };
export type LoreInput = Omit<LoreEntry, "id" | "series_id">;
export type LoreScope = SeriesScope & { loreId: number };

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
    id: id(row.id), series_id: id(row.series_id), kind, level, pinned, name: term(row.name),
    keywords: parseList(row.keywords, term), aliases: parseList(row.aliases, term),
    visible_text: text(row.visible_text), hidden_notes: text(row.hidden_notes),
  };
}

export function createLoreClient({ baseUrl = getApiBaseUrl(), fetch: fetchRequest = globalThis.fetch }: {
  baseUrl?: string; fetch?: (url: string, init?: RequestInit) => Promise<Response>;
} = {}) {
  async function request<T>(scope: SeriesScope | LoreScope, method: string, status: number, parse: (value: unknown) => T, body?: unknown): Promise<T> {
    const path = `/series/${encodeURIComponent(scope.seriesId)}/lore${"loreId" in scope ? `/${encodeURIComponent(scope.loreId)}` : ""}`;
    const response = await fetchRequest(`${baseUrl.replace(/\/+$/, "")}${path}`, {
      method, headers: { Accept: "application/json", ...(body === undefined ? {} : { "Content-Type": "application/json" }) },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    if (!response.ok) {
      let error: { error: string; code: string };
      try { error = parseApiError(await response.json()); } catch {
        throw new ManuscriptApiError(`HTTP ${response.status}: invalid error response`, response.status, "INVALID_RESPONSE", "invalid_response");
      }
      throw new ManuscriptApiError(error.error, response.status, error.code, "http");
    }
    try {
      if (response.status !== status) throw new Error(`Expected HTTP ${status}`);
      return parse(status === 204 ? await response.text() : await response.json());
    } catch (error) {
      throw new ManuscriptApiError(error instanceof Error ? error.message : "Invalid response", response.status, "INVALID_RESPONSE", "invalid_response");
    }
  }
  return {
    list: (scope: SeriesScope) => request(scope, "GET", 200, value => parseList(value, parseLoreEntry)),
    get: (scope: LoreScope) => request(scope, "GET", 200, parseLoreEntry),
    create: (scope: SeriesScope, input: LoreInput) => request(scope, "POST", 201, parseLoreEntry, input),
    update: (scope: LoreScope, input: Partial<LoreInput>) => request(scope, "PUT", 200, parseLoreEntry, input),
    delete: (scope: LoreScope) => request(scope, "DELETE", 204, value => {
      if (value !== "") throw new Error("Expected empty delete response");
    }),
  };
}
