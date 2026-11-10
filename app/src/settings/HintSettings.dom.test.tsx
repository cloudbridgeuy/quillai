import { afterEach, expect, test } from "bun:test";
import { act } from "react";
import { defaultLocale, formatMessage } from "../i18n/messages";
import type { Rule } from "../md_editor/rules/engine";
import { cleanup, click, render } from "../test/dom";
import { createSettingsClient, type SeriesRules } from "./client";
import { HintSettings } from "./HintSettings";

afterEach(cleanup);

const rules: Rule[] = [
  { id: "adverbs", label: "Adverbs", languages: ["en"], defaultEnabled: true, defaultScope: "both", params: [{ key: "max", kind: "int", label: "Max per sentence", default: 2, min: 1, max: 9 }], check: () => [] },
  { id: "watch", label: "Watch words", languages: ["en", "es"], defaultEnabled: false, defaultScope: "narration", params: [{ key: "words", kind: "words", label: "Words", default: ["just"] }], check: () => [] },
  { id: "leismo", label: "Leísmo", languages: ["es"], defaultEnabled: true, defaultScope: "both", params: [], check: () => [] },
];

async function setup(stored: SeriesRules, { failPut = false, catalogue = rules }: { failPut?: boolean; catalogue?: readonly Rule[] } = {}) {
  const calls: { method: string; url: string; body?: unknown }[] = [];
  let state = stored;
  const client = createSettingsClient({ baseUrl: "http://api.test", fetch: async (url, init) => {
    const method = init?.method ?? "GET";
    if (method === "PUT") {
      const body: Partial<SeriesRules> = JSON.parse(String(init?.body));
      calls.push({ method, url, body });
      if (failPut) return Response.json({ error: "bad", code: "VALIDATION_ERROR" }, { status: 422 });
      state = { ...state, ...body };
    } else calls.push({ method, url });
    return Response.json(state);
  } });
  const container = await render(<HintSettings client={client} seriesId={7} rules={catalogue} />);
  const control = <T extends HTMLElement>(label: string, selector: string) => [...container.querySelectorAll("label")].find(el => el.querySelector("span")?.textContent === label)?.querySelector<T>(selector) ?? undefined;
  const toggles = () => [...container.querySelectorAll<HTMLInputElement>("input[type=checkbox]")].map(box => [box.parentElement?.textContent, box.checked]);
  const edit = (el: HTMLInputElement | HTMLTextAreaElement | undefined, value: string) => act(async () => {
    if (!el) return;
    Object.getOwnPropertyDescriptor(Object.getPrototypeOf(el), "value")!.set!.call(el, value);
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
  });
  return { container, calls, control, toggles, edit };
}

test("loads the series language and shows only its rules, in catalogue order", async () => {
  const { calls, control, toggles } = await setup({ language: "en", rules: { watch: { enabled: true } } });
  expect(calls).toEqual([{ method: "GET", url: "http://api.test/series/7/rules" }]);
  expect(control<HTMLSelectElement>("Language", "select")?.value).toBe("en");
  expect(toggles()).toEqual([["Adverbs", true], ["Watch words", true]]);
});

test("an empty rule catalogue shows the heading and language only", async () => {
  const { container, control, toggles } = await setup({ language: "es", rules: {} }, { catalogue: [] });
  expect(container.querySelector("h3")?.textContent).toBe("Hints");
  expect(control<HTMLSelectElement>("Language", "select")?.value).toBe("es");
  expect(toggles()).toEqual([]);
  expect(container.querySelectorAll("textarea, input").length).toBe(0);
});

test("a toggle saves the whole rules object and keeps unknown ids", async () => {
  const { calls, control, toggles } = await setup({ language: "en", rules: { legacy: { enabled: false } } });
  const box = control<HTMLInputElement>("Adverbs", "input");
  if (box) await click(box);
  expect(calls.at(-1)).toEqual({ method: "PUT", url: "http://api.test/series/7/rules", body: { rules: { legacy: { enabled: false }, adverbs: { enabled: false } } } });
  expect(toggles()).toEqual([["Adverbs", false], ["Watch words", false]]);
});

test("a language change saves the language and swaps the rules shown", async () => {
  const { calls, control, toggles } = await setup({ language: "en", rules: {} });
  await act(async () => {
    const select = control<HTMLSelectElement>("Language", "select");
    if (!select) return;
    select.value = "es";
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(calls.at(-1)).toEqual({ method: "PUT", url: "http://api.test/series/7/rules", body: { language: "es" } });
  expect(toggles()).toEqual([["Watch words", false], ["Leísmo", true]]);
});

test("a failed save shows the error and restores the saved value", async () => {
  const { container, calls, control, toggles } = await setup({ language: "en", rules: {} }, { failPut: true });
  const box = control<HTMLInputElement>("Adverbs", "input");
  if (box) await click(box);
  expect(calls.length).toBe(2);
  expect(container.querySelector("[role=alert]")?.textContent).toBe(formatMessage(defaultLocale, "error.validation"));
  expect(toggles()).toEqual([["Adverbs", true], ["Watch words", false]]);
});

test("an int param saves as a number on blur", async () => {
  const { calls, control, edit } = await setup({ language: "en", rules: {} });
  expect(control<HTMLInputElement>("Max per sentence", "input")?.value).toBe("2");
  await edit(control<HTMLInputElement>("Max per sentence", "input"), "4");
  expect(calls.at(-1)?.body).toEqual({ rules: { adverbs: { params: { max: 4 } } } });
});

test("the watch-words field is one word per line", async () => {
  const { calls, control, edit } = await setup({ language: "en", rules: { watch: { params: { words: ["so", "very"] } } } });
  expect(control<HTMLTextAreaElement>("Words", "textarea")?.value).toBe("so\nvery");
  await edit(control<HTMLTextAreaElement>("Words", "textarea"), " really \n\nquite\n");
  expect(calls.at(-1)?.body).toEqual({ rules: { watch: { params: { words: ["really", "quite"] } } } });
});
