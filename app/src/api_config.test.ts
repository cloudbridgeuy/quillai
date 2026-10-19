import { expect, test } from "bun:test";
import {
  DEFAULT_API_BASE_URL, getApiBaseUrl, parseApiBaseUrl, resolveApiBaseUrl, type ApiBaseUrlRejection,
} from "./api_config";
import { buildAiGenerateUrl } from "./md_editor/ai_contract";
import { createManuscriptClient } from "./manuscripts/client";

test("API base URL defaults to the local API", () => {
  expect(resolveApiBaseUrl()).toBe(DEFAULT_API_BASE_URL);
  expect(resolveApiBaseUrl("  ")).toBe(DEFAULT_API_BASE_URL);
  expect(parseApiBaseUrl(null)).toEqual({ kind: "default", url: DEFAULT_API_BASE_URL });
});

test("API base URL accepts loopback hosts on any port and removes trailing slashes", () => {
  expect(resolveApiBaseUrl(" http://127.0.0.1:49152/// ")).toBe("http://127.0.0.1:49152");
  expect(resolveApiBaseUrl("https://localhost:8443/api/")).toBe("https://localhost:8443/api");
  expect(resolveApiBaseUrl("http://[::1]:3000")).toBe("http://[::1]:3000");
  expect(resolveApiBaseUrl("http://LOCALHOST:2909")).toBe("http://localhost:2909");
  expect(parseApiBaseUrl("http://127.0.0.1:49152")).toEqual({ kind: "override", url: "http://127.0.0.1:49152" });
});

test("API base URL falls back to the default for malformed values without throwing", () => {
  for (const value of ["/api", "not a URL", "http://", "http://[::1", "http://127.0.0.1:99999", "%%%", "://127.0.0.1"]) {
    expect(() => resolveApiBaseUrl(value)).not.toThrow();
    expect(resolveApiBaseUrl(value)).toBe(DEFAULT_API_BASE_URL);
    expect(parseApiBaseUrl(value)).toEqual({ kind: "rejected", url: DEFAULT_API_BASE_URL, reason: "malformed" });
  }
});

test("API base URL rejects non-loopback hosts and unsafe configuration", () => {
  const cases: [string, ApiBaseUrlRejection][] = [
    ["https://evil.example", "host"],
    ["http://127.0.0.1.evil.example:2908", "host"],
    ["http://localhost.evil.example", "host"],
    ["http://evil.example/127.0.0.1", "host"],
    ["http://10.0.0.5:2908", "host"],
    ["http://0.0.0.0:2908", "host"],
    ["http://127.0.0.1@evil.example", "credentials"],
    ["https://user:pass@127.0.0.1", "credentials"],
    ["file:///api", "scheme"],
    ["javascript:alert(1)", "scheme"],
    ["ws://127.0.0.1:2908", "scheme"],
    ["ftp://localhost", "scheme"],
    ["http://127.0.0.1:2908?x=1", "query"],
    ["http://127.0.0.1:2908#api", "query"],
  ];
  for (const [value, reason] of cases) {
    expect(parseApiBaseUrl(value)).toEqual({ kind: "rejected", url: DEFAULT_API_BASE_URL, reason });
    expect(resolveApiBaseUrl(value)).toBe(DEFAULT_API_BASE_URL);
  }
});

test("browser configuration falls back to the default for rejected query values", () => {
  const previous = Object.getOwnPropertyDescriptor(globalThis, "window");
  try {
    for (const search of ["?apiBaseUrl=", "?apiBaseUrl=not%20a%20url", "?apiBaseUrl=https%3A%2F%2Fevil.example"]) {
      Object.defineProperty(globalThis, "window", { configurable: true, value: { location: { search } } });
      expect(getApiBaseUrl()).toBe(DEFAULT_API_BASE_URL);
    }
  } finally {
    if (previous) Object.defineProperty(globalThis, "window", previous);
    else Reflect.deleteProperty(globalThis, "window");
  }
});

test("AI and manuscript clients use the same browser configuration", async () => {
  const previous = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { location: { search: "?apiBaseUrl=http%3A%2F%2F127.0.0.1%3A49152%2F" } },
  });
  try {
    expect(buildAiGenerateUrl()).toBe("http://127.0.0.1:49152/ai/generate");
    const client = createManuscriptClient({ fetch: async (url) => {
      expect(url).toBe("http://127.0.0.1:49152/series");
      return Response.json([]);
    } });
    expect(await client.listSeries()).toEqual([]);
  } finally {
    if (previous) Object.defineProperty(globalThis, "window", previous);
    else Reflect.deleteProperty(globalThis, "window");
  }
});
