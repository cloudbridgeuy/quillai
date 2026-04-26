export type WindowSelection = {
  index: number;
  length: number;
};

export type DocumentWindow = {
  start: number;
  end: number;
};

export type DocumentWindowMode = "window" | "document";

export type CalculateDocumentWindowInput = {
  documentLength: number;
  selection?: WindowSelection | null;
  windowSize: number;
  mode?: DocumentWindowMode;
};

export function calculateDocumentWindow({
  documentLength,
  selection,
  windowSize,
  mode = "window",
}: CalculateDocumentWindowInput): DocumentWindow {
  const normalizedDocumentLength = clampInteger(documentLength, 0);

  if (mode === "document") {
    return { start: 0, end: normalizedDocumentLength };
  }

  if (normalizedDocumentLength === 0) {
    return { start: 0, end: 0 };
  }

  const normalizedSelection = normalizeSelection(normalizedDocumentLength, selection);
  const { focusStart, focusEnd } = getFocusRange(
    normalizedDocumentLength,
    normalizedSelection,
  );
  const focusLength = focusEnd - focusStart;
  const effectiveWindowSize = getEffectiveWindowSize(
    normalizedDocumentLength,
    windowSize,
    focusLength,
  );
  const focusCenter =
    focusLength === 0 ? focusStart : (focusStart + focusEnd) / 2;
  const unclampedStart = Math.floor(focusCenter - effectiveWindowSize / 2);
  const start = clampInteger(
    unclampedStart,
    0,
    normalizedDocumentLength - effectiveWindowSize,
  );

  return {
    start,
    end: start + effectiveWindowSize,
  };
}

function getFocusRange(
  documentLength: number,
  selection: WindowSelection | null,
): { focusStart: number; focusEnd: number } {
  if (!selection) {
    return {
      focusStart: documentLength,
      focusEnd: documentLength,
    };
  }

  return {
    focusStart: selection.index,
    focusEnd: selection.index + selection.length,
  };
}

function getEffectiveWindowSize(
  documentLength: number,
  windowSize: number,
  focusLength: number,
): number {
  return Math.min(
    documentLength,
    Math.max(clampInteger(windowSize, 0), focusLength, 1),
  );
}

function normalizeSelection(
  documentLength: number,
  selection?: WindowSelection | null,
): WindowSelection | null {
  if (!selection) {
    return null;
  }

  const index = clampInteger(selection.index, 0, documentLength);
  const maxLength = documentLength - index;
  const length = clampInteger(selection.length, 0, maxLength);

  return { index, length };
}

function clampInteger(value: number, min: number, max = Number.MAX_SAFE_INTEGER): number {
  if (!Number.isFinite(value)) {
    return min;
  }

  return Math.min(max, Math.max(min, Math.trunc(value)));
}
