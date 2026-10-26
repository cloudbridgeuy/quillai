import { useState } from "react";
import type { LoreEntry, LoreInput } from "../../manuscripts/lore_client";
import { newLoreDraft, parseLoreTerms, replaceLoreEntry } from "../../manuscripts/lore_panel";
import type { useLore } from "../../manuscripts/use_lore";

export function LorePanel({ lore, disabled }: { lore: ReturnType<typeof useLore>; disabled: boolean }) {
  const [draft, setDraft] = useState<LoreInput | null>(null);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  function edit(entry?: LoreEntry) {
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
      setDraft(null);
    } catch (error) {
      setError(error instanceof Error ? error.message : "Could not save lore.");
    } finally { setPending(false); }
  }
  return <section className="editor-lore-panel" aria-label="Series lore">
    <h3>Series lore</h3>
    {lore.status === "loading" ? <p role="status">Loading lore…</p> : null}
    {lore.status === "failed" ? <div role="alert">{lore.error}<button className="editor-button" onClick={lore.retry}>Retry lore</button></div> : null}
    {lore.status === "ready" ? <>
      {lore.entries.length === 0 ? <p>No lore yet.</p> : <ul className="editor-manuscript-tree">
        {lore.entries.map(entry => <li key={entry.id}><button type="button" disabled={disabled || pending}
          onClick={() => edit(entry)}>{entry.name} · {entry.kind}{entry.pinned ? " · pinned" : ""}</button></li>)}
      </ul>}
      <button className="editor-button" disabled={disabled || pending} onClick={() => edit()}>New lore</button>
    </> : null}
    {draft ? <form onSubmit={event => { event.preventDefault(); void save(); }}>
      <fieldset disabled={disabled || pending}>
        <legend>{editingId === null ? "Create lore" : "Edit lore"}</legend>
        <label className="editor-field"><span>Kind</span><select className="editor-input" value={draft.kind}
          onChange={event => setDraft({ ...draft, kind: event.target.value as LoreInput["kind"] })}>
          {["character", "place", "term", "note"].map(kind => <option key={kind}>{kind}</option>)}
        </select></label>
        <label className="editor-field"><span>Name</span><input className="editor-input" required value={draft.name}
          onChange={event => setDraft({ ...draft, name: event.target.value })} /></label>
        {(["keywords", "aliases"] as const).map(field => <label className="editor-field" key={field}>
          <span>{field === "keywords" ? "Keywords" : "Aliases"} (one per line)</span>
          <textarea className="editor-textarea" rows={2} defaultValue={draft[field].join("\n")} key={`${editingId}-${field}`}
            onChange={event => setDraft({ ...draft, [field]: parseLoreTerms(event.target.value) })} />
        </label>)}
        <label className="editor-field"><span>Visible text (AI context)</span><textarea className="editor-textarea" rows={3}
          value={draft.visible_text} onChange={event => setDraft({ ...draft, visible_text: event.target.value })} /></label>
        <label className="editor-field"><span>Hidden notes — author-only, never sent to AI</span><textarea className="editor-textarea" rows={3}
          value={draft.hidden_notes} onChange={event => setDraft({ ...draft, hidden_notes: event.target.value })} /></label>
        <label className="editor-field"><span>In AI requests</span><select className="editor-input" value={draft.level}
          onChange={event => setDraft({ ...draft, level: event.target.value as LoreInput["level"] })}>
          <option value="include">Include</option><option value="known_dont_reveal">Known, don't reveal</option><option value="leave_out">Leave out</option>
        </select></label>
        <label className="editor-toggle"><input type="checkbox" checked={draft.pinned}
          onChange={event => setDraft({ ...draft, pinned: event.target.checked })} /><span>Pin in AI context</span></label>
        <div className="editor-actions">
          <button className="editor-button" type="submit" disabled={!draft.name.trim()}>Save lore</button>
          <button className="editor-button" type="button" onClick={() => setDraft(null)}>Cancel lore</button>
          {editingId !== null ? <button className="editor-button" type="button" onClick={() => void save(true)}>Delete lore</button> : null}
        </div>
      </fieldset>
    </form> : null}
    {error ? <p role="alert">{error}</p> : null}
  </section>;
}
