import { afterEach, expect, test } from "bun:test";
import { act } from "react";
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

const base = { base_url: null, generate_model: null as string | null, edit_model: null as string | null, key_set: true, sources: { base_url: "none", api_key: "env", generate_model: "none", edit_model: "none" } };
const model = (id: string, label = id) => ({ id, label, input_price_per_million: 0, output_price_per_million: 0 });
const catalog = { models: [model("m1", "Model One"), model("m2", "Model Two")], defaults: { generate: "m1", edit: "m2" } };

async function setup(settings: typeof base, models: Response | (() => Response) = Response.json(catalog)) {
  const updates: Record<string, string>[] = [];
  const client = createSettingsClient({ baseUrl: "http://api.test", fetch: async (url, init) => {
    if (url.endsWith("/ai/models")) return typeof models === "function" ? models() : models.clone();
    if (init?.method === "PUT") updates.push(JSON.parse(String(init.body)));
    return Response.json(settings);
  } });
  const container = await render(<AiSettings client={client} onSaved={() => {}} />);
  const select = (label: string) => [...container.querySelectorAll("label")].find(el => el.querySelector("span")?.textContent === label)?.querySelector("select") as HTMLSelectElement | undefined;
  const choose = (label: string, value: string) => act(async () => {
    const el = select(label)!;
    el.value = value;
    el.dispatchEvent(new Event("change", { bubbles: true }));
  });
  const save = () => click([...container.querySelectorAll("button")].find(button => button.textContent === "Save AI settings")!);
  return { container, updates, select, choose, save };
}

test("one model menu per AI action with a default option", async () => {
  const { select } = await setup(base);
  expect([...select("Continue model")!.options].map(o => `${o.value}|${o.textContent}`)).toEqual(["|Default (m1)", "m1|Model One", "m2|Model Two"]);
  expect([...select("Edit model")!.options][0]!.textContent).toBe("Default (m2)");
});

test("changing only Edit sends only edit_model", async () => {
  const { choose, save, updates } = await setup(base);
  await choose("Edit model", "m1");
  await save();
  expect(updates).toEqual([{ edit_model: "m1" }]);
});

test("choosing Default clears the saved model with an empty string", async () => {
  const { choose, save, updates, select } = await setup({ ...base, generate_model: "m2", sources: { ...base.sources, generate_model: "saved" } });
  expect(select("Continue model")!.value).toBe("m2");
  await choose("Continue model", "");
  await save();
  expect(updates).toEqual([{ generate_model: "" }]);
});

test("a saved model outside the catalog stays visible and selected", async () => {
  const { select, container } = await setup({ ...base, edit_model: "old-x", sources: { ...base.sources, edit_model: "saved" } });
  const el = select("Edit model")!;
  expect(el.value).toBe("old-x");
  expect([...el.options].some(o => o.textContent === "old-x (not in catalog)")).toBe(true);
  expect(container.textContent).toContain("Saved here");
});

test("catalog failure falls back to one text field per action", async () => {
  const { container } = await setup(base, Response.json({ error: "x", code: "AI_NOT_CONFIGURED" }, { status: 503 }));
  expect(container.querySelectorAll("select").length).toBe(0);
  expect(container.querySelectorAll("input").length).toBe(3);
  expect(container.textContent).toContain("Continue model");
  expect(container.textContent).toContain("Edit model");
});

test("Default reads plain when a model is saved here", async () => {
  const { select } = await setup({ ...base, generate_model: "m2", sources: { ...base.sources, generate_model: "saved" } });
  expect([...select("Continue model")!.options][0]!.textContent).toBe("Default");
  expect([...select("Edit model")!.options][0]!.textContent).toBe("Default (m2)");
});

