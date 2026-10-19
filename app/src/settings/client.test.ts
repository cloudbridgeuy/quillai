import { expect, test } from "bun:test";
import { createSettingsClient, SettingsApiError } from "./client";

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
