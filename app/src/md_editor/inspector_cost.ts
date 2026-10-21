import type { AiStreamUsageEvent } from "./ai_stream";
import { calculateActualCost, snapshotCostPricing, type ActualCost, type CostPricing } from "./cost";
import type { ModelCatalog } from "./model_catalog";

export type InspectorRun = Readonly<{
  id: number;
  model: string;
  pricing: CostPricing;
  usage: AiStreamUsageEvent | null;
}>;

export function beginInspectorRun(id: number, catalog: ModelCatalog | null, modelId: string | undefined): InspectorRun {
  return { id, model: modelLabel(catalog, modelId), pricing: pricingFor(catalog, modelId), usage: null };
}

export function recordInspectorUsage(run: InspectorRun | null, id: number, usage: AiStreamUsageEvent): InspectorRun | null {
  return run?.id === id ? { ...run, usage } : run;
}

function pricingFor(catalog: ModelCatalog | null, modelId: string | undefined) {
  return snapshotCostPricing(catalog, modelId === undefined
    ? { kind: "server-default-unresolved" } : { kind: "known", modelId });
}

function modelLabel(catalog: ModelCatalog | null, modelId: string | undefined) {
  const model = catalog?.models.find(({ id }) => id === modelId);
  return model ? `${model.label} (${model.id})` : "unknown — unresolved model";
}

const reasons: Record<Extract<ActualCost, { kind: "unknown" }>["reason"], string> = {
  "unresolved-model": "unresolved model",
  "missing-model-price": "missing model price",
  "invalid-price": "invalid price",
  "missing-usage": "missing usage",
  "invalid-usage": "invalid usage",
};

export function formatInspectorUsd(value: { numerator: bigint; denominator: bigint }): string {
  if (value.numerator > 0n && value.numerator * 10_000n < value.denominator) return "<$0.0001";
  const scaled = (value.numerator * 10_000n + value.denominator / 2n) / value.denominator;
  const fraction = (scaled % 10_000n).toString().padStart(4, "0").replace(/0+$/, "");
  return `$${(scaled / 10_000n).toLocaleString("en-US")}${fraction ? `.${fraction}` : ""}`;
}

export function buildInspectorCost(catalog: ModelCatalog | null, modelId: string | undefined, inputTokens: number | null, run: InspectorRun | null) {
  const pricing = pricingFor(catalog, modelId);
  const unknown = (reason: keyof typeof reasons) => `unknown — ${reasons[reason]}`;
  const actual = run ? calculateActualCost(run.pricing, run.usage ? [run.usage] : []) : null;
  return {
    model: modelLabel(catalog, modelId),
    estimate: pricing.kind === "unknown" ? unknown(pricing.reason) : inputTokens === null ? "unknown — no request preview"
      : `≈ ${formatInspectorUsd({ numerator: BigInt(inputTokens) * pricing.inputUsdPerMillion.numerator, denominator: pricing.inputUsdPerMillion.denominator * 1_000_000n })} · input only`,
    outputRate: pricing.kind === "unknown" ? unknown(pricing.reason) : `${formatInspectorUsd(pricing.outputUsdPerMillion)} / million output tokens`,
    actualModel: run?.model ?? "No run yet",
    usage: run?.usage ? `${run.usage.promptTokens} input · ${run.usage.completionTokens} output tokens` : "unknown — missing usage",
    actual: actual === null ? "unknown — no run yet" : actual.kind === "unknown" ? unknown(actual.reason) : formatInspectorUsd(actual.usd),
  };
}
