import { afterEach, expect, test } from "bun:test";
import Quill from "quill";
import { Delta } from "quill/core";
import { act, useState } from "react";
import type { SeriesRules } from "../../settings/client";
import { cleanup, render } from "../../test/dom";
import { createEngine, type Hint, type Rule } from "./engine";
import { HINT_DEBOUNCE_MS } from "./hint_runner";
import { useHintRunner, type UseHintRunnerOptions } from "./use_hint_runner";

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

async function mount(overrides: Partial<UseHintRunnerOptions> = {}, series: SeriesRules = { language: "en", rules: {} }) {
  const quill = new Quill(document.body.appendChild(document.createElement("div")));
  quill.setContents(new Delta().insert("a cat\n"), "silent");
  let now = 0;
  const pending = new Map<number, { at: number; run: () => void }>();
  let nextId = 1;
  const timers = {
    set: (run: () => void, ms: number) => { const id = nextId++; pending.set(id, { at: now + ms, run }); return id; },
    clear: (handle: unknown) => { pending.delete(handle as number); },
  };
  const advance = async (ms: number) => {
    now += ms;
    await act(async () => { for (const [id, timer] of [...pending]) if (timer.at <= now) { pending.delete(id); timer.run(); } });
  };
  const fetched: number[] = [];
  const client = { getSeriesRules: async (id: number) => { fetched.push(id); return series; } };
  const seen: { hints: Hint[]; setConfig: (config: SeriesRules | null) => void; recheck: () => void }[] = [];
  let update: (next: UseHintRunnerOptions) => void = () => {};
  const base: UseHintRunnerOptions = { quill, client, seriesId: 7, ready: true, aiActive: false, engine: createEngine([catRule]), timers, ...overrides };
  function Probe() {
    const [props, setProps] = useState(base);
    update = setProps;
    seen.push(useHintRunner(props));
    return null;
  }
  await render(<Probe />);
  const view = (next: Partial<UseHintRunnerOptions>) => act(async () => update({ ...base, ...next }));
  return { quill, advance, fetched, seen, hints: () => seen.at(-1)!.hints, view };
}

const typeUser = (quill: Quill, text: string) => act(async () => { quill.updateContents(new Delta().retain(quill.getLength() - 1).insert(text), "user"); });

test("loads the series config and shows hints for the current text", async () => {
  const { fetched, hints } = await mount();
  expect(fetched).toEqual([7]);
  expect(hints()).toEqual([{ ruleId: "cat", start: 2, length: 3, messageKey: "hints.adverb" }]);
});

test("a user edit updates hints only after the debounce", async () => {
  const { quill, advance, hints } = await mount();
  await typeUser(quill, " cat");
  await advance(HINT_DEBOUNCE_MS - 1);
  expect(hints()).toHaveLength(1);
  await advance(1);
  expect(hints()).toHaveLength(2);
});

test("an API-source edit does not schedule a recompute", async () => {
  const { quill, advance, hints } = await mount();
  await act(async () => { quill.updateContents(new Delta().retain(quill.getLength() - 1).insert(" cat"), "api"); });
  await advance(HINT_DEBOUNCE_MS * 2);
  expect(hints()).toHaveLength(1);
});

test("no recompute during composition, one pass when it ends", async () => {
  const { quill, advance, hints } = await mount();
  const line = quill.root.querySelector("p")!;
  await act(async () => { line.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true })); });
  await typeUser(quill, " cat");
  await advance(HINT_DEBOUNCE_MS * 2);
  expect(hints()).toHaveLength(1);
  await act(async () => { line.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true })); });
  expect(hints()).toHaveLength(2);
});

test("an AI run pauses recompute and one pass runs when it ends", async () => {
  const { quill, advance, hints, view } = await mount();
  await view({ aiActive: true });
  await typeUser(quill, " cat");
  await advance(HINT_DEBOUNCE_MS * 2);
  expect(hints()).toHaveLength(1);
  await view({ aiActive: false });
  expect(hints()).toHaveLength(2);
});

test("setConfig recomputes at once with the new rules", async () => {
  const { seen, hints } = await mount();
  expect(hints()).toHaveLength(1);
  await act(async () => seen.at(-1)!.setConfig({ language: "en", rules: { cat: { enabled: false } } }));
  expect(hints()).toEqual([]);
});

test("no series id means no hints and no request", async () => {
  const { fetched, hints } = await mount({ seriesId: null });
  expect(fetched).toEqual([]);
  expect(hints()).toEqual([]);
});

test("nothing is computed before the editor is ready", async () => {
  const { hints } = await mount({ ready: false });
  expect(hints()).toEqual([]);
});

test("recheck after a silent load recomputes for the new text", async () => {
  const { quill, seen, hints } = await mount();
  await act(async () => { quill.setContents(new Delta().insert("cat cat cat\n"), "silent"); });
  expect(hints()).toHaveLength(1);
  await act(async () => seen.at(-1)!.recheck());
  expect(hints()).toHaveLength(3);
});
