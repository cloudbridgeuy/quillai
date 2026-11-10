import { afterEach, expect, test } from "bun:test";
import Quill from "quill";
import { Delta } from "quill/core";
import { act } from "react";
import { buildAiRequest } from "../ai_contract";
import { toContractDeltaDocument } from "../delta_bridge";
import { diffDocuments } from "../document_diff";
import { cleanup, render } from "../../test/dom";
import { createEngine, type Rule } from "./engine";
import { HintOverlay } from "./HintOverlay";
import { HINT_TIP_DELAY_MS, HintTip } from "./HintTip";
import { HINT_DEBOUNCE_MS } from "./hint_runner";
import { useHintRunner } from "./use_hint_runner";

afterEach(cleanup);

const catRule: Rule = {
  id: "cat",
  labelKey: "rules.adverbs.label",
  languages: ["en"],
  defaultEnabled: true,
  params: [],
  defaultScope: "both",
  check: (doc) => doc.words.filter((w) => w.lower === "cat").map((w) => ({ ruleId: "cat", start: w.start, length: w.end - w.start, messageKey: "hints.adverb" })),
};

async function mountLayer(text: string) {
  const quill = new Quill(document.body.appendChild(document.createElement("div")));
  quill.setContents(new Delta().insert(text), "silent");
  quill.getBounds = (index: number, length = 0) => ({ left: index * 10, top: 10, width: length * 10, height: 20, bottom: 30, right: index * 10 + length * 10 });
  let now = 0;
  const pending = new Map<number, { at: number; run: () => void }>();
  let nextId = 1;
  const timers = {
    set: (run: () => void, ms: number) => { const id = nextId++; pending.set(id, { at: now + ms, run }); return id; },
    clear: (handle: unknown) => { pending.delete(handle as number); },
  };
  const advance = (ms: number) => act(async () => {
    now += ms;
    for (const [id, timer] of [...pending]) if (timer.at <= now) { pending.delete(id); timer.run(); }
  });
  const engine = createEngine([catRule]);
  const client = { getSeriesRules: async () => ({ language: "en" as const, rules: {} }) };
  function Layer() {
    const { hints } = useHintRunner({ quill, client, seriesId: 1, ready: true, aiActive: false, engine, timers });
    return <><HintOverlay quill={quill} hints={hints} /><HintTip quill={quill} hints={hints} rules={[catRule]} timers={timers} /></>;
  }
  await render(<Layer />);
  await advance(HINT_DEBOUNCE_MS);
  return { quill, advance };
}

const hover = (quill: Quill) => act(async () => { quill.root.dispatchEvent(new MouseEvent("pointermove", { bubbles: true, clientX: 30, clientY: 20 })); });
const escape = () => act(async () => { document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); });

test("drawing hints, opening and closing the tip never change the document, storage or requests", async () => {
  const fetched: string[] = [];
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async (input: RequestInfo | URL) => { fetched.push(String(input)); return Response.json({}); }) as typeof fetch;
  const changes: string[] = [];
  let quill: Quill | undefined;
  let before: { delta: string; html: string; local: string; session: string } | undefined;
  try {
    const layer = await mountLayer("a cat sat\n");
    quill = layer.quill;
    quill.on(Quill.events.TEXT_CHANGE, (_delta: unknown, _old: unknown, source: string) => changes.push(source));
    before = { delta: JSON.stringify(quill.getContents()), html: quill.root.innerHTML, local: JSON.stringify({ ...localStorage }), session: JSON.stringify({ ...sessionStorage }) };
    expect(quill.container.querySelectorAll("[data-hint-underline]")).toHaveLength(1);
    await hover(quill);
    await layer.advance(HINT_DEBOUNCE_MS + HINT_TIP_DELAY_MS);
    expect(document.body.querySelector("[role=tooltip]")).not.toBeNull();
    await escape();
    expect(document.body.querySelector("[role=tooltip]")).toBeNull();
  } finally {
    globalThis.fetch = originalFetch;
  }
  expect(changes).toEqual([]);
  expect(JSON.stringify(quill.getContents())).toBe(before.delta);
  expect(quill.root.innerHTML).toBe(before.html);
  expect(JSON.stringify({ ...localStorage })).toBe(before.local);
  expect(JSON.stringify({ ...sessionStorage })).toBe(before.session);
  expect(fetched).toEqual([]);
});

test("a Continue and an Edit request built while hints show carry no hint data", async () => {
  const { quill } = await mountLayer("a cat sat\n");
  expect(quill.container.querySelectorAll("[data-hint-underline]")).toHaveLength(1);
  const length = quill.getLength() - 1;
  const slice = (index: number, size: number) => ({ delta: toContractDeltaDocument(quill.getContents(index, size)), text: quill.getText(index, size) });
  const common = { window: { start: 0, end: length }, storyContext: null, instruction: null, maxOutputTokens: 1000 };
  const requests = [
    buildAiRequest({ ...common, action: "generate", document: slice(0, length), selection: null, messages: [], resumePrompt: null }),
    buildAiRequest({ ...common, action: "edit", document: slice(0, length), selection: { ...slice(2, 3), range: { index: 2, length: 3 } } }),
  ].map((request) => JSON.stringify(request));
  for (const body of requests) {
    for (const leaked of ["hint", "ruleId", "rules.adverbs", "Adverbs", "Consider cutting", "underline"]) expect(body).not.toContain(leaked);
  }
});

test("typing while hints show saves only the user's edit", async () => {
  const { quill, advance } = await mountLayer("a cat sat\n");
  const base = quill.getContents();
  await hover(quill);
  await act(async () => { quill.updateContents(new Delta().retain(9).insert(" cat"), "user"); });
  await advance(HINT_DEBOUNCE_MS + HINT_TIP_DELAY_MS);
  expect(quill.container.querySelectorAll("[data-hint-underline]")).toHaveLength(2);
  expect(diffDocuments(base, quill.getContents()).ops).toEqual(new Delta().retain(9).insert(" cat").ops);
});
