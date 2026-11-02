const DELAY_MS = 300;

export function showChapterBusy(start: number, now: number): boolean {
  return now - start >= DELAY_MS;
}

export function scheduleChapterBusy(start: number, show: () => void, schedule = setTimeout, cancel = clearTimeout, now = Date.now): () => void {
  const timer = schedule(() => { if (showChapterBusy(start, now())) show(); }, Math.max(0, start + DELAY_MS - now()));
  return () => cancel(timer);
}
