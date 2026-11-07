import { useEffect, useRef, useState } from "react";
import { defaultLocale, formatMessage } from "../i18n/messages";
import { Alert } from "../ui/Alert";
import { Button } from "../ui/Button";
import { Field } from "../ui/Field";
import { Heading } from "../ui/Heading";
import { Select } from "../ui/Select";
import type { ModelCatalog } from "../md_editor/model_catalog";
import { TextField } from "../ui/TextField";
import { aiKeyDecision, aiModelActions, aiSettingsUpdate, type AiSettings, type AiSettingsForm, type createSettingsClient } from "./client";

export function AiSettings({ client, onSaved, focusAction }: { client: ReturnType<typeof createSettingsClient>; onSaved: () => void; focusAction?: "generate" | "edit" }) {
  const root = useRef<HTMLElement>(null);
  const [catalogLoaded, setCatalogLoaded] = useState(false);
  const [settings, setSettings] = useState<AiSettings | null>(null);
  const [catalog, setCatalog] = useState<ModelCatalog | null>(null);
  const [form, setForm] = useState<AiSettingsForm>({ base_url: "", models: { generate: "", edit: "", image: "", lore_complete: "" }, key: { action: "keep" } });
  const [error, setError] = useState("");
  const [pending, setPending] = useState(true);
  function apply(value: AiSettings) {
    setSettings(value);
    setForm({ base_url: value.base_url ?? "", models: { generate: value.generate_model ?? "", edit: value.edit_model ?? "", image: value.image_model ?? "", lore_complete: value.lore_complete_model ?? "" }, key: { action: "keep" } });
  }
  useEffect(() => {
    let active = true;
    client.getModelCatalog().then(value => { if (active) { setCatalog(value); setCatalogLoaded(true); } });
    client.getAiSettings().then(value => { if (active) apply(value); })
      .catch((reason: unknown) => { if (active) setError(reason instanceof Error ? reason.message : formatMessage(defaultLocale, "settings.ai.loadFailed")); })
      .finally(() => { if (active) setPending(false); });
    return () => { active = false; };
  }, [client]);
  useEffect(() => {
    if (!focusAction || !settings || !catalogLoaded) return;
    root.current?.querySelector<HTMLElement>(`[data-model-action="${focusAction}"] select, [data-model-action="${focusAction}"] input`)?.focus({ preventScroll: true });
  }, [focusAction, settings !== null, catalogLoaded]);
  async function save() {
    if (!settings) return;
    setPending(true);
    setError("");
    try {
      apply(await client.putAiSettings(aiSettingsUpdate(settings, form)));
      setCatalog(await client.getModelCatalog());
      onSaved();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : formatMessage(defaultLocale, "settings.ai.saveFailed"));
    } finally { setPending(false); }
  }
  const source = (value: "saved" | "env" | "none") => formatMessage(defaultLocale, value === "saved" ? "settings.ai.savedHere" : value === "env" ? "settings.ai.fromEnv" : "settings.ai.notSet");
  function modelOptions({ action, field }: (typeof aiModelActions)[number]) {
    const saved = form.models[action];
    const defaultModel = action === "lore_complete" ? undefined : catalog!.defaults[action];
    const fallback = settings!.sources[field] === "saved" || defaultModel === undefined ? formatMessage(defaultLocale, "settings.ai.default") : formatMessage(defaultLocale, "settings.ai.defaultModel", { model: defaultModel });
    const models = catalog!.models.filter(({ kind }) => (kind === "image") === (action === "image"));
    const options = [{ value: "", label: fallback }, ...models.map(({ id, label }) => ({ value: id, label: label || id }))];
    if (saved && !catalog!.models.some(({ id }) => id === saved)) options.push({ value: saved, label: formatMessage(defaultLocale, "settings.ai.notInCatalog", { model: saved }) });
    return options;
  }
  const keyDecision = settings ? aiKeyDecision(settings.sources.api_key, form.key) : null;
  return <section ref={root} className="space-y-3">
    <Heading level="section" as="h3">{formatMessage(defaultLocale, "settings.ai.title")}</Heading>
    {settings ? <div className="space-y-3">
      <Field label={formatMessage(defaultLocale, "settings.ai.baseUrl")}><TextField value={form.base_url} disabled={pending} onChange={event => setForm({ ...form, base_url: event.target.value })} /></Field>
      <p className="editor-note">{source(settings.sources.base_url)}</p>
      {aiModelActions.filter(({ action }) => action === "lore_complete" ? catalog !== null : action !== "image" || catalog?.models.some(({ kind }) => kind === "image")).map(entry => {
        const { action, field, labelKey } = entry;
        const label = formatMessage(defaultLocale, labelKey);
        const noteId = `ai-model-source-${action}`;
        const setModel = (value: string) => setForm({ ...form, models: { ...form.models, [action]: value } });
        return <div key={action} data-model-action={action} className="space-y-1">
          {catalog
            ? <Select label={label} value={form.models[action] ?? ""} options={modelOptions(entry)} disabled={pending} aria-describedby={noteId} onChange={setModel} />
            : <Field label={label}><TextField value={form.models[action] ?? ""} disabled={pending} aria-describedby={noteId} onChange={event => setModel(event.target.value)} /></Field>}
          <p id={noteId} className="editor-note">{source(settings.sources[field] ?? "none")}</p>
        </div>;
      })}
      <p className="editor-note">{formatMessage(defaultLocale, settings.key_set ? "settings.ai.keySaved" : "settings.ai.noKey")} · {source(settings.sources.api_key)}</p>
      <div className="flex gap-2">
        <Button size="sm" disabled={pending} onClick={() => setForm({ ...form, key: { action: "replace", value: "" } })}>{formatMessage(defaultLocale, "settings.ai.replace")}</Button>
        {keyDecision?.showRemove ? <Button size="sm" disabled={pending} onClick={() => setForm({ ...form, key: { action: "remove" } })}>{formatMessage(defaultLocale, "settings.ai.remove")}</Button> : null}
      </div>
      {form.key.action === "replace" ? <Field label={formatMessage(defaultLocale, "settings.ai.replacementKey")}><TextField type="password" autoComplete="off" value={form.key.value} disabled={pending} onChange={event => setForm({ ...form, key: { action: "replace", value: event.target.value } })} /></Field> : null}
      {keyDecision?.note ? <p className="editor-note">{keyDecision.note}</p> : null}
      <Button variant="primary" size="sm" disabled={pending} onClick={save}>{formatMessage(defaultLocale, "settings.ai.save")}</Button>
    </div> : null}
    {error ? <Alert tone="error" variant="inline">{error}</Alert> : null}
  </section>;
}
