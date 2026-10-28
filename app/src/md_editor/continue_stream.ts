import Delta from "quill-delta";

export type ContinueStream = {
  anchor: number;
  end: number;
  started: boolean;
  paragraph: boolean;
  removal: Delta;
};

export function transformContinueSelection(selection: { index: number; length: number }, change: Delta) {
  const end = change.transformPosition(selection.index + selection.length, true);
  const deletions = new Delta(change.ops.filter((op) => op.insert === undefined));
  const fullyDeleted = selection.length > 0 &&
    deletions.transformPosition(selection.index) === deletions.transformPosition(selection.index + selection.length);
  const index = fullyDeleted ? end : change.transformPosition(selection.index, true);
  return { index, length: Math.max(0, end - index) };
}

export function continuePlacement(text: string, caret: number, first: string) {
  const lineStart = caret === 0 ? 0 : text.lastIndexOf("\n", caret - 1) + 1;
  const newline = text.indexOf("\n", caret);
  const lineEnd = newline < 0 ? text.length : newline;
  const before = text.slice(lineStart, caret);
  const after = text.slice(caret, lineEnd);
  const paragraph = !text.slice(lineStart, lineEnd).trim() || (!after.trim() && /[.!?]["'”’]*\s*$/u.test(before));
  const left = text[caret - 1];
  const right = text[caret];
  const space = !!left && !/\s/u.test(left) && (!right || /\s/u.test(right)) && !!first && !/^[\s\p{P}]/u.test(first);
  return { paragraph, space, lineEnd };
}

export function createContinueStream(text: string, caret: number): ContinueStream {
  const { paragraph, lineEnd } = continuePlacement(text, caret, "");
  const anchor = paragraph ? lineEnd : caret;
  return { anchor, end: anchor, started: false, paragraph, removal: new Delta() };
}

export function transformContinueStream(state: ContinueStream, change: Delta): ContinueStream {
  return {
    ...state,
    anchor: change.transformPosition(state.anchor),
    end: change.transformPosition(state.end),
    removal: change.transform(state.removal, true).chop(),
  };
}

export function continueOwnedRanges(removal: Delta): Array<{ start: number; end: number }> {
  let position = 0;
  return removal.ops.flatMap((op) => {
    const start = position;
    position += typeof op.retain === "number" ? op.retain : op.delete ?? 0;
    return op.delete ? [{ start, end: position }] : [];
  });
}

export function appendContinueFragment(state: ContinueStream, fragment: Delta, text: string) {
  if (fragment.length() === 0) return { state, change: new Delta() };
  const first = fragment.ops[0]?.insert;
  const space = continuePlacement(text, state.end, typeof first === "string" ? first : "").space;
  const prefix = state.started ? "" : state.paragraph ? "\n" : space ? " " : "";
  const inserted = (prefix ? new Delta().insert(prefix) : new Delta()).concat(fragment);
  const change = new Delta().retain(state.end).concat(inserted);
  const removal = new Delta([...state.removal.ops]);
  const consumed = continueOwnedRanges(removal).at(-1)?.end ?? 0;
  removal.retain(state.end - consumed).delete(inserted.length());
  return {
    change,
    state: { ...state, started: true, end: state.end + inserted.length(), removal },
  };
}
