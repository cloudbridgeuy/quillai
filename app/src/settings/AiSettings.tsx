import { useEffect, useState } from "react";
import { Alert } from "../ui/Alert";
import { Button } from "../ui/Button";
import { Field } from "../ui/Field";
import { Heading } from "../ui/Heading";
import { TextField } from "../ui/TextField";
import { aiKeyDecision, aiSettingsUpdate, type AiSettings, type AiSettingsForm, type createSettingsClient } from "./client";

export function AiSettings({ client, onSaved }: { client: ReturnType<typeof createSettingsClient>; onSaved: () => void }) {
  const [settings, setSettings] = useState<AiSettings | null>(null);
  const [form, setForm] = useState<AiSettingsForm>({ base_url: "", model: "", key: { action: "keep" } });
  const [error, setError] = useState("");
  const [pending, setPending] = useState(true);
  function apply(value: AiSettings) {
    setSettings(value);
    setForm({ base_url: value.base_url ?? "", model: value.model ?? "", key: { action: "keep" } });
  }
  useEffect(() => {
    let active = true;
    client.getAiSettings().then(value => { if (active) apply(value); })
      .catch((reason: unknown) => { if (active) setError(reason instanceof Error ? reason.message : "Could not load AI settings."); })
      .finally(() => { if (active) setPending(false); });
    return () => { active = false; };
  }, [client]);
  async function save() {
    if (!settings) return;
    setPending(true);
    setError("");
    try {
      apply(await client.putAiSettings(aiSettingsUpdate(settings, form)));
      onSaved();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Could not save AI settings.");
    } finally { setPending(false); }
  }
  const source = (value: "saved" | "env" | "none") => value === "saved" ? "Saved here" : value === "env" ? "From quillai.env" : "Not set";
  const keyDecision = settings ? aiKeyDecision(settings.sources.api_key, form.key) : null;
  return <section className="space-y-3">
    <Heading level="section" as="h3">AI</Heading>
    {settings ? <div className="space-y-3">
      <Field label="Base URL"><TextField value={form.base_url} disabled={pending} onChange={event => setForm({ ...form, base_url: event.target.value })} /></Field>
      <p className="editor-note">{source(settings.sources.base_url)}</p>
      <Field label="Model"><TextField value={form.model} disabled={pending} onChange={event => setForm({ ...form, model: event.target.value })} /></Field>
      <p className="editor-note">{source(settings.sources.model)}</p>
      <p className="editor-note">{settings.key_set ? "Key saved" : "No key set"} · {source(settings.sources.api_key)}</p>
      <div className="flex gap-2">
        <Button size="sm" disabled={pending} onClick={() => setForm({ ...form, key: { action: "replace", value: "" } })}>Replace</Button>
        {keyDecision?.showRemove ? <Button size="sm" disabled={pending} onClick={() => setForm({ ...form, key: { action: "remove" } })}>Remove</Button> : null}
      </div>
      {form.key.action === "replace" ? <Field label="Replacement API key"><TextField type="password" autoComplete="off" value={form.key.value} disabled={pending} onChange={event => setForm({ ...form, key: { action: "replace", value: event.target.value } })} /></Field> : null}
      {keyDecision?.note ? <p className="editor-note">{keyDecision.note}</p> : null}
      <Button variant="primary" size="sm" disabled={pending} onClick={save}>Save AI settings</Button>
    </div> : null}
    {error ? <Alert tone="error" variant="inline">{error}</Alert> : null}
  </section>;
}
