export function getTabTitle(chapterTitle: string | null, ready: boolean): string {
  return ready && chapterTitle !== null ? `${chapterTitle} — QuillAI` : "QuillAI";
}
