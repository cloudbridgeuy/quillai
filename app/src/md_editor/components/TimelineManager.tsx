import { useId, useRef, useState } from "react";
import { defaultLocale, formatMessage } from "../../i18n/messages";
import type { useLore } from "../../manuscripts/use_lore";
import { movePoint, nextPosition, orderPoints } from "../../manuscripts/timeline_panel";
import { Alert } from "../../ui/Alert";
import { Button } from "../../ui/Button";
import { Field } from "../../ui/Field";
import { ListButton } from "../../ui/ListButton";
import { Modal } from "../../ui/Modal";
import { TextField } from "../../ui/TextField";

const t = (key: Parameters<typeof formatMessage>[1], values?: Parameters<typeof formatMessage>[2]) => formatMessage(defaultLocale, key, values);

export function TimelineManager({ lore, disabled }: { lore: ReturnType<typeof useLore>; disabled: boolean }) {
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<number | null>(null);
  const [name, setName] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [deleteArmed, setDeleteArmed] = useState(false);
  const returnFocusRef = useRef<HTMLElement | null>(null);
  const formId = useId();
  const universeId = lore.universeId;
  const points = orderPoints(lore.points);
  function select(id: number | null) {
    setSelected(id);
    setName(lore.points.find(point => point.id === id)?.name ?? "");
    setDeleteArmed(false);
  }
  async function run(work: (universeId: number) => Promise<void>) {
    if (universeId === null) return;
    setPending(true);
    setError("");
    try { await work(universeId); } catch (failure) {
      setError(failure instanceof Error ? failure.message : t("timeline.failed"));
    } finally { lore.refreshTimeline(); setPending(false); }
  }
  const busy = disabled || pending;
  return <>
    <Button disabled={disabled || universeId === null} onClick={event => { returnFocusRef.current = event.currentTarget; select(null); setError(""); setOpen(true); }}>{t("timeline.open")}</Button>
    {open ? <Modal title={t("timeline.title")} onClose={() => setOpen(false)} returnFocusRef={returnFocusRef}
      footer={selected !== null ? <Button variant="danger" disabled={busy} onClick={() => {
        if (!deleteArmed) { setDeleteArmed(true); return; }
        void run(async () => { await lore.client.deletePoint({ pointId: selected }); select(null); });
      }}>{t(deleteArmed ? "landing.deleted.confirmDelete" : "timeline.delete")}</Button> : null}>
      {points.length === 0 ? <p>{t("timeline.empty")}</p> : <ul className="flex flex-col gap-1">
        {points.map((point, index) => <li key={point.id} className="flex gap-2">
          <ListButton tone="lore" className="min-w-0 flex-1" pressed={point.id === selected} disabled={busy} onClick={() => select(point.id)}>{point.name}</ListButton>
          <Button size="sm" disabled={busy || index === 0} aria-label={t("timeline.moveUp", { name: point.name })}
            onClick={() => void run(async () => { for (const move of movePoint(points, point.id, "up")) await lore.client.updatePoint({ pointId: move.id }, { position: move.position }); })}>↑</Button>
          <Button size="sm" disabled={busy || index === points.length - 1} aria-label={t("timeline.moveDown", { name: point.name })}
            onClick={() => void run(async () => { for (const move of movePoint(points, point.id, "down")) await lore.client.updatePoint({ pointId: move.id }, { position: move.position }); })}>↓</Button>
        </li>)}
      </ul>}
      <form id={formId} onSubmit={event => {
        event.preventDefault();
        const trimmed = name.trim();
        void run(async universeId => {
          if (selected === null) { await lore.client.createPoint({ universeId }, { name: trimmed, position: nextPosition(points) }); setName(""); }
          else await lore.client.updatePoint({ pointId: selected }, { name: trimmed });
        });
      }}>
        <Field className="my-3" label={t("timeline.name")}><TextField required value={name} disabled={busy} onChange={event => setName(event.target.value)} /></Field>
        <Button variant="primary" type="submit" disabled={busy || !name.trim()}>{t(selected === null ? "timeline.add" : "timeline.rename")}</Button>
        {selected !== null ? <Button disabled={busy} onClick={() => select(null)}>{t("timeline.new")}</Button> : null}
      </form>
      {error ? <Alert tone="error" variant="inline">{error}</Alert> : null}
    </Modal> : null}
  </>;
}
