import diff from "fast-diff";
import Delta from "quill-delta";

export function diffDocuments(before: Delta, after: Delta): Delta {
  const text = (document: Delta) => document.ops.map((op) => {
    if (op.insert == null) throw new Error("Document diff requires insert operations.");
    return typeof op.insert === "string" ? op.insert : "\0";
  }).join("");
  const change = new Delta();
  let index = 0;
  for (const [kind, value] of diff(text(before), text(after), undefined, false)) {
    if (kind === diff.DELETE) {
      change.delete(value.length);
      continue;
    }
    if (kind === diff.INSERT) {
      for (const op of after.slice(index, index + value.length).ops) change.push(op);
    } else {
      change.retain(value.length);
    }
    index += value.length;
  }
  const normalized = change.chop();
  return normalized.compose(before.compose(normalized).diff(after));
}
