import type Quill from "quill";
import { useLayoutEffect, useState } from "react";
import { createPortal } from "react-dom";
import { Button } from "../../ui/Button";
import { defaultLocale, formatMessage } from "../../i18n/messages";

const FLAG_MIN_LEFT = 64;
const FLAG_OFFSET = 210;

type Geometry = { top: number; bottom: number; viewportTop: number; left: number };

export function ContinueWindow({ quill, start, end, modelName, modelId, onModelClick }: {
  quill: Quill; start: number; end: number; modelName: string; modelId?: string; onModelClick: () => void;
}) {
  const [geometry, setGeometry] = useState<Geometry | null>(null);

  useLayoutEffect(() => {
    const measure = () => {
      const first = quill.getBounds(start);
      const last = quill.getBounds(end);
      if (!first || !last) return;
      const container = quill.container.getBoundingClientRect();
      const next = { top: first.top, bottom: last.bottom, viewportTop: container.top + first.top, left: container.left };
      setGeometry((previous) => previous?.top === next.top && previous.bottom === next.bottom && previous.viewportTop === next.viewportTop && previous.left === next.left ? previous : next);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(quill.root);
    window.addEventListener("scroll", measure, true);
    window.addEventListener("resize", measure);
    return () => {
      observer.disconnect();
      window.removeEventListener("scroll", measure, true);
      window.removeEventListener("resize", measure);
    };
  });

  if (!geometry) return null;
  const screensAbove = -geometry.viewportTop / window.innerHeight;
  return <>
    {createPortal(<div className="editor-continue-window" aria-hidden="true"
      data-start={start} data-end={end}
      style={{ top: geometry.top, height: Math.max(0, geometry.bottom - geometry.top) }} />, quill.container)}
    {createPortal(<Button variant="link" size="sm" className="editor-continue-window-model text-micro" title={modelId}
      style={{ top: geometry.top }}
      onMouseDown={(event) => event.preventDefault()} onClick={onModelClick}>
      {formatMessage(defaultLocale, "editor.modelLink", { model: modelName })}
    </Button>, quill.container)}
    {geometry.viewportTop < 0 ? createPortal(<Button size="sm" className="editor-continue-window-flag"
      style={{ left: Math.max(FLAG_MIN_LEFT, geometry.left - FLAG_OFFSET) }}
      onMouseDown={(event) => event.preventDefault()}
      onClick={() => window.scrollBy({ top: geometry.viewportTop - 24, behavior: "smooth" })}>
      {formatMessage(defaultLocale, "editor.continueWindow")}
      <small>{formatMessage(defaultLocale, "editor.windowStarts", { position: screensAbove < 1 ? formatMessage(defaultLocale, "editor.justAbove") : formatMessage(defaultLocale, "editor.pagesUp", { pages: String(Math.round(screensAbove)) }) })}</small>
    </Button>, document.body) : null}
  </>;
}
