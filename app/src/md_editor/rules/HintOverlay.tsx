import type Quill from "quill";
import { useLayoutEffect, useState } from "react";
import { createPortal } from "react-dom";
import { HintUnderline } from "../../ui/HintUnderline";
import type { Hint } from "./engine";
import { hintSegments, type Rect } from "./hint_geometry";

const sameRects = (a: Rect[], b: Rect[]) => JSON.stringify(a) === JSON.stringify(b);

export function HintOverlay({ quill, hints }: { quill: Quill; hints: readonly Hint[] }) {
  const [rects, setRects] = useState<Rect[]>([]);

  useLayoutEffect(() => {
    const measure = () => {
      const text = quill.getText();
      const container = quill.container.getBoundingClientRect();
      const visible = { top: -container.top, bottom: window.innerHeight - container.top };
      const next = hints.flatMap((hint) => hintSegments(hint, text, (index, length) => quill.getBounds(index, length), visible));
      setRects((previous) => (sameRects(previous, next) ? previous : next));
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
  }, [quill, hints]);

  return createPortal(rects.map((rect) => <HintUnderline key={`${rect.top}:${rect.left}`} {...rect} />), quill.container);
}
