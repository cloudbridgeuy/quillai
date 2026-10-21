export function selectionBoundary(text: string, index: number): number {
  const before = text.charCodeAt(index - 1);
  const after = text.charCodeAt(index);
  return before >= 0xd800 && before <= 0xdbff && after >= 0xdc00 && after <= 0xdfff ? index - 1 : index;
}
