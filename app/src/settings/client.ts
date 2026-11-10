import { apiErrorMessage } from "../api_error";
import { getApiBaseUrl } from "../api_config";
import { parseApiError } from "../manuscripts/contract";
import type { PromptAiAction } from "../md_editor/ai_contract";
import { parseModelCatalog, type ModelAction, type ModelCatalog } from "../md_editor/model_catalog";

export type AuthorGuidance = { text: string };

type Source = "saved" | "env" | "none";
type ModelField = "generate_model" | "edit_model" | "image_model" | "lore_complete_model" | "dialogue_model";
export type AiSettings = { base_url: string | null; generate_model: string | null; edit_model: string | null; image_model?: string | null; lore_complete_model?: string | null; dialogue_model?: string | null; key_set: boolean; sources: Record<"base_url" | "api_key" | "generate_model" | "edit_model", Source> & { image_model?: Source; lore_complete_model?: Source; dialogue_model?: Source } };
export type AiSettingsUpdate = { base_url?: string; api_key?: string } & Partial<Record<ModelField, string>>;
export type AiSettingsForm = { base_url: string; models: Record<PromptAiAction, string> & { image?: string; lore_complete?: string; dialogue?: string }; key: { action: "keep" | "remove" } | { action: "replace"; value: string } };

export const aiModelActions: readonly { action: ModelAction | "lore_complete" | "dialogue"; field: ModelField; labelKey: string }[] = [
  { action: "generate", field: "generate_model", labelKey: "settings.ai.continueModel" },
  { action: "edit", field: "edit_model", labelKey: "settings.ai.editModel" },
  { action: "lore_complete", field: "lore_complete_model", labelKey: "settings.ai.loreCompleteModel" },
  { action: "dialogue", field: "dialogue_model", labelKey: "settings.ai.dialogueModel" },
  { action: "image", field: "image_model", labelKey: "settings.ai.imageModel" },
];

export function aiKeyDecision(source: Source, form: AiSettingsForm["key"]): { showRemove: boolean; note: string; keyToSend: string | undefined } {
  const showRemove = source === "saved";
  const note = source === "env" ? "Set in quillai.env. Edit that file to change it." : source === "none" ? "No API key is set." : form.action === "remove" ? "The saved key will be removed on save. The quillai.env key, if present, will be used instead." : "";
  const keyToSend = form.action === "replace" ? form.value.trim() || undefined : form.action === "remove" && showRemove ? "" : undefined;
  return { showRemove, note, keyToSend };
}

export function aiSettingsUpdate(settings: AiSettings, form: AiSettingsForm): AiSettingsUpdate {
  const base_url = form.base_url.trim();
  const update: AiSettingsUpdate = {};
  if (base_url !== (settings.base_url ?? "").trim()) update.base_url = base_url;
  for (const { action, field } of aiModelActions) {
    const model = (form.models[action] ?? "").trim();
    if (model !== (settings[field] ?? "").trim()) update[field] = model;
  }
  const { keyToSend } = aiKeyDecision(settings.sources.api_key, form.key);
  if (keyToSend !== undefined) update.api_key = keyToSend;
  return update;
}

function parseAiSettings(value: unknown): AiSettings {
  if (typeof value !== "object" || value === null || !("sources" in value) || typeof value.sources !== "object" || value.sources === null || !("base_url" in value) || !("generate_model" in value) || !("edit_model" in value) || !("key_set" in value) || (value.base_url !== null && typeof value.base_url !== "string") || (value.generate_model !== null && typeof value.generate_model !== "string") || (value.edit_model !== null && typeof value.edit_model !== "string") || typeof value.key_set !== "boolean") throw new Error("Expected AI settings");
  const sources = value.sources as Record<string, unknown>;
  if (![sources.base_url, sources.api_key, sources.generate_model, sources.edit_model].every(source => source === "saved" || source === "env" || source === "none")) throw new Error("Expected AI settings sources");
  const image = "image_model" in value ? { image_model: typeof value.image_model === "string" ? value.image_model : null } : {};
  const imageSource = sources.image_model === "saved" || sources.image_model === "env" || sources.image_model === "none" ? { image_model: sources.image_model as Source } : {};
  const lore = "lore_complete_model" in value ? { lore_complete_model: typeof value.lore_complete_model === "string" ? value.lore_complete_model : null } : {};
  const loreSource = sources.lore_complete_model === "saved" || sources.lore_complete_model === "env" || sources.lore_complete_model === "none" ? { lore_complete_model: sources.lore_complete_model as Source } : {};
  const dialogue = "dialogue_model" in value ? { dialogue_model: typeof value.dialogue_model === "string" ? value.dialogue_model : null } : {};
  const dialogueSource = sources.dialogue_model === "saved" || sources.dialogue_model === "env" || sources.dialogue_model === "none" ? { dialogue_model: sources.dialogue_model as Source } : {};
  return { base_url: value.base_url, generate_model: value.generate_model, edit_model: value.edit_model, ...image, ...lore, ...dialogue, key_set: value.key_set, sources: { base_url: sources.base_url as Source, api_key: sources.api_key as Source, generate_model: sources.generate_model as Source, edit_model: sources.edit_model as Source, ...imageSource, ...loreSource, ...dialogueSource } };
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
    async getModelCatalog(): Promise<ModelCatalog | null> {
      try {
        const response = await fetchRequest(`${baseUrl.replace(/\/+$/, "")}/ai/models`, { headers: { Accept: "application/json" } });
        return response.ok ? parseModelCatalog(await response.json()) : null;
      } catch {
        return null;
      }
    },
  };
}
