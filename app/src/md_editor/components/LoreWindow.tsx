import { defaultLocale, formatMessage } from "../../i18n/messages";
import type Quill from "quill";
import { useLayoutEffect, useState } from "react";
import { Heading } from "../../ui/Heading";
import { Popover } from "../../ui/Popover";
import { ListButton } from "../../ui/ListButton";
import type { buildInjectedLore } from "../context_inspector";
import { loreLevel } from "./lore_entry_row";
import { nextOccurrence } from "./lore_occurrence";

type Lore = ReturnType<typeof buildInjectedLore>;
type Mark = { entry: Lore["matches"][number]; start: number; range: globalThis.Range };

export function supportsLoreHighlights(css: { highlights?: unknown } | undefined, highlight: unknown): boolean {
  return css?.highlights != null && typeof highlight === "function";
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
    const highlightsSupported = supportsLoreHighlights(
      typeof CSS === "undefined" ? undefined : CSS,
      typeof Highlight === "undefined" ? undefined : Highlight,
    );
    if (highlightsSupported) {
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
    const measure = () => { if (selected) redraw(value => value + 1); };
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
      if (highlightsSupported) {
        for (const level of ["include", "known_dont_reveal", "leave_out"]) CSS.highlights.delete(`quillai-lore-${level}`);
        CSS.highlights.delete("quillai-lore-hover");
      }
    };
  });

  const rect = selected?.range.getBoundingClientRect();
  return <section className="editor-preview-section" aria-label={formatMessage(defaultLocale, "lore.window.title")}>
    <Heading level="section" as="h3">{formatMessage(defaultLocale, "lore.window.title")}</Heading>
    <p className="editor-note">{formatMessage(defaultLocale, "lore.window.context")}</p>
    {lore.injected.length === 0 ? <p>{formatMessage(defaultLocale, "lore.window.empty")}</p> : <ul className="editor-lore-window">
      {lore.injected.map(entry => {
        const entryMarks = marks.filter(match => match.entry.id === entry.id);
        const open = selected?.entry.id === entry.id && !!rect;
        return <li key={entry.id}><Popover open={open} anchor={{ left: rect?.left ?? 0, top: rect?.bottom ?? 0 }}
          onOpenChange={next => { if (!next) setActive(null); }}
          role="dialog" aria-label={formatMessage(defaultLocale, "lore.window.match")} className="editor-lore-card"
          trigger={<ListButton tone="lore" onMouseDown={event => event.preventDefault()} onClick={event => {
            event.preventDefault();
            const start = nextOccurrence(entryMarks.map(match => match.start), active?.id === entry.id ? active.start : null);
            const mark = entryMarks.find(match => match.start === start);
            if (!mark) return;
            window.scrollBy({ top: mark.range.getBoundingClientRect().top - window.innerHeight / 2 });
            setActive({ id: entry.id, start: mark.start });
          }} disabled={entry.occurrences.length === 0}>
            <strong>{entry.name}</strong><span>{formatMessage(defaultLocale, "lore.window.matches", { count: entry.occurrences.length, countText: String(entry.occurrences.length) })}</span>
            <small>{entry.kind}{entry.reason === "pinned" ? formatMessage(defaultLocale, "lore.pinned") : ""}</small><small>{loreLevel(entry.level)}</small>
          </ListButton>}>
          {selected ? card(selected) : null}
        </Popover></li>;
      })}
    </ul>}
    {selected && rect && !lore.injected.some(entry => entry.id === selected.entry.id) ? <Popover open anchor={{ left: rect.left, top: rect.bottom }}
      onOpenChange={open => { if (!open) setActive(null); }} role="dialog" aria-label={formatMessage(defaultLocale, "lore.window.match")} className="editor-lore-card">
      {card(selected)}
    </Popover> : null}
  </section>;
}

function card(selected: Mark) {
  return <>
    <span className="editor-lore-flag">{formatMessage(defaultLocale, "lore.window.match")}</span>
    <h3>{selected.entry.name}</h3>
    <p>{selected.entry.kind} · {loreLevel(selected.entry.level)}</p>
    <p className="editor-lore-visible">{selected.entry.visible_text || formatMessage(defaultLocale, "lore.window.noVisible")}</p>
    <small>{formatMessage(defaultLocale, "lore.window.shortcut")}</small>
  </>;
}
