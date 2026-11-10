import { afterEach, expect, test } from "bun:test";
import Quill from "quill";
import { Delta } from "quill/core";
import { act } from "react";
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

test("drawing hints, opening and closing the tip never change the document, storage or requests", async () => {
  const quill = new Quill(document.body.appendChild(document.createElement("div")));
  quill.setContents(new Delta().insert("a cat sat\n"), "silent");
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
  const fetched: string[] = [];
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async (input: RequestInfo | URL) => { fetched.push(String(input)); return Response.json({}); }) as typeof fetch;
  const engine = createEngine([catRule]);
  const client = { getSeriesRules: async () => ({ language: "en" as const, rules: {} }) };
  function Layer() {
    const { hints } = useHintRunner({ quill, client, seriesId: 1, ready: true, aiActive: false, engine, timers });
    return <><HintOverlay quill={quill} hints={hints} /><HintTip quill={quill} hints={hints} rules={[catRule]} timers={timers} /></>;
  }
  const changes: string[] = [];
  quill.on(Quill.events.TEXT_CHANGE, (_delta: unknown, _old: unknown, source: string) => changes.push(source));
  const before = { delta: JSON.stringify(quill.getContents()), html: quill.root.innerHTML, local: JSON.stringify({ ...localStorage }), session: JSON.stringify({ ...sessionStorage }) };
  try {
    await render(<Layer />);
    await advance(HINT_DEBOUNCE_MS);
    expect(quill.container.querySelectorAll("[data-hint-underline]")).toHaveLength(1);
    await act(async () => { quill.root.dispatchEvent(new MouseEvent("pointermove", { bubbles: true, clientX: 30, clientY: 20 })); });
    await advance(HINT_DEBOUNCE_MS + HINT_TIP_DELAY_MS);
    expect(document.body.querySelector("[role=tooltip]")).not.toBeNull();
    await act(async () => { document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); });
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
