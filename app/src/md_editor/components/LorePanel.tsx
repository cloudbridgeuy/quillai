import { defaultLocale, formatMessage } from "../../i18n/messages";
import { useId, useRef, useState } from "react";
import type { LoreEntry, LoreInput } from "../../manuscripts/lore_client";
import { newLoreDraft, parseLoreTerms, replaceLoreEntry } from "../../manuscripts/lore_panel";
import type { useLore } from "../../manuscripts/use_lore";
import { Button } from "../../ui/Button";
import { ListButton } from "../../ui/ListButton";
import { Alert } from "../../ui/Alert";
import { Select } from "../../ui/Select";
import { pickerOptions } from "../../manuscripts/timeline_panel";
import { TimelineManager } from "./TimelineManager";
import { Toggle } from "../../ui/Toggle";
import { Modal } from "../../ui/Modal";
import { Field } from "../../ui/Field";
import { Heading } from "../../ui/Heading";
import { StickyNote } from "../../ui/StickyNote";
import { TextField } from "../../ui/TextField";
import { PIN_HINT, TERMS_HINT } from "./lore_pin_hint";
import { loreEntryLabel, loreEntryPreview, loreLevel } from "./lore_entry_row";

export function LorePanel({ lore, disabled, pieceId = null }: { lore: ReturnType<typeof useLore>; disabled: boolean; pieceId?: number | null }) {
  const [draft, setDraft] = useState<LoreInput | null>(null);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [deleteArmed, setDeleteArmed] = useState(false);
  const returnFocusRef = useRef<HTMLElement | null>(null);
  const formId = useId();
  function close() {
    setDraft(null);
    setDeleteArmed(false);
    setError("");
  }
  function edit(opener: HTMLElement, entry?: LoreEntry) {
    returnFocusRef.current = opener;
    setDeleteArmed(false);
    setEditingId(entry?.id ?? null);
    setDraft(entry ?? newLoreDraft());
    setError("");
  }
  async function save(remove = false) {
    const universeId = lore.universeId;
    if (!draft || universeId === null) return;
    setPending(true);
    setError("");
    try {
      if (remove && editingId !== null) {
        await lore.client.delete({ universeId, loreId: editingId });
        lore.change(lore.entries.filter(entry => entry.id !== editingId));
      } else {
        const input: LoreInput = { kind: draft.kind, name: draft.name, keywords: draft.keywords, aliases: draft.aliases,
          visible_text: draft.visible_text, hidden_notes: draft.hidden_notes, level: draft.level, pinned: draft.pinned };
        const saved = editingId === null ? await lore.client.create({ universeId }, input)
          : await lore.client.update({ universeId, loreId: editingId }, input);
        lore.change(replaceLoreEntry(lore.entries, saved));
      }
      close();
    } catch (error) {
      setError(error instanceof Error ? error.message : formatMessage(defaultLocale, "lore.saveFailed"));
    } finally { setPending(false); }
  }
  async function setPoint(value: string) {
    if (pieceId === null) return;
    setPending(true);
    setError("");
    try {
      const saved = await lore.client.setPiecePoint({ pieceId }, value === "" ? null : Number(value));
      lore.setPiecePoint(saved.timelinePointId);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : formatMessage(defaultLocale, "timeline.failed"));
    } finally { setPending(false); }
  }
  return <section className="editor-lore-panel" aria-label={formatMessage(defaultLocale, "lore.series")}>
    <Heading level="section" as="h3">{formatMessage(defaultLocale, "lore.series")}</Heading>
    {pieceId !== null && lore.status === "ready" ? <Select label={formatMessage(defaultLocale, "timeline.chapterAt")} disabled={disabled || pending}
      value={lore.piece?.timelinePointId != null && lore.points.some(point => point.id === lore.piece?.timelinePointId) ? String(lore.piece.timelinePointId) : ""}
      options={pickerOptions(lore.points, formatMessage(defaultLocale, "timeline.base"))} onChange={value => void setPoint(value)} /> : null}
    {error && !draft ? <Alert tone="error" variant="inline">{error}</Alert> : null}
    {lore.status === "loading" ? <p role="status">{formatMessage(defaultLocale, "lore.loading")}</p> : null}
    {lore.status === "failed" ? <Alert tone="error" variant="inline">{lore.error}<Button size="sm" onClick={lore.retry}>{formatMessage(defaultLocale, "lore.retry")}</Button></Alert> : null}
    {lore.status === "ready" ? <>
      {lore.entries.length === 0 ? <p>{formatMessage(defaultLocale, "lore.empty")}</p> : <ul className="editor-manuscript-tree">
        {lore.entries.map(entry => <li key={entry.id}><ListButton tone="lore" disabled={disabled || pending} aria-label={loreEntryLabel(entry)}
          onClick={event => edit(event.currentTarget, entry)}>
          <strong className="min-w-0 truncate">{entry.name}</strong><span>{loreLevel(entry.level)}</span>
          <small>{entry.kind}{entry.pinned ? formatMessage(defaultLocale, "lore.pinned") : ""}</small><small className="max-w-40 min-w-0 truncate">{loreEntryPreview(entry.visible_text)}</small>
        </ListButton></li>)}
      </ul>}
      <Button disabled={disabled || pending} onClick={event => edit(event.currentTarget)}>{formatMessage(defaultLocale, "lore.new")}</Button>
      <TimelineManager lore={lore} disabled={disabled || pending} />
    </> : null}
    {draft ? <Modal data-manuscript-draft title={formatMessage(defaultLocale, editingId === null ? "lore.create" : "lore.edit")} onClose={close} returnFocusRef={returnFocusRef}
      footer={<>{editingId !== null ? <Button variant="danger" disabled={disabled || pending}
        onClick={() => { if (deleteArmed) void save(true); else setDeleteArmed(true); }}>{formatMessage(defaultLocale, deleteArmed ? "landing.deleted.confirmDelete" : "lore.delete")}</Button> : null}
        <Button variant="primary" type="submit" form={formId} disabled={disabled || pending || !draft.name.trim()}>{formatMessage(defaultLocale, "lore.save")}</Button></>}>
      <form id={formId} onSubmit={event => { event.preventDefault(); void save(); }}>
      <fieldset disabled={disabled || pending}>
        <Field className="my-3" label={formatMessage(defaultLocale, "tree.kind")}><TextField as="select" value={draft.kind}
          onChange={event => setDraft({ ...draft, kind: event.target.value as LoreInput["kind"] })}>
          {(["character", "place", "term", "note"] as const).map(kind => <option key={kind} value={kind}>{formatMessage(defaultLocale, `lore.kind.${kind}`)}</option>)}
        </TextField></Field>
        <Field className="my-3" label={formatMessage(defaultLocale, "lore.name")}><TextField required value={draft.name}
          onChange={event => setDraft({ ...draft, name: event.target.value })} /></Field>
        {(["keywords", "aliases"] as const).map(field => <Field className="my-3" key={field} label={formatMessage(defaultLocale, field === "keywords" ? "lore.keywords" : "lore.aliases")}>
          <StickyNote tone="green" rows={2} defaultValue={draft[field].join("\n")} key={`${editingId}-${field}`} aria-describedby={`${formId}-terms-hint`}
            onChange={event => setDraft({ ...draft, [field]: parseLoreTerms(event.target.value) })} />
        </Field>)}
        <p id={`${formId}-terms-hint`} className="text-ui text-graphite">{TERMS_HINT}</p>
        <Field className="my-3" label={formatMessage(defaultLocale, "lore.visible")}><StickyNote tone="green" rows={3}
          value={draft.visible_text} onChange={event => setDraft({ ...draft, visible_text: event.target.value })} /></Field>
        <Field className="my-3" label={formatMessage(defaultLocale, "lore.hidden")}><StickyNote tone="green" rows={3}
          value={draft.hidden_notes} onChange={event => setDraft({ ...draft, hidden_notes: event.target.value })} /></Field>
        <Field className="my-3" label={formatMessage(defaultLocale, "lore.requests")}><TextField as="select" value={draft.level}
          onChange={event => setDraft({ ...draft, level: event.target.value as LoreInput["level"] })}>
          <option value="include">{formatMessage(defaultLocale, "lore.include")}</option><option value="known_dont_reveal">{formatMessage(defaultLocale, "lore.known")}</option><option value="leave_out">{formatMessage(defaultLocale, "lore.leaveOut")}</option>
        </TextField></Field>
        <Toggle label={formatMessage(defaultLocale, "lore.pin")} checked={draft.pinned} aria-describedby={`${formId}-pin-hint`}
          onChange={event => setDraft({ ...draft, pinned: event.target.checked })} />
        <p id={`${formId}-pin-hint`} className="text-ui text-graphite">{PIN_HINT}</p>
      </fieldset>
      </form>
      {error ? <Alert tone="error" variant="inline">{error}</Alert> : null}
    </Modal> : null}
  </section>;
}
