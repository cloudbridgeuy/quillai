import { apiErrorMessage } from "../api_error";
import { getApiBaseUrl } from "../api_config";
import { ManuscriptApiError } from "./client";
import { parseApiError, parseList } from "./contract";

const types = ["lore", "chapter", "piece", "book", "series", "collection", "universe"] as const;
export type TrashItem = {
  type: typeof types[number]; id: number; title: string; deletedAt: string;
  location: { universe: string | null; series: string | null; book: string | null };
  childCounts: { chapters: number; pieces: number; lore: number };
  deletedParents: string[]; resourcePath: string; restoreAncestors: string[];
};

function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Expected object");
  return value as Record<string, unknown>;
}
function string(value: unknown): string {
  if (typeof value !== "string") throw new Error("Expected string");
  return value;
}
function count(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new Error("Expected nonnegative integer");
  return value;
}
function path(value: unknown): string {
  const result = string(value);
  if (!/^\/(?:universes\/\d+(?:\/lore\/\d+)?|series\/\d+(?:\/books\/\d+(?:\/chapters\/\d+)?|\/lore\/\d+)?|containers\/\d+|pieces\/\d+)$/.test(result)) throw new Error("Invalid resource path");
  return result;
}
function parseItem(value: unknown): TrashItem {
  const item = object(value), location = object(item.location), counts = object(item.childCounts);
  const type = string(item.type);
  if (!types.some(candidate => candidate === type)) throw new Error("Invalid trash type");
  const id = count(item.id);
  if (id === 0) throw new Error("Expected positive id");
  const deletedAt = string(item.deletedAt);
  if (!deletedAt || Number.isNaN(Date.parse(deletedAt.replace(" ", "T") + (deletedAt.includes("T") ? "" : "Z")))) throw new Error("Invalid deletion date");
  const nullable = (v: unknown) => v === null ? null : string(v);
  return {
    type: type as TrashItem["type"], id, title: string(item.title), deletedAt,
    location: { universe: nullable(location.universe), series: nullable(location.series), book: nullable(location.book) },
    childCounts: { chapters: count(counts.chapters), pieces: count(counts.pieces), lore: count(counts.lore) },
    deletedParents: parseList(item.deletedParents, string), resourcePath: path(item.resourcePath),
    restoreAncestors: parseList(item.restoreAncestors, path),
  };
}

export function createTrashClient({ baseUrl = getApiBaseUrl(), fetch: fetchRequest = globalThis.fetch }: {
  baseUrl?: string; fetch?: (url: string, init?: RequestInit) => Promise<Response>;
} = {}) {
  const base = baseUrl.replace(/\/+$/, "");
  async function request<T>(path: string, method: string, status: number, parse: (value: unknown) => T): Promise<T> {
    const response = await fetchRequest(`${base}${path}`, { method, headers: { Accept: "application/json" } });
    if (!response.ok) {
      let error: { error: string; code: string };
      try { error = parseApiError(await response.json()); }
      catch { throw new ManuscriptApiError(`HTTP ${response.status}: invalid error response`, response.status, "INVALID_RESPONSE", "invalid_response"); }
      throw new ManuscriptApiError(apiErrorMessage(error.code, error.error), response.status, error.code, "http", error.error);
    }
    try {
      if (response.status !== status) throw new Error(`Expected HTTP ${status}`);
      return parse(status === 204 ? await response.text() : await response.json());
    } catch (error) {
      throw new ManuscriptApiError(error instanceof Error ? error.message : "Invalid response", response.status, "INVALID_RESPONSE", "invalid_response");
    }
  }
  async function ancestors(item: TrashItem) {
    for (const path of item.restoreAncestors) await request(`${path}/restore`, "POST", 200, object);
  }
  return {
    list: () => request("/trash", "GET", 200, value => parseList(value, parseItem)),
    restore: async (item: TrashItem) => { await ancestors(item); await request(`${item.resourcePath}/restore`, "POST", 200, object); },
    deleteForever: async (item: TrashItem) => {
      await ancestors(item);
      await request(`${item.resourcePath}?permanent=true`, "DELETE", 204, value => { if (value !== "") throw new Error("Expected empty response"); });
    },
  };
}
