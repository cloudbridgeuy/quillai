import { expect, test } from "bun:test";
import { beginInspectorRun, buildInspectorCost, formatInspectorUsd, recordInspectorUsage } from "./inspector_cost";
import type { ModelCatalog } from "./model_catalog";

const catalog: ModelCatalog = { models: [{ id: "fast", label: "Fast", input_price_per_million: 2, output_price_per_million: 6 }], defaults: { generate: "fast", edit: "fast" } };
const usage = { type: "usage" as const, promptTokens: 1000, completionTokens: 500 };

test("pending estimate is input only; output is a rate, not estimated usage", () => {
  const view = buildInspectorCost(catalog, "fast", 1000, null);
  expect(view.model).toBe("Fast");
  expect(view.estimate).toBe("≈ $0.002 · input only");
  expect(view.outputRate).toBe("$6 / million output tokens");
  expect(view.actual).toBe("unknown — no run yet");
});

test("request model appears only when model IDs differ", () => {
  const models = { ...catalog, models: [...catalog.models, { ...catalog.models[0]!, id: "other" }] };
  const run = beginInspectorRun(1, models, "fast");
  expect(buildInspectorCost(models, "fast", 100, null).actualModel).toBeNull();
  expect(buildInspectorCost(models, "fast", 100, run).actualModel).toBeNull();
  expect(buildInspectorCost(models, "other", 100, run).actualModel).toBe("Fast");
});

test("actual uses last usage and request pricing snapshot, independent of pending estimate", () => {
  const original = structuredClone(catalog);
  const run = recordInspectorUsage(recordInspectorUsage(beginInspectorRun(1, original, "fast"), 1, usage), 1, { ...usage, promptTokens: 2000 });
  original.models[0]!.input_price_per_million = 50;
  const view = buildInspectorCost(original, "fast", 9000, run);
  expect(view.actual).toBe("$0.007");
  expect(view.usage).toBe("2000 input · 500 output tokens");
  expect(view.estimate).toBe("≈ $0.45 · input only");
});

test("new run clears actual; stale usage cannot land on newer run or other action", () => {
  const old = recordInspectorUsage(beginInspectorRun(1, catalog, "fast"), 1, usage);
  const next = beginInspectorRun(2, catalog, "fast");
  expect(recordInspectorUsage(next, 1, usage)).toBe(next);
  expect(recordInspectorUsage(old, 2, usage)).toBe(old);
  expect(recordInspectorUsage(null, 1, usage)).toBeNull();
  expect(buildInspectorCost(catalog, "fast", 1000, next).actual).toBe("unknown — missing usage");
});

test("all cost unknown reasons remain visible and never display zero", () => {
  const cases = [
    { catalog: null, id: undefined, usage, reason: "unresolved model" },
    { catalog, id: "absent", usage, reason: "missing model price" },
    { catalog: { ...catalog, models: [{ ...catalog.models[0]!, input_price_per_million: -1 }] }, id: "fast", usage, reason: "invalid price" },
    { catalog, id: "fast", usage: null, reason: "missing usage" },
    { catalog, id: "fast", usage: { ...usage, promptTokens: 1_000_000_001 }, reason: "invalid usage" },
  ];
  for (const item of cases) {
    let run = beginInspectorRun(1, item.catalog, item.id);
    if (item.usage) run = recordInspectorUsage(run, 1, item.usage)!;
    expect(buildInspectorCost(item.catalog, item.id, 100, run).actual).toBe(`unknown — ${item.reason}`);
  }
  expect(buildInspectorCost(null, undefined, 100, null).estimate).toBe("unknown — unresolved model");
  expect(buildInspectorCost(catalog, "fast", null, null).estimate).toBe("unknown — no request preview");
});

test("invalid latest usage stays unknown after valid usage; explicit zero is known", () => {
  const valid = recordInspectorUsage(beginInspectorRun(1, catalog, "fast"), 1, usage);
  const invalid = recordInspectorUsage(valid, 1, { ...usage, completionTokens: Number.NaN });
  expect(buildInspectorCost(catalog, "fast", 0, invalid).actual).toBe("unknown — invalid usage");
  const zero = recordInspectorUsage(valid, 1, { ...usage, promptTokens: 0, completionTokens: 0 });
  expect(buildInspectorCost(catalog, "fast", 0, zero).actual).toBe("$0");
});

test("USD display rounds only in the view, retains tiny nonzero and large exact values", () => {
  expect(formatInspectorUsd({ numerator: 1n, denominator: 1_000_000n })).toBe("<$0.0001");
  expect(formatInspectorUsd({ numerator: 123456n, denominator: 1_000_000n })).toBe("$0.1235");
  expect(formatInspectorUsd({ numerator: 10n ** 310n, denominator: 10n ** 308n })).toBe("$100");
});
