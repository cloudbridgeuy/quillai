import { afterEach, expect, test } from "bun:test";
import { act } from "react";
import { cleanup, click, render } from "../../test/dom";
import { useLore } from "../../manuscripts/use_lore";
import { LorePanel } from "./LorePanel";
import { formatMessage } from "../../i18n/messages";

const realFetch = globalThis.fetch;
const points = [{ id: 5, name: "Act", position: 1 }, { id: 6, name: "End", position: 2 }];

let latest: ReturnType<typeof useLore> | null = null;
function Harness() {
  const lore = useLore({ universeId: 2, pieceId: 7 });
  latest = lore;
  return <LorePanel lore={lore} disabled={false} pieceId={7} />;
}

function stub({ failRefetch = false, failPut = false, parentPoint = null as number | null } = {}) {
  const calls: string[] = [];
  let putDone = false;
  globalThis.fetch = (async (url: string, init?: RequestInit) => {
    const { pathname } = new URL(url);
    const method = init?.method ?? "GET";
    calls.push(init?.body ? `${method} ${pathname} ${init.body}` : `${method} ${pathname}`);
    if (pathname === "/universes/2/lore") return Response.json([]);
    if (pathname === "/universes/2/timeline") return Response.json(points);
    if (pathname === "/pieces/7" && method === "PUT") {
      if (failPut) return Response.json({ error: "boom", code: "INTERNAL" }, { status: 500 });
      putDone = true;
      return Response.json({ parentId: null, timelinePointId: JSON.parse(String(init!.body)).timelinePointId });
    }
    if (pathname === "/pieces/7" && failRefetch && putDone) return Response.json({ error: "boom", code: "INTERNAL" }, { status: 500 });
    if (pathname === "/pieces/3") return Response.json({ parentId: null, timelinePointId: parentPoint });
    return Response.json({ parentId: parentPoint === null ? null : 3, timelinePointId: null });
  }) as typeof fetch;
  return calls;
}

const flush = () => act(async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); });
const picker = () => document.querySelector("select") as HTMLSelectElement;
const choose = (value: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(picker(), value);
  picker().dispatchEvent(new Event("change", { bubbles: true }));
});

afterEach(async () => {
  await cleanup();
  globalThis.fetch = realFetch;
  latest = null;
});

test("picker lists the base option first and PUTs the chosen point, then null", async () => {
  const calls = stub();
  await render(<Harness />);
  await flush();
  expect([...picker().options].map(option => option.textContent)).toEqual(["No point (base text)", "Act", "End"]);
  await choose("6");
  await flush();
  expect(calls).toContain('PUT /pieces/7 {"timelinePointId":6}');
  expect(picker().value).toBe("6");
  await choose("");
  await flush();
  expect(calls).toContain('PUT /pieces/7 {"timelinePointId":null}');
  expect(picker().value).toBe("");
});

test("the chosen point sticks when the piece refetch fails", async () => {
  stub({ failRefetch: true });
  await render(<Harness />);
  await flush();
  await choose("5");
  await flush();
  expect(picker().value).toBe("5");
  expect(latest!.piece?.timelinePointId).toBe(5);
});

test("a failed PUT shows an alert and keeps the previous value", async () => {
  stub({ failPut: true });
  await render(<Harness />);
  await flush();
  await choose("5");
  await flush();
  expect(document.querySelector('[role="alert"]')).not.toBeNull();
  expect(picker().value).toBe("");
});

test("the empty option inherits the parent's point until the piece sets its own", async () => {
  stub({ parentPoint: 6 });
  await render(<Harness />);
  await flush();
  expect(picker().options[0]!.textContent).toBe("Inherit: End");
  await choose("5");
  await flush();
  expect(picker().options[0]!.textContent).toBe("No point (base text)");
});

type StateRow = { id: number; name: string; pointId: number; visibleText: string; hiddenNotes: string };