test("the catalog is fetched again after a save", async () => {
  let fetches = 0;
  const { choose, save, select } = await setup(base, () => (++fetches === 1 ? Response.json(catalog) : Response.json({ ...catalog, models: [...catalog.models, model("m3", "Model Three")] })));
  await choose("Edit model", "m1");
  await save();
  expect(fetches).toBe(2);
  expect([...select("Edit model")!.options].map(o => o.value)).toContain("m3");
});

test.each([["generate", "Continue model"], ["edit", "Edit model"]] as const)("focusAction %s focuses the %s menu once loaded", async (focusAction, label) => {
  const client = createSettingsClient({ baseUrl: "http://api.test", fetch: async (url) => url.endsWith("/ai/models") ? Response.json(catalog) : Response.json(base) });
  const container = await render(<AiSettings client={client} onSaved={() => {}} focusAction={focusAction} />);
  const select = [...container.querySelectorAll("label")].find(el => el.querySelector("span")?.textContent === label)!.querySelector("select");
  expect(document.activeElement).toBe(select!);
});

const imageModel = { id: "img1", label: "Image One", kind: "image", price_per_image: 0.011, size: "1024x1024", quality: "medium" };
const imageCatalog = { models: [...catalog.models, imageModel], defaults: { ...catalog.defaults, image: "img1" } };

test("the Image menu lists only image models and saves image_model", async () => {
  const { select, choose, save, updates } = await setup(base, Response.json(imageCatalog));
  expect(select("Image model")).toBeInstanceOf(HTMLSelectElement);
  expect([...select("Image model")!.options].map(o => o.value)).toEqual(["", "img1"]);
  expect(select("Image model")!.options[0]!.textContent).toStartWith("Default");
  expect([...select("Continue model")!.options].map(o => o.value)).not.toContain("img1");
  await choose("Image model", "img1");
  await save();
  expect(updates).toEqual([{ image_model: "img1" }]);
});

test("the Image menu is hidden when the catalog has no image models", async () => {
  const { select, container } = await setup(base);
  expect(select("Continue model")).toBeDefined();
  expect(select("Image model")).toBeUndefined();
  expect(container.textContent).not.toContain("Image model");
});

test("without focusAction nothing in AiSettings takes focus", async () => {
  const { container } = await setup(base);
  expect(container.contains(document.activeElement)).toBe(false);
});

test("the Lore completion menu lists text models, shows Default and saves lore_complete_model", async () => {
  const { select, choose, save, updates } = await setup(base, Response.json(imageCatalog));
  expect(select("Lore completion model")).toBeInstanceOf(HTMLSelectElement);
  expect([...select("Lore completion model")!.options].map(o => `${o.value}|${o.textContent}`)).toEqual(["|Default", "m1|Model One", "m2|Model Two"]);
  await choose("Lore completion model", "m2");
  await save();
  expect(updates).toEqual([{ lore_complete_model: "m2" }]);
});

test("the Dialogue menu lists text models, shows Default and saves dialogue_model", async () => {
  const { select, choose, save, updates } = await setup(base, Response.json(imageCatalog));
  expect(select("Dialogue model")).toBeInstanceOf(HTMLSelectElement);
  expect([...select("Dialogue model")!.options].map(o => `${o.value}|${o.textContent}`)).toEqual(["|Default", "m1|Model One", "m2|Model Two"]);
  await choose("Dialogue model", "m2");
  await save();
  expect(updates).toEqual([{ dialogue_model: "m2" }]);
});

test("a saved dialogue_model loads into the Dialogue menu and Default clears it", async () => {
  const savedDialogue = { ...base, dialogue_model: "m1", sources: { ...base.sources, dialogue_model: "saved" } };
  const { select, choose, save, updates, container } = await setup(savedDialogue);
  expect(select("Dialogue model")?.value).toBe("m1");
  expect(container.querySelector("[data-model-action='dialogue'] .editor-note")?.textContent).toBe("Saved here");
  await choose("Dialogue model", "");
  await save();
  expect(updates).toEqual([{ dialogue_model: "" }]);
});
