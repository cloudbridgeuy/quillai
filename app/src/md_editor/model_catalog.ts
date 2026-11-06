import type { AiAction } from "./ai_contract";

export type AiModel = {
  id: string;
  label: string;
  input_price_per_million: number;
  output_price_per_million: number;
};

export type ModelCatalog = {
  models: AiModel[];
  defaults: Record<AiAction, string>;
};

export function parseModelCatalog(value: unknown): ModelCatalog | null {
  if (!isRecord(value) || !Array.isArray(value.models) || !isRecord(value.defaults)) return null;
  const models: AiModel[] = [];
  for (const model of value.models) {
    if (!isRecord(model) || typeof model.id !== "string" || !model.id.trim() ||
      typeof model.label !== "string" || !model.label.trim() ||
      !isPrice(model.input_price_per_million) || !isPrice(model.output_price_per_million) ||
      models.some(({ id }) => id === model.id)) return null;
    models.push({ id: model.id, label: model.label,
      input_price_per_million: model.input_price_per_million,
      output_price_per_million: model.output_price_per_million });
  }
  const { generate, edit } = value.defaults;
  if (typeof generate !== "string" || typeof edit !== "string" ||
    !models.some(({ id }) => id === generate) || !models.some(({ id }) => id === edit)) return null;
  return { models, defaults: { generate, edit } };
}

/** The model the server will use for an action: Settings, else the catalog default (already in `defaults`). */
export function expectedModelId(catalog: ModelCatalog | null, action: AiAction): string | undefined {
  return catalog?.defaults[action];
}

/** Display name (catalog label, else id) and id of the model an action will use; null without a catalog. */
export function actionModelDisplay(catalog: ModelCatalog | null, action: AiAction): { name: string; id: string } | null {
  const id = expectedModelId(catalog, action);
  if (!catalog || id === undefined) return null;
  return { name: catalog.models.find((model) => model.id === id)?.label ?? id, id };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isPrice(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}