function stubStates({ states = [] as StateRow[], conflict = false, serverError = false, noPoints = false, hold = null as Promise<void> | null } = {}) {
  const calls: string[] = [];
  let current = states;
  globalThis.fetch = (async (url: string, init?: RequestInit) => {
    const { pathname } = new URL(url);
    const method = init?.method ?? "GET";
    calls.push(init?.body ? `${method} ${pathname} ${init.body}` : `${method} ${pathname}`);
    const entry = { id: 4, universe_id: 2, kind: "character", level: "include", pinned: false, name: "Mira", keywords: [], aliases: [],
      visible_text: "base", hidden_notes: "", states: current };
    if (pathname === "/universes/2/lore") return Response.json([entry]);
    if (pathname === "/universes/2/lore/4") return Response.json(entry);
    if (pathname === "/universes/2/timeline") return Response.json(noPoints ? [] : points);
    if (pathname.startsWith("/pieces/")) return Response.json({ parentId: null, timelinePointId: null });
    if (pathname.startsWith("/universes/2/lore/4/states")) {
      if (method !== "GET" && serverError) return Response.json({ error: "boom", code: "INTERNAL_SERVER_ERROR" }, { status: 500 });
      if (method !== "GET" && hold) await hold;
      if (method !== "GET" && conflict) return Response.json({ error: "dup", code: "CONFLICT" }, { status: 409 });
      const body = init?.body ? JSON.parse(String(init.body)) : {};
      if (method === "POST") { current = [...current, { id: 30, visibleText: "", hiddenNotes: "", ...body }]; return Response.json(current.at(-1), { status: 201 }); }
      if (method === "PUT") { current = current.map(row => ({ ...row, ...body })); return Response.json(current[0]); }
      if (method === "DELETE") { current = []; return new Response(null, { status: 204 }); }
    }
    return Response.json([]);
  }) as typeof fetch;
  return calls;
}

const byLabel = (label: string) => [...document.querySelectorAll("label")].find(node => node.firstElementChild?.textContent === label)!.control as HTMLInputElement;
const button = (name: string) => [...document.querySelectorAll("button")].find(node => node.textContent === name || node.getAttribute("aria-label") === name) as HTMLButtonElement;
const typeInto = (label: string, value: string) => act(async () => {
  const field = byLabel(label);
  Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), "value")!.set!.call(field, value);
  field.dispatchEvent(new Event(field instanceof HTMLSelectElement ? "change" : "input", { bubbles: true }));
});
const press = (name: string) => act(async () => button(name).click());
const openEntry = async () => { await render(<Harness />); await flush(); await flush(); await press("Mira · character"); await flush(); };

test("states section adds a state at a point and refreshes the list", async () => {
  const calls = stubStates();
  await openEntry();
  expect(document.body.textContent).toContain("No states yet.");
  await typeInto("State name", "Younger");
  await typeInto("State text", "young text");
  await typeInto("Point", "5");
  await press("Add state");
  await flush();
  expect(calls).toContain('POST /universes/2/lore/4/states {"name":"Younger","pointId":5,"visibleText":"young text","hiddenNotes":""}');
  expect(document.body.textContent).toContain("Younger");
  expect(document.body.textContent).not.toContain("No states yet.");
});

test("states section edits a state with PUT", async () => {
  const calls = stubStates({ states: [{ id: 8, name: "Older", pointId: 6, visibleText: "old", hiddenNotes: "" }] });
  await openEntry();
  await press("Older · End");
  expect(byLabel("State name").value).toBe("Older");
  await typeInto("State name", "Oldest");
  await press("Save state");
  await flush();
  expect(calls).toContain('PUT /universes/2/lore/4/states/8 {"name":"Oldest","pointId":6,"visibleText":"old","hiddenNotes":""}');
});

test("states section deletes with a two-click confirmation", async () => {
  const calls = stubStates({ states: [{ id: 8, name: "Older", pointId: 6, visibleText: "old", hiddenNotes: "" }] });
  await openEntry();
  await press("Older · End");
  await press("Delete state");
  expect(calls.some(call => call.startsWith("DELETE"))).toBe(false);
  await press("Confirm delete");
  await flush();
  expect(calls).toContain("DELETE /universes/2/lore/4/states/8");
  expect(document.body.textContent).toContain("No states yet.");
});

test("a 409 shows the states-specific message", async () => {
  stubStates({ conflict: true });
  await openEntry();
  await typeInto("State name", "Dup");
  await typeInto("Point", "5");
  await press("Add state");
  await flush();
  expect(document.body.textContent).toContain("This entry already has a state at that point.");
  expect(document.body.textContent).not.toContain(reloadText);
});

