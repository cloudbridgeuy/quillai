import type Quill from "quill";
import { useLayoutEffect, useState } from "react";
import { createPortal } from "react-dom";
import type { buildInjectedLore } from "../context_inspector";

type Lore = ReturnType<typeof buildInjectedLore>;
type Mark = { entry: Lore["matches"][number]; start: number; range: globalThis.Range };

function loreLevel(level: string) {
  return level === "known_dont_reveal" ? "Known, don't reveal" : level === "leave_out" ? "Leave out" : "Include";
}

export function LoreWindow({ quill, start, lore }: { quill: Quill; start: number; lore: Lore }) {
  const [active, setActive] = useState<{ id: number; start: number } | null>(null);
  const [, redraw] = useState(0);
  const marks: Mark[] = lore.matches.flatMap(entry => entry.occurrences.flatMap(occurrence => {
    const [first, firstOffset] = quill.getLeaf(start + occurrence.start);
    const [last, lastOffset] = quill.getLeaf(start + occurrence.end);
    if (!first || !last) return [];
    const range = document.createRange();
    const [firstNode, firstPosition] = first.position(firstOffset, false);
    const [lastNode, lastPosition] = last.position(lastOffset, false);
    range.setStart(firstNode, firstPosition);
    range.setEnd(lastNode, lastPosition);
    return [{ entry, start: occurrence.start, range }];
  }));
  const selected = marks.find(mark => mark.entry.id === active?.id && mark.start === active.start);

  useLayoutEffect(() => {
    if (typeof Highlight !== "undefined" && CSS.highlights) {
      for (const level of ["include", "known_dont_reveal", "leave_out"]) {
        CSS.highlights.set(`quillai-lore-${level}`, new Highlight(...marks.filter(mark => mark.entry.level === level).map(mark => mark.range)));
      }
      CSS.highlights.set("quillai-lore-hover", new Highlight(...(selected ? [selected.range] : [])));
    }
    let timer: ReturnType<typeof setTimeout> | undefined;
    let pending: Mark | undefined;
    const leave = () => { clearTimeout(timer); pending = undefined; };
    const move = (event: PointerEvent) => {
      const match = marks.find(mark => [...mark.range.getClientRects()].some(rect => event.clientX >= rect.left && event.clientX <= rect.right && event.clientY >= rect.top && event.clientY <= rect.bottom));
      if (match === pending) return;
      leave();
      pending = match;
      if (match) timer = setTimeout(() => setActive({ id: match.entry.id, start: match.start }), 600);
    };
    const dismiss = () => setActive(null);
    const measure = () => redraw(value => value + 1);
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") dismiss();
      if (event.altKey && event.key.toLowerCase() === "l") {
        const caret = (quill.getSelection()?.index ?? start) - start;
        const nearest = [...marks].sort((a, b) => Math.abs(a.start - caret) - Math.abs(b.start - caret))[0];
        if (nearest) {
          event.preventDefault();
          setActive({ id: nearest.entry.id, start: nearest.start });
        }
      }
    };
    quill.root.addEventListener("pointermove", move);
    quill.root.addEventListener("pointerleave", leave);
    window.addEventListener("keydown", key);
    window.addEventListener("scroll", measure, true);
    window.addEventListener("resize", measure);
    return () => {
      leave();
      quill.root.removeEventListener("pointermove", move);
      quill.root.removeEventListener("pointerleave", leave);
      window.removeEventListener("keydown", key);
      window.removeEventListener("scroll", measure, true);
      window.removeEventListener("resize", measure);
      if (typeof CSS !== "undefined" && CSS.highlights) {
        for (const level of ["include", "known_dont_reveal", "leave_out"]) CSS.highlights.delete(`quillai-lore-${level}`);
        CSS.highlights.delete("quillai-lore-hover");
      }
    };
  });

  const rect = selected?.range.getBoundingClientRect();
  return <section className="editor-preview-section" aria-label="Lore in window">
    <h3>Lore in window</h3>
    <p className="editor-note">Continue context · includes pins · excludes leave out and budget skips.</p>
    {lore.injected.length === 0 ? <p>No lore in window.</p> : <ul className="editor-lore-window">
      {lore.injected.map(entry => <li key={entry.id}><button type="button"
        onMouseDown={event => event.preventDefault()} disabled={entry.occurrences.length === 0}
        onClick={() => {
          const mark = marks.find(match => match.entry.id === entry.id);
          if (!mark) return;
          window.scrollBy({ top: mark.range.getBoundingClientRect().top - window.innerHeight / 2 });
          setActive({ id: mark.entry.id, start: mark.start });
        }}>
        <strong>{entry.name}</strong><span>{entry.occurrences.length} matches</span>
        <small>{entry.kind}{entry.reason === "pinned" ? " · pinned" : ""}</small><small>{loreLevel(entry.level)}</small>
      </button></li>)}
    </ul>}
    {selected && rect ? createPortal(<div role="dialog" aria-label="Lore match" className="editor-lore-card"
      style={{ left: Math.max(12, Math.min(rect.left, window.innerWidth - 324)), top: Math.max(12, Math.min(rect.bottom + 8, window.innerHeight - 300)) }}>
      <span className="editor-lore-flag">Lore match</span>
      <button type="button" aria-label="Close lore match" onClick={() => setActive(null)}>×</button>
      <h3>{selected.entry.name}</h3>
      <p>{selected.entry.kind} · {loreLevel(selected.entry.level)}</p>
      <p className="editor-lore-visible">{selected.entry.visible_text || "No visible text."}</p>
      <small>Alt+L opens · Esc closes</small>
    </div>, document.body) : null}
  </section>;
}
