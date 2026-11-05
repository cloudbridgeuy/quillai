/** Start of the match after `current` (wrapping), or the first when `current` is null or no longer present. */
export function nextOccurrence(starts: readonly number[], current: number | null): number | undefined {
  const sorted = [...starts].sort((a, b) => a - b);
  const index = current === null ? -1 : sorted.indexOf(current);
  return index === -1 ? sorted[0] : sorted[(index + 1) % sorted.length];
}
