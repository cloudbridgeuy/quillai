import Delta from "quill-delta";

export type ContinueStream = {
  anchor: number;
  end: number;
  started: boolean;
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

export function createContinueStream(text: string, caret: number): ContinueStream {
  const newline = text.indexOf("\n", caret);
  const anchor = newline < 0 ? text.length : newline;
  return { anchor, end: anchor, started: false, removal: new Delta() };
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

export function appendContinueFragment(state: ContinueStream, fragment: Delta) {
  if (fragment.length() === 0) return { state, change: new Delta() };
  const inserted = (state.started ? new Delta() : new Delta().insert("\n")).concat(fragment);
  const change = new Delta().retain(state.end).concat(inserted);
  const removal = new Delta([...state.removal.ops]);
  const consumed = continueOwnedRanges(removal).at(-1)?.end ?? 0;
  removal.retain(state.end - consumed).delete(inserted.length());
  return {
    change,
    state: { ...state, started: true, end: state.end + inserted.length(), removal },
  };
}
