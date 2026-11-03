import { useEffect, useState, type RefObject } from "react";
import type Quill from "quill";
import { countText, formatTextCounts } from "../text_counts";

export function TextCounts({ quillRef }: { quillRef: RefObject<Quill | null> }) {
  const [counts, setCounts] = useState(() => countText("\n"));
  const [selectedWords, setSelectedWords] = useState<number | null>(null);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;
    let quill: Quill | null = null;
    const updateSelection = () => {
      if (!quill) return;
      const selection = quill.getSelection();
      setSelectedWords(selection && selection.length > 0
        ? countText(quill.getContents(selection.index, selection.length).ops, false).words
        : null);
    };
    const update = () => {
      if (!quill) return;
      setCounts(countText(quill.getContents().ops));
      updateSelection();
    };
    const schedule = () => {
      clearTimeout(timer);
      timer = setTimeout(update, 300);
    };
    const attach = setTimeout(() => {
      quill = quillRef.current;
      if (!quill) return;
      quill.on("text-change", schedule);
      quill.on("selection-change", updateSelection);
      update();
    }, 0);
    return () => {
      clearTimeout(attach);
      clearTimeout(timer);
      quill?.off("text-change", schedule);
      quill?.off("selection-change", updateSelection);
    };
  }, [quillRef]);

  return <p className="editor-note" aria-label="Document counts" aria-live="off">
    {formatTextCounts(counts, selectedWords)}
  </p>;
}
