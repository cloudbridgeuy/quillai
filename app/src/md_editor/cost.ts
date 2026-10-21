import type { AiStreamEvent } from "./ai_contract";
import type { AiStreamUsageEvent } from "./ai_stream";
import type { ModelCatalog } from "./model_catalog";

export const MAX_COST_TOKENS_PER_DIRECTION = 1_000_000_000;

export type CostModelIdentity =
  | { kind: "known"; modelId: string }
  | { kind: "server-default-unresolved" };

type Rational = Readonly<{ numerator: bigint; denominator: bigint }>;

type UnknownPricing = Readonly<{
  kind: "unknown";
  reason: "unresolved-model" | "missing-model-price" | "invalid-price";
}>;

export type CostPricing = UnknownPricing | Readonly<{
  kind: "priced";
  modelId: string;
  inputUsdPerMillion: Rational;
  outputUsdPerMillion: Rational;
}>;

export type ActualCost =
  | UnknownPricing
  | Readonly<{ kind: "unknown"; reason: "missing-usage" | "invalid-usage" }>
  | Readonly<{
    kind: "known";
    modelId: string;
    coverage: "observed-usage";
    usd: Rational & { readonly currency: "USD" };
  }>;

export function snapshotCostPricing(
  catalog: ModelCatalog | null,
  identity: CostModelIdentity,
): CostPricing {
  if (identity.kind === "server-default-unresolved") {
    return { kind: "unknown", reason: "unresolved-model" };
  }
  const models = catalog?.models.filter(({ id }) => id === identity.modelId) ?? [];
  const model = models[0];
  if (models.length !== 1 || !model) {
    return { kind: "unknown", reason: "missing-model-price" };
  }
  const inputUsdPerMillion = decimalPrice(model.input_price_per_million);
  const outputUsdPerMillion = decimalPrice(model.output_price_per_million);
  if (!inputUsdPerMillion || !outputUsdPerMillion) {
    return { kind: "unknown", reason: "invalid-price" };
  }
  return { kind: "priced", modelId: model.id, inputUsdPerMillion, outputUsdPerMillion };
}

export function calculateActualCost(
  pricing: CostPricing,
  events: readonly (AiStreamEvent | AiStreamUsageEvent)[],
): ActualCost {
  if (pricing.kind === "unknown") return pricing;
  let latest: AiStreamUsageEvent | undefined;
  for (const event of events) {
    if (event.type === "usage") latest = event;
  }
  if (!latest) return { kind: "unknown", reason: "missing-usage" };
  if (!isTokenCount(latest.promptTokens) || !isTokenCount(latest.completionTokens)) {
    return { kind: "unknown", reason: "invalid-usage" };
  }
  const input = pricing.inputUsdPerMillion;
  const output = pricing.outputUsdPerMillion;
  const usd = reduce(
    BigInt(latest.promptTokens) * input.numerator * output.denominator
      + BigInt(latest.completionTokens) * output.numerator * input.denominator,
    input.denominator * output.denominator * 1_000_000n,
  );
  return {
    kind: "known",
    modelId: pricing.modelId,
    coverage: "observed-usage",
    usd: { currency: "USD", ...usd },
  };
}

function isTokenCount(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 0 && value <= MAX_COST_TOKENS_PER_DIRECTION;
}

function decimalPrice(value: number): Rational | null {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) return null;
  const [mantissa = "0", exponent = "0"] = value.toString().split("e");
  const [integer = "0", fraction = ""] = mantissa.split(".");
  const numerator = BigInt(integer + fraction);
  const scale = fraction.length - Number(exponent);
  return scale >= 0
    ? reduce(numerator, 10n ** BigInt(scale))
    : reduce(numerator * 10n ** BigInt(-scale), 1n);
}

function reduce(numerator: bigint, denominator: bigint): Rational {
  let a = numerator;
  let b = denominator;
  while (b !== 0n) [a, b] = [b, a % b];
  return { numerator: numerator / a, denominator: denominator / a };
}
