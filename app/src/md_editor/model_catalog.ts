import type { AiAction } from "./ai_contract";

export type ModelAction = AiAction | "image";

export type TextModel = {
  id: string;
  label: string;
  kind?: "text";
  input_price_per_million: number;
  output_price_per_million: number;
};

export type ImageModel = {
  id: string;
  label: string;
  kind: "image";
  price_per_image: number;
  size: string;
  quality: string;
};

export type AiModel = TextModel | ImageModel;

export type ModelCatalog = {
  models: AiModel[];
  defaults: Record<AiAction, string> & { image?: string };
};

export function parseModelCatalog(value: unknown): ModelCatalog | null {
  if (!isRecord(value) || !Array.isArray(value.models) || !isRecord(value.defaults)) return null;
  const models: AiModel[] = [];
  for (const model of value.models) {
    if (!isRecord(model) || typeof model.id !== "string" || !model.id.trim() ||
      typeof model.label !== "string" || !model.label.trim() ||
      models.some(({ id }) => id === model.id)) return null;
    const { id, label } = model;
    if (model.kind === "image") {
      if (!isPrice(model.price_per_image) || !isText(model.size) || !isText(model.quality)) return null;
      models.push({ id, label, kind: "image", price_per_image: model.price_per_image, size: model.size, quality: model.quality });
    } else {
      if ((model.kind !== undefined && model.kind !== "text") ||
        !isPrice(model.input_price_per_million) || !isPrice(model.output_price_per_million)) return null;
      models.push({ id, label,
        input_price_per_million: model.input_price_per_million,
        output_price_per_million: model.output_price_per_million });
    }
  }
  const { generate, edit, image } = value.defaults;
  const hasKind = (id: unknown, kind: "text" | "image") =>
    models.some((model) => model.id === id && (model.kind ?? "text") === kind);
  if (!hasKind(generate, "text") || !hasKind(edit, "text") ||
    (image !== undefined && !hasKind(image, "image"))) return null;
  const defaults = { generate: generate as string, edit: edit as string };
  return { models, defaults: image === undefined ? defaults : { ...defaults, image: image as string } };
}

export function expectedModelId(catalog: ModelCatalog | null, action: ModelAction): string | undefined {
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

function isText(value: unknown): value is string {
  return typeof value === "string" && value.trim() !== "";
}

function isPrice(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}