test("states strings exist in English and Spanish", () => {
  for (const key of ["lore.states.title", "lore.states.empty", "lore.states.conflict", "lore.states.add"] as const) {
    expect(formatMessage("en", key)).not.toBe(key);
    expect(formatMessage("es", key)).not.toBe(formatMessage("en", key));
  }
  expect(formatMessage("es", "lore.states.conflict")).toBe("Esta entrada ya tiene un estado en ese punto.");
});

const reloadText = "That changed somewhere else. Reload and try again.";
const noRefetch = (calls: string[]) => expect(calls).not.toContain("GET /universes/2/lore/4");

test("a saved state is listed locally and the draft resets without a refetch", async () => {
  const calls = stubStates();
  await openEntry();
  await typeInto("State name", "Younger");
  await typeInto("Point", "6");
  await press("Add state");
  await flush();
  noRefetch(calls);
  expect(document.body.textContent).toContain("Younger · End");
  expect(byLabel("State name").value).toBe("");
  expect(byLabel("Point").value).toBe("");
});

test("states are listed by point position", async () => {
  stubStates({ states: [{ id: 8, name: "Later", pointId: 6, visibleText: "", hiddenNotes: "" }] });
  await openEntry();
  await typeInto("State name", "Sooner");
  await typeInto("Point", "5");
  await press("Add state");
  await flush();
  const rows = [...document.querySelectorAll('section[aria-label="States"] li button')].map(node => node.textContent);
  expect(rows).toEqual(["Sooner · Act", "Later · End"]);
});

test("entry Save is disabled while a state request is pending", async () => {
  let release = () => {};
  stubStates({ hold: new Promise<void>(resolve => { release = resolve; }) });
  await openEntry();
  await typeInto("State name", "Younger");
  await typeInto("Point", "5");
  await press("Add state");
  expect(button("Save lore").disabled).toBe(true);
  expect(button("Delete lore").disabled).toBe(true);
  release();
  await flush();
  expect(button("Save lore").disabled).toBe(false);
});

test("a state on a deleted point shows the Deleted point option and label", async () => {
  stubStates({ states: [{ id: 8, name: "Ghost", pointId: 99, visibleText: "", hiddenNotes: "" }] });
  await openEntry();
  expect(document.body.textContent).toContain("Ghost · Deleted point");
  await press("Ghost · Deleted point");
  expect(byLabel("Point").value).toBe("99");
  expect([...(byLabel("Point") as unknown as HTMLSelectElement).options].map(option => option.textContent)).toContain("Deleted point");
  expect(formatMessage("es", "lore.states.deletedPoint")).toBe("Punto eliminado");
});

test("without timeline points the form is replaced by a hint", async () => {
  stubStates({ noPoints: true });
  await openEntry();
  expect(document.body.textContent).toContain("Add timeline points to give this entry states.");
  expect(button("Add state")).toBeUndefined();
});

test("a 500 shows its message and keeps the draft", async () => {
  stubStates({ serverError: true });
  await openEntry();
  await typeInto("State name", "Younger");
  await typeInto("Point", "5");
  await press("Add state");
  await flush();
  expect(document.querySelector('[role="alert"]')).not.toBeNull();
  expect(document.body.textContent).not.toContain("This entry already has a state at that point.");
  expect(byLabel("State name").value).toBe("Younger");
});

test("a 409 on edit shows the states message, not the reload text, and keeps the draft", async () => {
  stubStates({ conflict: true, states: [{ id: 8, name: "Older", pointId: 6, visibleText: "", hiddenNotes: "" }] });
  await openEntry();
  await press("Older · End");
  await typeInto("Point", "5");
  await press("Save state");
  await flush();
  expect(document.body.textContent).toContain("This entry already has a state at that point.");
  expect(document.body.textContent).not.toContain(reloadText);
  expect(byLabel("State name").value).toBe("Older");
  expect(byLabel("Point").value).toBe("5");
});

