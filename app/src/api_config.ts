export const DEFAULT_API_BASE_URL = "http://127.0.0.1:2908";

export function resolveApiBaseUrl(override?: string | null): string {
  const value = override?.trim() || DEFAULT_API_BASE_URL;
  const url = new URL(value);
  if (!["http:", "https:"].includes(url.protocol) || url.username || url.password || url.search || url.hash) {
    throw new Error("API base URL must be an HTTP(S) URL without credentials, query, or fragment");
  }
  return url.href.replace(/\/+$/, "");
}

export function getApiBaseUrl(): string {
  return resolveApiBaseUrl(
    typeof window === "undefined" ? undefined : new URLSearchParams(window.location.search).get("apiBaseUrl"),
  );
}
