import { expect, test } from "bun:test";
import { aiSettingsUpdate, createSettingsClient, SettingsApiError } from "./client";

test("settings client parses server guidance, sends JSON, and preserves structured failures", async () => {
  const calls: { url: string; init?: RequestInit }[] = [];
  const replies = [
    Response.json({ text: "" }), Response.json({ text: "Saved 💡" }),
    Response.json({ error: "Too large", code: "VALIDATION_ERROR" }, { status: 422 }),
    Response.json({ text: 42 }), new Response("bad", { status: 500 }),
  ];
  const client = createSettingsClient({ baseUrl: "http://localhost:1234/", fetch: async (url, init) => {
    calls.push({ url, init });
    return replies.shift()!;
  } });
  expect(await client.getAuthorGuidance()).toEqual({ text: "" });
  expect(await client.putAuthorGuidance({ text: "Saved 💡" })).toEqual({ text: "Saved 💡" });
  expect(calls[1]!.url).toBe("http://localhost:1234/settings/author-guidance");
  expect(calls[1]!.init?.method).toBe("PUT");
  expect(JSON.parse(calls[1]!.init?.body as string)).toEqual({ text: "Saved 💡" });
  await expect(client.putAuthorGuidance({ text: "x" })).rejects.toMatchObject({ status: 422, code: "VALIDATION_ERROR", kind: "http" });
  await expect(client.getAuthorGuidance()).rejects.toBeInstanceOf(SettingsApiError);
  await expect(client.getAuthorGuidance()).rejects.toMatchObject({ status: 500, kind: "invalid_response" });
});

test("AI settings client validates responses and sends partial updates", async () => {
  const settings = { base_url: "https://example.com", model: "model", key_set: true, sources: { base_url: "env", api_key: "saved", model: "saved" } } as const;
  const calls: { url: string; init?: RequestInit }[] = [];
  const replies = [Response.json(settings), Response.json(settings), Response.json({ ...settings, key_set: "yes" }), Response.json({ ...settings, sources: { ...settings.sources, model: "unknown" } }), Response.json({ error: "invalid", code: "VALIDATION_ERROR" }, { status: 422 })];
  const client = createSettingsClient({ baseUrl: "http://localhost:1234/", fetch: async (url, init) => { calls.push({ url, init }); return replies.shift()!; } });
  expect(await client.getAiSettings()).toEqual(settings);
  expect(await client.putAiSettings({ api_key: "" })).toEqual(settings);
  expect(calls[1]!.url).toBe("http://localhost:1234/settings/ai");
  expect(JSON.parse(calls[1]!.init!.body as string)).toEqual({ api_key: "" });
  await expect(client.getAiSettings()).rejects.toMatchObject({ code: "INVALID_RESPONSE" });
  await expect(client.getAiSettings()).rejects.toMatchObject({ code: "INVALID_RESPONSE" });
  await expect(client.putAiSettings({ model: "bad" })).rejects.toMatchObject({ status: 422, code: "VALIDATION_ERROR" });
});

test("AI form sends only changed values and explicit key actions", () => {
  const settings = { base_url: "https://example.com", model: "model", key_set: true, sources: { base_url: "env", api_key: "saved", model: "saved" } } as const;
  expect(aiSettingsUpdate(settings, { base_url: " https://example.com ", model: "model", key: { action: "keep" } })).toEqual({});
  expect(aiSettingsUpdate(settings, { base_url: "", model: " new ", key: { action: "replace", value: " secret " } })).toEqual({ base_url: "", model: "new", api_key: "secret" });
  expect(aiSettingsUpdate(settings, { base_url: "https://example.com", model: "model", key: { action: "remove" } })).toEqual({ api_key: "" });
  expect(aiSettingsUpdate(settings, { base_url: "https://example.com", model: "model", key: { action: "replace", value: "  " } })).toEqual({});
});

test("AI form omits values equal to padded effective settings", () => {
  const settings = { base_url: " https://example.com ", model: " model ", key_set: false, sources: { base_url: "env", api_key: "none", model: "env" } } as const;
  expect(aiSettingsUpdate(settings, { base_url: " https://example.com ", model: " model ", key: { action: "keep" } })).toEqual({});
});

test("AI settings client accepts unset settings and rejects missing or malformed fields", async () => {
  const settings = { base_url: null, model: null, key_set: false, sources: { base_url: "none", api_key: "none", model: "none" } } as const;
  const read = (value: unknown) => createSettingsClient({ fetch: async () => Response.json(value) }).getAiSettings();
  expect(await read(settings)).toEqual(settings);
  for (const value of [{ ...settings, base_url: 3 }, { ...settings, model: false }, { ...settings, sources: null }, { ...settings, sources: {} }, { model: null, key_set: false, sources: settings.sources }, { base_url: null, key_set: false, sources: settings.sources }, { base_url: null, model: null, key_set: false }]) {
    await expect(read(value)).rejects.toMatchObject({ code: "INVALID_RESPONSE" });
  }
});
