import { useState } from "react";
import { defaultLocale, formatMessage } from "../../i18n/messages";
import { ManuscriptApiError } from "../../manuscripts/client";
import type { LoreEntry } from "../../manuscripts/lore_client";
import { replaceLoreEntry } from "../../manuscripts/lore_panel";
import { pickerOptions } from "../../manuscripts/timeline_panel";
import type { useLore } from "../../manuscripts/use_lore";
import { Alert } from "../../ui/Alert";
import { Button } from "../../ui/Button";
import { Field } from "../../ui/Field";
import { Heading } from "../../ui/Heading";
import { ListButton } from "../../ui/ListButton";
import { Select } from "../../ui/Select";
import { StickyNote } from "../../ui/StickyNote";
import { TextField } from "../../ui/TextField";

const t = (key: Parameters<typeof formatMessage>[1], values?: Parameters<typeof formatMessage>[2]) => formatMessage(defaultLocale, key, values);

type StateDraft = { id: number | null; name: string; pointId: string; visibleText: string; hiddenNotes: string };

const newStateDraft = (): StateDraft => ({ id: null, name: "", pointId: "", visibleText: "", hiddenNotes: "" });

export function LoreStates({ lore, entry, disabled }: { lore: ReturnType<typeof useLore>; entry: LoreEntry; disabled: boolean }) {
  const [draft, setDraft] = useState<StateDraft>(newStateDraft);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [deleteArmed, setDeleteArmed] = useState(false);
  const states = entry.states ?? [];
  const busy = disabled || pending;
  function select(id: number | null) {
    const state = states.find(row => row.id === id);
    setDraft(state ? { id: state.id, name: state.name, pointId: String(state.pointId), visibleText: state.visibleText, hiddenNotes: state.hiddenNotes } : newStateDraft());
    setDeleteArmed(false);
    setError("");
  }
  async function run(work: (scope: { universeId: number; loreId: number }) => Promise<void>) {
    const universeId = lore.universeId;
    if (universeId === null) return;
    const scope = { universeId, loreId: entry.id };
    setPending(true);
    setError("");
    try {
      await work(scope);
      lore.change(replaceLoreEntry(lore.entries, await lore.client.get(scope)));
      select(null);
    } catch (failure) {
      setError(failure instanceof ManuscriptApiError && failure.status === 409 ? t("lore.states.conflict")
        : failure instanceof Error ? failure.message : t("lore.states.failed"));
    } finally { setPending(false); }
  }
  function submit() {
    const input = { name: draft.name.trim(), pointId: Number(draft.pointId), visibleText: draft.visibleText, hiddenNotes: draft.hiddenNotes };
    void run(async scope => {
      if (draft.id === null) await lore.client.createState(scope, input);
      else await lore.client.updateState({ ...scope, stateId: draft.id }, input);
    });
  }
  return <section aria-label={t("lore.states.title")}>
    <Heading level="section" as="h4">{t("lore.states.title")}</Heading>
    {states.length === 0 ? <p>{t("lore.states.empty")}</p> : <ul className="flex flex-col gap-1">
      {states.map(state => {
        const point = lore.points.find(candidate => candidate.id === state.pointId);
        return <li key={state.id}><ListButton tone="lore" pressed={state.id === draft.id} disabled={busy} onClick={() => select(state.id)}>
          {point ? t("lore.states.row", { name: state.name, point: point.name }) : state.name}
        </ListButton></li>;
      })}
    </ul>}
    {lore.points.length === 0 ? <p>{t("lore.states.noPoints")}</p> : <form onSubmit={event => { event.preventDefault(); submit(); }}>
      <fieldset disabled={busy}>
        <Field className="my-3" label={t("lore.states.name")}><TextField required value={draft.name}
          onChange={event => setDraft({ ...draft, name: event.target.value })} /></Field>
        <Select className="my-3" label={t("lore.states.point")} value={draft.pointId} options={pickerOptions(lore.points, t("lore.states.choosePoint"))}
          onChange={pointId => setDraft({ ...draft, pointId })} />
        <Field className="my-3" label={t("lore.states.text")}><StickyNote tone="green" rows={3} value={draft.visibleText}
          onChange={event => setDraft({ ...draft, visibleText: event.target.value })} /></Field>
        <Field className="my-3" label={t("lore.states.notes")}><StickyNote tone="green" rows={3} value={draft.hiddenNotes}
          onChange={event => setDraft({ ...draft, hiddenNotes: event.target.value })} /></Field>
        <Button variant="primary" type="submit" disabled={!draft.name.trim() || draft.pointId === ""}>{t(draft.id === null ? "lore.states.add" : "lore.states.save")}</Button>
        {draft.id !== null ? <>
          <Button onClick={() => select(null)}>{t("lore.states.new")}</Button>
          <Button variant="danger" onClick={() => {
            if (!deleteArmed) { setDeleteArmed(true); return; }
            const stateId = draft.id!;
            void run(async scope => { await lore.client.deleteState({ ...scope, stateId }); });
          }}>{t(deleteArmed ? "landing.deleted.confirmDelete" : "lore.states.delete")}</Button>
        </> : null}
      </fieldset>
    </form>}
    {error ? <Alert tone="error" variant="inline">{error}</Alert> : null}
  </section>;
}
