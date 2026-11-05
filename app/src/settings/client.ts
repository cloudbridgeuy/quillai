import { apiErrorMessage } from "../api_error";
import { getApiBaseUrl } from "../api_config";
import { parseApiError } from "../manuscripts/contract";

export type AuthorGuidance = { text: string };

type Source = "saved" | "env" | "none";
export type AiSettings = { base_url: string | null; generate_model: string | null; edit_model: string | null; key_set: boolean; sources: { base_url: Source; api_key: Source; generate_model: Source; edit_model: Source } };
export type AiSettingsUpdate = { base_url?: string; api_key?: string; generate_model?: string; edit_model?: string };
export type AiSettingsForm = { base_url: string; model: string; key: { action: "keep" | "remove" } | { action: "replace"; value: string } };

export function aiKeyDecision(source: Source, form: AiSettingsForm["key"]): { showRemove: boolean; note: string; keyToSend: string | undefined } {
  const showRemove = source === "saved";
  const note = source === "env" ? "Set in quillai.env. Edit that file to change it." : source === "none" ? "No API key is set." : form.action === "remove" ? "The saved key will be removed on save. The quillai.env key, if present, will be used instead." : "";
  const keyToSend = form.action === "replace" ? form.value.trim() || undefined : form.action === "remove" && showRemove ? "" : undefined;
  return { showRemove, note, keyToSend };
}

export function aiSettingsUpdate(settings: AiSettings, form: AiSettingsForm): AiSettingsUpdate {
  const base_url = form.base_url.trim();
  const model = form.model.trim();
  const update: AiSettingsUpdate = {};
  if (base_url !== (settings.base_url ?? "").trim()) update.base_url = base_url;
  if (model !== (settings.generate_model ?? "").trim()) {
    update.generate_model = model;
    update.edit_model = model;
  }
  const { keyToSend } = aiKeyDecision(settings.sources.api_key, form.key);
  if (keyToSend !== undefined) update.api_key = keyToSend;
  return update;
}

function parseAiSettings(value: unknown): AiSettings {
  if (typeof value !== "object" || value === null || !("sources" in value) || typeof value.sources !== "object" || value.sources === null || !("base_url" in value) || !("generate_model" in value) || !("edit_model" in value) || !("key_set" in value) || (value.base_url !== null && typeof value.base_url !== "string") || (value.generate_model !== null && typeof value.generate_model !== "string") || (value.edit_model !== null && typeof value.edit_model !== "string") || typeof value.key_set !== "boolean") throw new Error("Expected AI settings");
  const sources = value.sources as Record<string, unknown>;
  if (![sources.base_url, sources.api_key, sources.generate_model, sources.edit_model].every(source => source === "saved" || source === "env" || source === "none")) throw new Error("Expected AI settings sources");
  return { base_url: value.base_url, generate_model: value.generate_model, edit_model: value.edit_model, key_set: value.key_set, sources: { base_url: sources.base_url as Source, api_key: sources.api_key as Source, generate_model: sources.generate_model as Source, edit_model: sources.edit_model as Source } };
}

export class SettingsApiError extends Error {
  constructor(message: string, readonly status: number, readonly code: string, readonly kind: "http" | "invalid_response", readonly detail?: string) {
    super(message);
    this.name = "SettingsApiError";
  }
}

function parseAuthorGuidance(value: unknown): AuthorGuidance {
  if (typeof value !== "object" || value === null || !("text" in value) || typeof value.text !== "string") {
    throw new Error("Expected author guidance text");
  }
  return { text: value.text };
}

export function createSettingsClient({
  baseUrl = getApiBaseUrl(),
  fetch: fetchRequest = globalThis.fetch,
}: { baseUrl?: string; fetch?: (url: string, init?: RequestInit) => Promise<Response> } = {}) {
  async function request<T>(path: string, parse: (value: unknown) => T, method: "GET" | "PUT", input?: AuthorGuidance | AiSettingsUpdate): Promise<T> {
    const response = await fetchRequest(`${baseUrl.replace(/\/+$/, "")}/settings/${path}`, {
      method,
      headers: { Accept: "application/json", ...(input === undefined ? {} : { "Content-Type": "application/json" }) },
      ...(input === undefined ? {} : { body: JSON.stringify(input) }),
    });
    if (!response.ok) {
      let error: { error: string; code: string };
      try {
        error = parseApiError(await response.json());
      } catch {
        throw new SettingsApiError(`HTTP ${response.status}: invalid error response`, response.status, "INVALID_RESPONSE", "invalid_response");
      }
      throw new SettingsApiError(apiErrorMessage(error.code, error.error), response.status, error.code, "http", error.error);
    }
    try {
      if (response.status !== 200) throw new Error("Expected HTTP 200");
      return parse(await response.json());
    } catch (error) {
      throw new SettingsApiError(error instanceof Error ? error.message : "Invalid response", response.status, "INVALID_RESPONSE", "invalid_response");
    }
  }
  return {
    getAuthorGuidance: () => request("author-guidance", parseAuthorGuidance, "GET"),
    putAuthorGuidance: (input: AuthorGuidance) => request("author-guidance", parseAuthorGuidance, "PUT", input),
    getAiSettings: () => request("ai", parseAiSettings, "GET"),
    putAiSettings: (input: AiSettingsUpdate) => request("ai", parseAiSettings, "PUT", input),
  };
}
