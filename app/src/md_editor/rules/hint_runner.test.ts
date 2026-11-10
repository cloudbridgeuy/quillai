import { expect, test } from "bun:test";
import type { SeriesRules } from "../../settings/client";
import { createEngine, type Hint, type Rule } from "./engine";
import { createHintRunner, HINT_DEBOUNCE_MS, type Gate } from "./hint_runner";

const catRule: Rule = {
  id: "cat",
  labelKey: "rules.adverbs.label",
  languages: ["en"],
  defaultEnabled: true,
  params: [],
  defaultScope: "both",
  check: (doc) => doc.words.filter((w) => w.lower === "cat").map((w) => ({ ruleId: "cat", start: w.start, length: w.end - w.start, messageKey: "hints.adverb" })),
};

function setup(initial: Partial<Gate> = { ready: true }) {
  let now = 0;
  const pending = new Map<number, { at: number; run: () => void }>();
  let next = 1;
  let text = "a cat";
  const emitted: Hint[][] = [];
  const runs: string[] = [];
  const engine = createEngine([catRule]);
  const runner = createHintRunner({
    engine: { run: (value, language, stored) => { runs.push(`${language}:${value}`); return engine.run(value, language, stored); } },
    getText: () => text,
    onHints: (hints) => emitted.push(hints),
    timers: {
      set: (run, ms) => { const id = next++; pending.set(id, { at: now + ms, run }); return id; },
      clear: (handle) => { pending.delete(handle as number); },
    },
  });
  const advance = (ms: number) => {
    now += ms;
    for (const [id, timer] of [...pending]) if (timer.at <= now) { pending.delete(id); timer.run(); }
  };
  const config: SeriesRules = { language: "en", rules: {} };
  runner.setGate({ ready: false, aiActive: false, composing: false, ...initial });
  return { runner, advance, emitted, runs, config, setText: (value: string) => { text = value; }, pending: () => pending.size };
}

test("a user text change recomputes once after the 400 ms debounce", () => {
  const { runner, advance, emitted, config } = setup();
  runner.setConfig(config);
  emitted.length = 0;
  runner.textChanged("user");
  advance(HINT_DEBOUNCE_MS - 1);
  expect(emitted).toHaveLength(0);
  advance(1);
  expect(emitted).toHaveLength(1);
  expect(emitted[0]).toEqual([{ ruleId: "cat", start: 2, length: 3, messageKey: "hints.adverb" }]);
});

test("each change restarts the debounce", () => {
  const { runner, advance, emitted, config, setText } = setup();
  runner.setConfig(config);
  emitted.length = 0;
  runner.textChanged("user");
  advance(300);
  setText("a cat cat");
  runner.textChanged("user");
  advance(300);
  expect(emitted).toHaveLength(0);
  advance(100);
  expect(emitted).toHaveLength(1);
  expect(emitted[0]).toHaveLength(2);
});

test("only user-source changes schedule a recompute", () => {
  const { runner, advance, emitted, config, pending } = setup();
  runner.setConfig(config);
  emitted.length = 0;
  runner.textChanged("api");
  runner.textChanged("silent");
  expect(pending()).toBe(0);
  advance(1000);
  expect(emitted).toHaveLength(0);
});

test("a config change recomputes at once, with the new language and rules", () => {
  const { runner, runs, emitted, config } = setup();
  runner.setConfig(config);
  expect(runs).toEqual(["en:a cat"]);
  runner.setConfig({ language: "es", rules: {} });
  expect(runs).toEqual(["en:a cat", "es:a cat"]);
  expect(emitted.at(-1)).toEqual([]);
});

test("a recompute on config disables a rule through the stored rules", () => {
  const { runner, emitted } = setup();
  runner.setConfig({ language: "en", rules: { cat: { enabled: false } } });
  expect(emitted.at(-1)).toEqual([]);
});

test("nothing runs before the editor is ready, then one pass when it becomes ready", () => {
  const { runner, advance, runs, config } = setup({ ready: false });
  runner.setConfig(config);
  runner.textChanged("user");
  advance(1000);
  expect(runs).toEqual([]);
  runner.setGate({ ready: true });
  expect(runs).toEqual(["en:a cat"]);
});

test("no recompute while the AI is active, one pass when it ends", () => {
  const { runner, advance, runs, config } = setup();
  runner.setConfig(config);
  runs.length = 0;
  runner.setGate({ aiActive: true });
  runner.textChanged("user");
  runner.textChanged("user");
  advance(1000);
  runner.setConfig({ language: "en", rules: {} });
  expect(runs).toEqual([]);
  runner.setGate({ aiActive: false });
  expect(runs).toEqual(["en:a cat"]);
  runner.setGate({ aiActive: false });
  expect(runs).toEqual(["en:a cat"]);
});

test("no recompute during composition, one pass when it ends", () => {
  const { runner, advance, runs, config } = setup();
  runner.setConfig(config);
  runs.length = 0;
  runner.setGate({ composing: true });
  runner.textChanged("user");
  advance(1000);
  expect(runs).toEqual([]);
  runner.setGate({ composing: false });
  expect(runs).toEqual(["en:a cat"]);
});

test("no pass when a gate opens and nothing changed", () => {
  const { runner, runs, config } = setup();
  runner.setConfig(config);
  runs.length = 0;
  runner.setGate({ aiActive: true });
  runner.setGate({ aiActive: false });
  expect(runs).toEqual([]);
});

test("no series config means no hints", () => {
  const { runner, emitted, runs } = setup();
  runner.setConfig(null);
  expect(runs).toEqual([]);
  expect(emitted.at(-1)).toEqual([]);
});

test("dispose cancels a pending recompute", () => {
  const { runner, advance, runs, config } = setup();
  runner.setConfig(config);
  runs.length = 0;
  runner.textChanged("user");
  runner.dispose();
  advance(1000);
  expect(runs).toEqual([]);
});
