import type Quill from "quill";
import { useLayoutEffect, useState } from "react";
import { createPortal } from "react-dom";
import { Button } from "../../ui/Button";

const FLAG_MIN_LEFT = 64;
const FLAG_OFFSET = 210;

type Geometry = { top: number; bottom: number; viewportTop: number; left: number };

export function ContinueWindow({ quill, start, end }: { quill: Quill; start: number; end: number }) {
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
    {geometry.viewportTop < 0 ? createPortal(<Button size="sm" className="editor-continue-window-flag"
      style={{ left: Math.max(FLAG_MIN_LEFT, geometry.left - FLAG_OFFSET) }}
      onMouseDown={(event) => event.preventDefault()}
      onClick={() => window.scrollBy({ top: geometry.viewportTop - 24, behavior: "smooth" })}>
      Continue window
      <small>starts ↑ {screensAbove < 1 ? "just above" : `${Math.round(screensAbove)} pages up`}</small>
    </Button>, document.body) : null}
  </>;
}
