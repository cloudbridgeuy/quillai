import { useEffect, useRef, useState } from "react";
import { defaultLocale, formatMessage } from "../i18n/messages";
import { createTrashClient, type TrashItem } from "../manuscripts/trash_client";
import { Alert } from "../ui/Alert";
import { Button } from "../ui/Button";
import { Heading } from "../ui/Heading";

export function RecentlyDeleted({ onChange }: { onChange: () => void }) {
  const [client] = useState(createTrashClient);
  const [items, setItems] = useState<TrashItem[]>([]);
  const [error, setError] = useState("");
  const [pending, setPending] = useState("");
  const [loading, setLoading] = useState(true);
  const [confirm, setConfirm] = useState("");
  const busy = useRef(false);
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    let active = true;
    void client.list().then(result => { if (active) { setItems(result); setLoading(false); } },
      reason => { if (active) { setError(reason instanceof Error ? reason.message : formatMessage(defaultLocale, "landing.deleted.loadFailed")); setItems([]); setLoading(false); } });
    return () => { active = false; };
  }, [client, revision]);

  async function act(item: TrashItem, action: "restore" | "delete") {
    const key = `${item.type}:${item.id}:${action}`;
    if (busy.current) return;
    if (confirm !== key && (action === "delete" || item.deletedParents.length > 0)) { setConfirm(key); return; }
    busy.current = true;
    setPending(key);
    setError("");
    try {
      if (action === "restore") await client.restore(item); else await client.deleteForever(item);
      onChange();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : formatMessage(defaultLocale, "landing.deleted.actionFailed"));
      onChange();
    } finally {
      setConfirm("");
      busy.current = false;
      setPending("");
      setLoading(true);
      setRevision(value => value + 1);
    }
  }

  return <section className="flex flex-col gap-3 rounded-card bg-paper p-5 shadow-sheet" role="region" aria-labelledby="recently-deleted">
    <Heading level="section" id="recently-deleted">{formatMessage(defaultLocale, "landing.deleted.title")}</Heading>
    {error && <Alert tone="error" variant="inline">{error} <Button size="sm" onClick={() => { setError(""); setLoading(true); setRevision(value => value + 1); }}>{formatMessage(defaultLocale, "landing.deleted.retry")}</Button></Alert>}
    {items.length === 0 ? <p className="m-0 text-ui text-graphite">{formatMessage(defaultLocale, loading ? "app.loading" : error ? "landing.deleted.loadFailedEmpty" : "landing.deleted.empty")}</p> : <ul className="m-0 flex list-none flex-col gap-3 p-0">
      {items.map(item => {
        const key = `${item.type}:${item.id}`;
        const restoring = confirm === `${key}:restore`, deleting = confirm === `${key}:delete`;
        const counts = Object.entries(item.childCounts).filter(([, count]) => count > 0).map(([name, count]) => formatMessage(defaultLocale, `landing.deleted.${name}`, { count, countText: String(count) })).join(", ");
        const deletePrompt = counts ? item.restoreAncestors.length > 0 ? "landing.deleted.deleteWithCountsAndParents" : "landing.deleted.deleteWithCounts"
          : item.restoreAncestors.length > 0 ? "landing.deleted.deleteWithParents" : "landing.deleted.deletePrompt";
        const date = new Date(item.deletedAt.includes("T") ? item.deletedAt : `${item.deletedAt.replace(" ", "T")}Z`);
        return <li key={key} className="flex flex-col gap-2 border-b border-rule pb-3">
          <div className="flex flex-wrap items-center gap-3"><span className="text-label text-graphite">{item.type[0]?.toUpperCase()}{item.type.slice(1)}</span>
            <span className="text-ui font-semibold text-ink">{item.title}</span>
            <time className="text-meta text-graphite" dateTime={date.toISOString()}>{date.toLocaleDateString()}</time></div>
          {restoring && <Alert tone="info" variant="inline">{formatMessage(defaultLocale, "landing.deleted.restoreNotice", { parents: item.deletedParents.join(", ") })}</Alert>}
          {deleting && <Alert tone="warning" variant="inline">{formatMessage(defaultLocale, deletePrompt, { title: item.title, counts, parents: item.deletedParents.join(", ") })}</Alert>}
          <div className="flex flex-wrap gap-2">
            <Button size="sm" disabled={!!pending || loading} onClick={() => void act(item, "restore")}>{formatMessage(defaultLocale, restoring ? "landing.deleted.confirmRestore" : "landing.deleted.restore")}</Button>
            <Button size="sm" variant="danger" disabled={!!pending || loading} onClick={() => void act(item, "delete")}>{formatMessage(defaultLocale, deleting ? "landing.deleted.confirmDelete" : "landing.deleted.deleteForever")}</Button>
          </div>
        </li>;
      })}
    </ul>}
  </section>;
}
