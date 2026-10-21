import { expect, test } from "bun:test";
import { buildAiRequest } from "./ai_contract";
import { initialPersistedEditorState, parsePersistedEditorState } from "./editor_persistence";
import { parseModelCatalog, resolveModelChoice } from "./model_catalog";

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

test("missing and unknown persisted choices resolve to each action's server default", () => {
  const catalog = parseModelCatalog(response);
  for (const settings of [{}, { continueModel: "removed", editModel: "removed" },
    { continueModel: 2, editModel: [] }]) {
    const state = initialPersistedEditorState();
    const parsed = parsePersistedEditorState(JSON.stringify({ ...state,
      threadState: { ...state.threadState, settings: { maxOutputTokens: 512, resumePrompt: "Continue", ...settings } } }));
    expect(resolveModelChoice(catalog, "generate", parsed.threadState.settings.continueModel)).toBe(response.defaults.generate);
    expect(resolveModelChoice(catalog, "edit", parsed.threadState.settings.editModel)).toBe(response.defaults.edit);
  }
});

test("chosen models enter both action requests while unavailable catalogs omit model", () => {
  const catalog = parseModelCatalog(response);
  for (const action of ["generate", "edit"] as const) {
    for (const model of response.models) {
      const input = { action, model: resolveModelChoice(catalog, action, model.id),
        document: { text: "text", delta: { ops: [{ insert: "text" }] } },
        selection: { text: "text", delta: { ops: [{ insert: "text" }] }, range: { index: 0, length: 4 } },
        window: { start: 0, end: 4 } };
      expect(buildAiRequest(input).model).toBe(model.id);
      expect(buildAiRequest({ ...input, model: resolveModelChoice(null, action, model.id) })).not.toHaveProperty("model");
    }
  }
});