test("New state resets the draft", async () => {
  stubStates({ states: [{ id: 8, name: "Older", pointId: 6, visibleText: "old", hiddenNotes: "" }] });
  await openEntry();
  await press("Older · End");
  expect(byLabel("State name").value).toBe("Older");
  await press("New state");
  expect(byLabel("State name").value).toBe("");
  expect(byLabel("Point").value).toBe("");
  expect(button("Add state")).toBeDefined();
});

const mara = { id: 11, universe_id: 2, kind: "character", level: "include", pinned: false, name: "Mara", keywords: [], aliases: [], visible_text: "Tall", hidden_notes: "" };
const loreImage = (id: number) => ({ id, loreEntryId: 11, timelinePointId: 5, prompt: "p", model: "img1", size: "1024x1024", quality: "medium", mime: "image/webp", cost: 0.011, createdAt: "2026-11-06 10:00:00" });
const imageSettings = { base_url: null, generate_model: null, edit_model: null, image_model: null, key_set: true,
  sources: { base_url: "none", api_key: "env", generate_model: "none", edit_model: "none", image_model: "none" } };
const imageCatalog = (price: number | null) => ({
  models: [{ id: "m1", label: "M1", input_price_per_million: 0, output_price_per_million: 0 },
    { id: "img1", label: "Image One", kind: "image", size: "1024x1024", quality: "medium", ...(price === null ? {} : { price_per_image: price }) }],
  defaults: { generate: "m1", edit: "m1", image: "img1" },
});

function imageStub({ create = () => Response.json(loreImage(10), { status: 201 }), price = 0.011 as number | null } = {}) {
  const calls: string[] = [];
  globalThis.fetch = (async (url: string, init?: RequestInit) => {
    const { pathname } = new URL(url);
    const method = init?.method ?? "GET";
    calls.push(init?.body ? `${method} ${pathname} ${init.body}` : `${method} ${pathname}`);
    if (pathname === "/universes/2/lore") return Response.json([mara]);
    if (pathname === "/universes/2/timeline") return Response.json(points);
    if (pathname === "/universes/2/lore/11/images") return method === "POST" ? create() : Response.json([loreImage(9)]);
    if (pathname === "/ai/models") return Response.json(imageCatalog(price));
    if (pathname === "/settings/ai") return Response.json(imageSettings);
    return Response.json({ parentId: null, timelinePointId: 5 });
  }) as typeof fetch;
  return calls;
}

async function openMara() {
  await render(<Harness />);
  await flush();
  await click([...document.querySelectorAll("button")].find(button => button.textContent?.includes("Mara"))!);
  await flush();
}
const generateButton = () => [...document.querySelectorAll("button")].find(button => button.textContent === "Generate image");
const thumbnails = () => [...document.querySelectorAll("img")].map(img => img.getAttribute("src") ?? "");

