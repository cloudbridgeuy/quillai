import { afterEach, expect, test } from "bun:test";
import { cleanup, click, render, type } from "../test/dom";
import { AiSettings } from "./AiSettings";
import { createSettingsClient } from "./client";

afterEach(cleanup);

test.each([false, true])("Remove reflects a saved key (%s)", async (keySet) => {
  const settings = { base_url: null, generate_model: null, edit_model: null, key_set: keySet, sources: { base_url: "none", generate_model: "none", edit_model: "none", api_key: keySet ? "saved" : "none" } };
  const updates: unknown[] = [];
  const client = createSettingsClient({ fetch: async (_url, init) => {
    if (init?.method === "PUT") updates.push(JSON.parse(String(init.body)));
    return Response.json(settings);
  } });
  let saved = 0;
  const container = await render(<AiSettings client={client} onSaved={() => saved++} />);
  expect([...container.querySelectorAll("button")].some(button => button.textContent === "Remove")).toBe(keySet);
  await type(container.querySelector("input")!, "https://example.com");
  await click([...container.querySelectorAll("button")].find(button => button.textContent === "Save AI settings")!);
  expect(updates).toEqual([{ base_url: "https://example.com" }]);
  expect(saved).toBe(1);
});
