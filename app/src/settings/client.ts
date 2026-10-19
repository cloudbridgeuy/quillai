import { getApiBaseUrl } from "../api_config";
import { parseApiError } from "../manuscripts/contract";

export type AuthorGuidance = { text: string };

export class SettingsApiError extends Error {
  constructor(message: string, readonly status: number, readonly code: string, readonly kind: "http" | "invalid_response") {
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
  async function request(method: "GET" | "PUT", input?: AuthorGuidance): Promise<AuthorGuidance> {
    const response = await fetchRequest(`${baseUrl.replace(/\/+$/, "")}/settings/author-guidance`, {
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
      throw new SettingsApiError(error.error, response.status, error.code, "http");
    }
    try {
      if (response.status !== 200) throw new Error("Expected HTTP 200");
      return parseAuthorGuidance(await response.json());
    } catch (error) {
      throw new SettingsApiError(error instanceof Error ? error.message : "Invalid response", response.status, "INVALID_RESPONSE", "invalid_response");
    }
  }
  return {
    getAuthorGuidance: () => request("GET"),
    putAuthorGuidance: (input: AuthorGuidance) => request("PUT", input),
  };
}