test("Generate image posts the chapter's timeline point and shows thumbnails", async () => {
  const calls = imageStub();
  await openMara();
  expect(thumbnails().map(src => src.replace(/^.*\/universes/, "/universes"))).toEqual(["/universes/2/lore/11/images/9"]);
  await click(generateButton()!);
  await flush();
  expect(calls).toContain('POST /universes/2/lore/11/images {"timelinePointId":5}');
  expect(thumbnails().map(src => src.replace(/^.*\/images\//, ""))).toEqual(["9", "10"]);
});

test("the estimated cost shows the per-image price, or unknown without one", async () => {
  imageStub();
  await openMara();
  expect(document.body.textContent).toContain("Estimated cost: $0.011");
  await cleanup();
  imageStub({ price: null });
  await openMara();
  expect(document.body.textContent).toContain("Estimated cost: unknown");
});

test("a 503 asks for an image model and a 422 hides the button", async () => {
  imageStub({ create: () => Response.json({ error: "AI image model is not configured", code: "AI_NOT_CONFIGURED" }, { status: 503 }) });
  await openMara();
  expect(generateButton()).toBeInstanceOf(HTMLButtonElement);
  await click(generateButton()!);
  await flush();
  expect(document.body.textContent).toContain("Choose an image model in Settings");
  await cleanup();
  imageStub({ create: () => Response.json({ error: "lore entry is set to Leave out", code: "VALIDATION_ERROR" }, { status: 422 }) });
  await openMara();
  await click(generateButton()!);
  await flush();
  expect(generateButton()).toBeUndefined();
});

const ndjson = (...events: object[]) => new Response(events.map(event => `${JSON.stringify(event)}\n`).join(""));
function stubComplete(reply: () => Response, level = "include") {
  const calls = stubStates();
  const states = globalThis.fetch;
  globalThis.fetch = (async (url: string, init?: RequestInit) => {
    const { pathname } = new URL(url);
    if (pathname === "/ai/generate") { calls.push(`POST /ai/generate ${init?.body}`); return reply(); }
    const response = await states(url, init);
    if (pathname !== "/universes/2/lore") return response;
    return Response.json((await response.json()).map((entry: object) => ({ ...entry, level })));
  }) as typeof fetch;
  return calls;
}
const settle = () => act(async () => { await new Promise(resolve => setTimeout(resolve, 10)); });
const visible = () => { const field = byLabel("Visible text (AI context)"); if (!(field instanceof HTMLTextAreaElement)) throw new Error("visible field is not a textarea"); return field; };
const ctrlEnter = (field: HTMLElement) => act(async () => { field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", ctrlKey: true, bubbles: true })); });
const aiCalls = (calls: string[]) => calls.filter(call => call.startsWith("POST /ai/generate"));

test("Complete sends only name, kind and text before the caret, and inserts at the caret", async () => {
  const calls = stubComplete(() => ndjson({ type: "delta", delta: { ops: [{ insert: "More." }] } }, { type: "done", finishReason: "stop" }));
  await openEntry();
  await typeInto("Visible text (AI context)", "Born here. After");
  visible().setSelectionRange(10, 10);
  expect(button("Complete")).toBeInstanceOf(HTMLButtonElement);
  await press("Complete");
  await settle();
  expect(aiCalls(calls).map(call => JSON.parse(call.slice("POST /ai/generate ".length)))).toEqual([
    { action: "lore_complete", loreComplete: { name: "Mira", kind: "character", text: "Born here." } },
  ]);
  expect(visible().value).toBe("Born here. More. After");
});

test("Complete appears once, only for the visible field; other fields ignore Ctrl+Enter", async () => {
  const calls = stubComplete(() => ndjson({ type: "done", finishReason: "stop" }));
  await openEntry();
  expect([...document.querySelectorAll("button")].filter(node => node.getAttribute("aria-label") === "Complete")).toHaveLength(1);
  for (const label of ["Hidden notes — author-only, never sent to AI", "Keywords (one per line)", "Aliases (one per line)"]) await ctrlEnter(byLabel(label));
  await settle();
  expect(aiCalls(calls)).toEqual([]);
});

test("a leave_out entry hides Complete and ignores Ctrl+Enter until its level changes", async () => {
  const calls = stubComplete(() => ndjson({ type: "done", finishReason: "stop" }), "leave_out");
  await render(<Harness />); await flush(); await flush(); await press("Mira · character"); await flush();
  expect(button("Complete")).toBeUndefined();
  await ctrlEnter(visible());
  await settle();
  expect(aiCalls(calls)).toEqual([]);
  await typeInto("In AI requests", "include");
  expect(button("Complete")).toBeInstanceOf(HTMLButtonElement);
});

test("a failed completion removes only the AI text and shows the error under the field", async () => {
  stubComplete(() => ndjson({ type: "delta", delta: { ops: [{ insert: "Partial" }] } }, { type: "error", error: "provider failed" }));
  await openEntry();
  expect(button("Complete")).toBeInstanceOf(HTMLButtonElement);
  await press("Complete");
  await settle();
  expect(visible().value).toBe("base");
  expect(document.body.textContent).toContain("provider failed");
  await cleanup();
  stubComplete(() => Response.json({ error: "x", code: "AI_NOT_CONFIGURED" }, { status: 503 }));
  await openEntry();
  expect(button("Complete")).toBeInstanceOf(HTMLButtonElement);
  await press("Complete");
  await settle();
  expect(visible().value).toBe("base");
  expect([...document.querySelectorAll('[role="alert"]')].map(node => node.textContent)).toContain(formatMessage("en", "error.aiNotConfigured"));
});
