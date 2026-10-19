import { expect, test } from "bun:test";
import { DEFAULT_API_BASE_URL, resolveApiBaseUrl } from "./api_config";
import { buildAiGenerateUrl } from "./md_editor/ai_contract";
import { createManuscriptClient } from "./manuscripts/client";

test("API base URL defaults to the local API", () => {
  expect(resolveApiBaseUrl()).toBe(DEFAULT_API_BASE_URL);
  expect(resolveApiBaseUrl("  ")).toBe(DEFAULT_API_BASE_URL);
});

test("API base URL accepts a random port and removes trailing slashes", () => {
  expect(resolveApiBaseUrl(" http://127.0.0.1:49152/// ")).toBe("http://127.0.0.1:49152");
  expect(resolveApiBaseUrl("https://example.com/api/")).toBe("https://example.com/api");
});

test("API base URL rejects invalid or unsafe configuration", () => {
  for (const value of ["/api", "not a URL", "file:///api", "https://user:pass@example.com", "https://example.com?x=1", "https://example.com#api"]) {
    expect(() => resolveApiBaseUrl(value)).toThrow();
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
