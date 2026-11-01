import type Quill from "quill";
import Delta from "quill-delta";
import { continueOwnedRanges } from "./continue_stream";

export function editTintRemoval(anchor: number, end: number, enabled: boolean) {
  return enabled && end > anchor ? new Delta().retain(anchor).delete(end - anchor) : null;
}

export function renderContinueTint(quill: Quill, removal: Delta, enabled: boolean) {
  if (typeof CSS === "undefined" || !CSS.highlights || typeof Highlight === "undefined") return;
  const ranges = enabled ? continueOwnedRanges(removal).flatMap(({ start, end }) => {
    const [first, firstOffset] = quill.getLeaf(start);
    const [last, lastOffset] = quill.getLeaf(end);
    if (!first || !last) return [];
    const [startNode, startOffset] = first.position(firstOffset, false);
    const [endNode, endOffset] = last.position(lastOffset, false);
    const range = document.createRange();
    range.setStart(startNode, startOffset);
    range.setEnd(endNode, endOffset);
    return [range];
  }) : [];
  CSS.highlights.set("quillai-stream", new Highlight(...ranges));
}
