import { useEffect, useState } from "react";
import { defaultLocale, formatMessage, type MessageKey } from "../i18n/messages";
import { RULES, resolveConfig, type Language, type ParamDef, type ParamValue, type Rule, type StoredRuleConfig } from "../md_editor/rules/engine";
import { Alert } from "../ui/Alert";
import { Field } from "../ui/Field";
import { Heading } from "../ui/Heading";
import { Select } from "../ui/Select";
import { TextField } from "../ui/TextField";
import { Toggle } from "../ui/Toggle";
import type { createSettingsClient, SeriesRules, SeriesRulesUpdate } from "./client";

const t = (key: MessageKey) => formatMessage(defaultLocale, key);

function parseParam(param: ParamDef, text: string): ParamValue {
  return param.kind === "int" ? Number(text.trim()) : text.split("\n").map(word => word.trim()).filter(Boolean);
}

export function HintSettings({ client, seriesId, rules = RULES }: { client: ReturnType<typeof createSettingsClient>; seriesId: number; rules?: readonly Rule[] }) {
  const [saved, setSaved] = useState<SeriesRules | null>(null);
  const [view, setView] = useState<SeriesRules | null>(null);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    client.getSeriesRules(seriesId).then(value => { if (active) { setSaved(value); setView(value); } })
      .catch((reason: unknown) => { if (active) setError(reason instanceof Error ? reason.message : String(reason)); });
    return () => { active = false; };
  }, [client, seriesId]);
  if (!view) return <section className="space-y-3"><Heading level="section" as="h3">{t("settings.hints.title")}</Heading>{error ? <Alert tone="error" variant="inline">{error}</Alert> : null}</section>;
  async function save(update: SeriesRulesUpdate) {
    setView({ ...view!, ...update });
    setDrafts({});
    setError("");
    try {
      const value = await client.putSeriesRules(seriesId, update);
      setSaved(value);
      setView(value);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setView(saved);
    }
  }
  const saveRule = (id: string, change: StoredRuleConfig) => save({ rules: { ...view.rules, [id]: { ...view.rules[id], ...change } } });
  const saveParam = (rule: Rule, param: ParamDef, draftKey: string) => drafts[draftKey] === undefined ? undefined
    : saveRule(rule.id, { params: { ...view.rules[rule.id]?.params, [param.key]: parseParam(param, drafts[draftKey]) } });
  return <section className="space-y-3">
    <Heading level="section" as="h3">{t("settings.hints.title")}</Heading>
    {error ? <Alert tone="error" variant="inline">{error}</Alert> : null}
    <Select label={t("settings.hints.language")} value={view.language} options={[{ value: "en", label: t("settings.hints.english") }, { value: "es", label: t("settings.hints.spanish") }]} onChange={value => save({ language: value as Language })} />
    {rules.filter(rule => rule.languages.includes(view.language)).map(rule => {
      const config = resolveConfig(rule, view.rules[rule.id]);
      return <div key={rule.id} className="space-y-2">
        <Toggle label={t(rule.labelKey)} checked={config.enabled} onChange={event => saveRule(rule.id, { enabled: event.currentTarget.checked })} />
        {rule.params.map(param => {
          const draftKey = `${rule.id}.${param.key}`;
          const value = config.params[param.key];
          const onBlur = () => saveParam(rule, param, draftKey);
          const text = drafts[draftKey] ?? (Array.isArray(value) ? value.join("\n") : String(value));
          const setDraft = (next: string) => setDrafts({ ...drafts, [draftKey]: next });
          return <Field key={param.key} label={t(param.labelKey)}>
            {param.kind === "int"
              ? <TextField type="number" min={param.min} max={param.max} step={1} value={text} onChange={event => setDraft(event.currentTarget.value)} onBlur={onBlur} />
              : <TextField as="textarea" rows={4} value={text} onChange={event => setDraft(event.currentTarget.value)} onBlur={onBlur} />}
          </Field>;
        })}
      </div>;
    })}
  </section>;
}
