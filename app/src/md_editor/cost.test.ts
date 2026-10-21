import { expect, test } from "bun:test";
import type { AiStreamEvent } from "./ai_contract";
import { parseAiStreamChunk, type AiStreamUsageEvent } from "./ai_stream";
import { calculateActualCost, MAX_COST_TOKENS_PER_DIRECTION, snapshotCostPricing } from "./cost";
import type { ModelCatalog } from "./model_catalog";

function catalog(input = 0.1, output = 0.2): ModelCatalog {
  return {
    models: [{ id: "sent-model", label: "Model", input_price_per_million: input, output_price_per_million: output }],
    defaults: { generate: "sent-model", edit: "sent-model" },
  };
}

function pricing(input = 0.1, output = 0.2) {
  return snapshotCostPricing(catalog(input, output), { kind: "known", modelId: "sent-model" });
}

function usage(promptTokens = 1, completionTokens = 1): AiStreamUsageEvent {
  return { type: "usage", promptTokens, completionTokens };
}

test("1 missing usage is unknown, explicit zero usage or free prices is known zero", () => {
  expect(calculateActualCost(pricing(), [])).toEqual({ kind: "unknown", reason: "missing-usage" });
  for (const result of [calculateActualCost(pricing(), [usage(0, 0)]),
    calculateActualCost(pricing(0, 0), [usage()])]) {
    expect(result).toEqual({ kind: "known", modelId: "sent-model", coverage: "observed-usage",
      usd: { currency: "USD", numerator: 0n, denominator: 1n } });
  }
});

test("2 decimal input and output aggregate exactly without intermediate rounding", () => {
  expect(calculateActualCost(pricing(), [usage()])).toEqual({
    kind: "known", modelId: "sent-model", coverage: "observed-usage",
    usd: { currency: "USD", numerator: 3n, denominator: 10_000_000n },
  });
  expect(calculateActualCost(pricing(1.25, 3.75), [usage(1_000_000, 2_000_000)])).toMatchObject({
    usd: { numerator: 35n, denominator: 4n },
  });
});

test("3 model identity must be known actually sent or resolved, never current default", () => {
  for (const source of [catalog(), null]) {
    expect(snapshotCostPricing(source, { kind: "server-default-unresolved" })).toEqual({
      kind: "unknown", reason: "unresolved-model",
    });
    expect(snapshotCostPricing(source, { kind: "known", modelId: "absent" })).toEqual({
      kind: "unknown", reason: "missing-model-price",
    });
  }
  expect(calculateActualCost(snapshotCostPricing(null, { kind: "known", modelId: "sent-model" }),
    [usage()])).toEqual({ kind: "unknown", reason: "missing-model-price" });
});

test("3 request pricing snapshot survives catalog mutation and default changes", () => {
  const source = catalog();
  const pinned = snapshotCostPricing(source, { kind: "known", modelId: "sent-model" });
  source.models[0]!.input_price_per_million = 900;
  source.models[0]!.output_price_per_million = 900;
  source.defaults.generate = "other";
  source.models.length = 0;
  expect(calculateActualCost(pinned, [usage()])).toEqual(calculateActualCost(pricing(), [usage()]));
});

test("4 last usage snapshot wins, including invalid final snapshot", () => {
  expect(calculateActualCost(pricing(), [usage(50, 80), usage(2, 3)])).toEqual(
    calculateActualCost(pricing(), [usage(2, 3)]),
  );
  expect(calculateActualCost(pricing(), [usage(), usage(-1, 0)])).toEqual({
    kind: "unknown", reason: "invalid-usage",
  });
  expect(calculateActualCost(pricing(), [usage(-1, 0), usage()])).toEqual(
    calculateActualCost(pricing(), [usage()]),
  );
});

test("5 stop, error, cancellation, length and EOF price observed usage only", () => {
  const endings: AiStreamEvent[][] = [[], [{ type: "done", finishReason: "stop" }],
    [{ type: "done", finishReason: "length" }], [{ type: "done", finishReason: "unknown" }],
    [{ type: "error", error: "failed" }], [{ type: "cancelled" }]];
  for (const ending of endings) {
    expect(calculateActualCost(pricing(), [usage(), ...ending])).toEqual(
      calculateActualCost(pricing(), [usage()]),
    );
    expect(calculateActualCost(pricing(), ending)).toEqual({ kind: "unknown", reason: "missing-usage" });
  }
});

test("6 accepts parsed usage only, excludes untagged estimates and uses standard rates", () => {
  const estimateAssignable: { promptTokens: number; completionTokens: number } extends AiStreamUsageEvent
    ? true : false = false;
  expect(estimateAssignable).toBe(false);
  const parsed = parseAiStreamChunk('{"type":"delta","delta":{"ops":[]}}\n'
    + '{"type":"usage","promptTokens":1,"completionTokens":1}\n');
  expect(parsed.ok).toBe(true);
  expect(calculateActualCost(pricing(), parsed.events)).toEqual(calculateActualCost(pricing(), [usage()]));
});

test("rejects nonfinite, negative, fractional, unsafe and over-bound counts in either direction", () => {
  for (const count of [NaN, Infinity, -Infinity, -1, 0.5, Number.MAX_SAFE_INTEGER + 1,
    MAX_COST_TOKENS_PER_DIRECTION + 1]) {
    for (const event of [usage(count, 0), usage(0, count)]) {
      expect(calculateActualCost(pricing(), [event])).toEqual({ kind: "unknown", reason: "invalid-usage" });
    }
  }
  expect(calculateActualCost(pricing(1, 1), [usage(MAX_COST_TOKENS_PER_DIRECTION,
    MAX_COST_TOKENS_PER_DIRECTION)])).toMatchObject({ usd: { numerator: 2000n, denominator: 1n } });
});

test("rejects invalid prices in either direction even for zero usage", () => {
  for (const price of [NaN, Infinity, -Infinity, -0.1]) {
    for (const snapshot of [pricing(price, 0), pricing(0, price)]) {
      expect(calculateActualCost(snapshot, [usage(0, 0)])).toEqual({ kind: "unknown", reason: "invalid-price" });
    }
  }
});

test("rejects missing and nonnumeric prices in either direction even for zero usage", () => {
  for (const field of ["input_price_per_million", "output_price_per_million"]) {
    for (const price of [undefined, null, "0", false, true]) {
      const source = JSON.parse(JSON.stringify(catalog(0, 0)));
      if (price === undefined) delete source.models[0][field];
      else source.models[0][field] = price;
      const snapshot = snapshotCostPricing(source, { kind: "known", modelId: "sent-model" });
      expect(snapshot).toEqual({ kind: "unknown", reason: "invalid-price" });
      expect(calculateActualCost(snapshot, [usage(0, 0)])).toEqual({
        kind: "unknown", reason: "invalid-price",
      });
    }
  }
});

test("scientific notation and finite numeric extremes remain exact rationals", () => {
  for (const [price, numerator, denominator] of [
    [1e-7, 1n, 10n ** 13n],
    [1.25e21, 125n * 10n ** 13n, 1n],
    [Number.MIN_VALUE, 1n, 2n * 10n ** 329n],
    [Number.MAX_VALUE, 17976931348623157n * 10n ** 286n, 1n],
  ] as const) {
    expect(calculateActualCost(pricing(price, 0), [usage(1, 0)])).toMatchObject({
      usd: { numerator, denominator },
    });
  }
});
