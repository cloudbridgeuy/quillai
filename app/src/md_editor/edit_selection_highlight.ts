import type Quill from "quill";

export function editSelectionBounds(selection: { index: number; length: number } | null, documentLength: number) {
  if (!selection || !Number.isSafeInteger(documentLength) || !Number.isSafeInteger(selection.index)
    || !Number.isSafeInteger(selection.length) || selection.index < 0 || selection.length <= 0
    || selection.index + selection.length > documentLength) return null;
  return { start: selection.index, end: selection.index + selection.length };
}

export function renderEditSelectionHighlight(quill: Quill, selection: { index: number; length: number } | null) {
  if (typeof CSS === "undefined" || !CSS.highlights || typeof Highlight === "undefined") return;
  const bounds = editSelectionBounds(selection, quill.getLength());
  if (!bounds) {
    CSS.highlights.delete("quillai-edit-selection");
    return;
  }
  const [first, firstOffset] = quill.getLeaf(bounds.start);
  const [last, lastOffset] = quill.getLeaf(bounds.end);
  if (!first || !last) {
    CSS.highlights.delete("quillai-edit-selection");
    return;
  }
  const [startNode, startOffset] = first.position(firstOffset, false);
  const [endNode, endOffset] = last.position(lastOffset, false);
  const range = document.createRange();
  range.setStart(startNode, startOffset);
  range.setEnd(endNode, endOffset);
  CSS.highlights.set("quillai-edit-selection", new Highlight(range));
}

export function trackEditSelectionHighlight(quill: Quill, selection: { index: number; length: number } | null) {
  renderEditSelectionHighlight(quill, selection);
  const onSelection = (next: { index: number; length: number } | null, _previous: unknown, source: string) => {
    if (next && source === "user") renderEditSelectionHighlight(quill, next);
  };
  const onEditorChange = (event: string) => {
    if (event === "text-change") renderEditSelectionHighlight(quill, null);
  };
  quill.on("selection-change", onSelection);
  quill.on("editor-change", onEditorChange);
  return () => {
    quill.off("selection-change", onSelection);
    quill.off("editor-change", onEditorChange);
    renderEditSelectionHighlight(quill, null);
  };
}
