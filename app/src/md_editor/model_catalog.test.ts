import { expect, test } from "bun:test";
import { buildInspectorCost } from "./inspector_cost";
import { initialPersistedEditorState, parsePersistedEditorState } from "./editor_persistence";
import { expectedModelId, parseModelCatalog } from "./model_catalog";

const response = {
  models: [
    { id: "provider/fast", label: "Fast", input_price_per_million: 0, output_price_per_million: 0.5 },
    { id: "provider/precise", label: "Precise", input_price_per_million: 2, output_price_per_million: 6 },
  ],
  defaults: { generate: "provider/fast", edit: "provider/precise" },
};

test("catalog parsing preserves public fields and strips provider configuration", () => {
  expect(parseModelCatalog({ ...response, provider: "secret" })).toEqual(response);
});

test("catalog parsing rejects empty, malformed and inconsistent responses without throwing", () => {
  for (const value of [null, [], {}, { ...response, models: [] },
    { ...response, defaults: { generate: "missing", edit: "provider/fast" } },
    { ...response, defaults: null }, { ...response, models: [response.models[0], response.models[0]] }]) {
    expect(parseModelCatalog(value)).toBeNull();
  }
  for (const fields of [{ id: " " }, { label: null }, { input_price_per_million: -1 },
    { output_price_per_million: Infinity }, { input_price_per_million: "2" }]) {
    expect(parseModelCatalog({ ...response, models: [{ ...response.models[0], ...fields }] })).toBeNull();
  }
});

test("old persisted per-chapter model fields still parse and are ignored", () => {
  for (const settings of [{}, { continueModel: "removed", editModel: "removed" }, { continueModel: 2, editModel: [] }]) {
    const state = initialPersistedEditorState();
    const parsed = parsePersistedEditorState(JSON.stringify({ ...state,
      threadState: { ...state.threadState, settings: { maxOutputTokens: 512, resumePrompt: "Continue", ...settings } } }));
    expect(parsed.threadState.settings).toEqual({ maxOutputTokens: 512, resumePrompt: "Continue" });
  }
});

test("expected model for an action is the catalog default, with no per-chapter pick", () => {
  const catalog = parseModelCatalog(response);
  expect(expectedModelId(catalog, "generate")).toBe(response.defaults.generate);
  expect(expectedModelId(catalog, "edit")).toBe(response.defaults.edit);
  expect(expectedModelId(null, "edit")).toBeUndefined();
  const view = buildInspectorCost(catalog, expectedModelId(catalog, "edit"), 1000, null);
  expect(view.model).toBe("Precise");
  expect(view.estimate).toBe("≈ $0.002 · input only");
  expect(view.estimate).not.toContain("unknown");
});
