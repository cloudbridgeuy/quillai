export const TIPS = [
  "Press Ctrl/Cmd+Enter to open Continue.",
  "Press Ctrl/Cmd+K to open Edit.",
  "Press Alt+L to open the nearest lore match.",
  "Open Settings, then AI, to configure your provider.",
  "Open the manuscript drawer to export Markdown, text, Word, or EPUB.",
];

export const TIPS_DISMISSED_KEY = "quillai.landing.tipsDismissed.v1";

type TipsStorage = Pick<Storage, "getItem" | "setItem">;

export function readTipsDismissed(storage?: TipsStorage): boolean {
  try { return (storage ?? window.localStorage).getItem(TIPS_DISMISSED_KEY) === "true"; } catch { return false; }
}

export function dismissTips(storage?: TipsStorage): void {
  try { (storage ?? window.localStorage).setItem(TIPS_DISMISSED_KEY, "true"); } catch {}
}
