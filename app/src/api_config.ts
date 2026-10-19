export const DEFAULT_API_BASE_URL = "http://127.0.0.1:2908";

const LOOPBACK_HOSTS = new Set(["127.0.0.1", "localhost", "[::1]"]);

export type ApiBaseUrlRejection = "malformed" | "scheme" | "credentials" | "query" | "host";

export type ApiBaseUrlResult =
  | { kind: "default"; url: string }
  | { kind: "override"; url: string }
  | { kind: "rejected"; url: string; reason: ApiBaseUrlRejection };

function parseUrl(value: string): URL | null {
  try {
    return new URL(value);
  } catch {
    return null;
  }
}

function rejectionFor(url: URL): ApiBaseUrlRejection | null {
  if (url.protocol !== "http:" && url.protocol !== "https:") return "scheme";
  if (url.username || url.password) return "credentials";
  if (url.search || url.hash) return "query";
  if (!LOOPBACK_HOSTS.has(url.hostname)) return "host";
  return null;
}

export function parseApiBaseUrl(override?: string | null): ApiBaseUrlResult {
  const value = typeof override === "string" ? override.trim() : "";
  if (!value) return { kind: "default", url: DEFAULT_API_BASE_URL };
  const url = parseUrl(value);
  if (!url) return { kind: "rejected", url: DEFAULT_API_BASE_URL, reason: "malformed" };
  const reason = rejectionFor(url);
  if (reason) return { kind: "rejected", url: DEFAULT_API_BASE_URL, reason };
  return { kind: "override", url: url.href.replace(/\/+$/, "") };
}

export function resolveApiBaseUrl(override?: string | null): string {
  return parseApiBaseUrl(override).url;
}

function readQueryOverride(): string | null {
  try {
    return typeof window === "undefined" ? null : new URLSearchParams(window.location.search).get("apiBaseUrl");
  } catch {
    return null;
  }
}

export function getApiBaseUrl(): string {
  return resolveApiBaseUrl(readQueryOverride());
}
