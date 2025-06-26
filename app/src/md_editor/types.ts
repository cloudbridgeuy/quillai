import type { Delta, EmitterSource, Range } from "quill";
export type {
  Delta as DeltaT,
  EmitterSource as EmitterSourceT,
  Range as RangeT,
};
export type OnTextChangeFn = (
  delta: Delta,
  oldDelta: Delta,
  source: EmitterSource,
) => void;
export type OnSelectionChangeFn = (
  range: Range | null,
  oldRange: Range | null,
  source: EmitterSource,
) => void;
