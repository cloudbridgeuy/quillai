import type Quill from "quill";
import type { EditorScope } from "../manuscripts/contract";
import { threadScopeKey } from "./ai_thread";
import { selectionBoundary } from "./selection_boundary";

export type CaretPosition = { index: number; length: number; offsetTop: number };
const KEY = "quillai.editor.position.v1";

function position(value: unknown): CaretPosition | null {
  if (!value || typeof value !== "object") return null;
  const { index, length, offsetTop } = value as CaretPosition;
  return Number.isSafeInteger(index) && index >= 0 && Number.isSafeInteger(length) && length >= 0
    && Number.isFinite(offsetTop) && offsetTop >= 0 ? { index, length, offsetTop } : null;
}

export function safeCaretPosition(value: unknown, text: string): CaretPosition | null {
  const saved = position(value);
  if (!saved) return null;
  const max = Math.max(0, text.length - 1);
  const index = selectionBoundary(text, Math.min(saved.index, max));
  const end = selectionBoundary(text, Math.min(saved.index + saved.length, max));
  return { index, length: Math.max(0, end - index), offsetTop: saved.offsetTop };
}

function entries(storage: Storage): Record<string, unknown> {
  try {
    const value = JSON.parse(storage.getItem(KEY) ?? "{}");
    return value && typeof value === "object" && !Array.isArray(value) ? value : {};
  } catch { return {}; }
}

export function readCaretPosition(key: string, storage?: Storage): CaretPosition | null {
  try {
    const values = entries(storage ?? window.localStorage);
    return Object.hasOwn(values, key) ? position(values[key]) : null;
  } catch { return null; }
}

export function writeCaretPosition(key: string, value: CaretPosition, storage?: Storage): void {
  try {
    const target = storage ?? window.localStorage;
    const valid = position(value);
    if (valid) target.setItem(KEY, JSON.stringify({ ...entries(target), [key]: valid }));
  } catch {}
}

export function createCaretTracker(quill: Quill) {
  let key: string | null = null;
  let range: { index: number; length: number } | null = null;
  let timer: number | undefined;
  let pending: CaretPosition | null = null;
  const previousRestoration = window.history.scrollRestoration;
  function apply(saved: CaretPosition) {
    const safe = safeCaretPosition(saved, quill.getText());
    if (!safe) return;
    quill.setSelection(safe.index, safe.length, "silent");
    range = { index: safe.index, length: safe.length };
    const bounds = quill.getBounds(safe.index, 0);
    if (bounds) window.scrollBy(0, quill.container.getBoundingClientRect().top + bounds.top - safe.offsetTop);
  }
  function flush() {
    window.clearTimeout(timer);
    const current = quill.getSelection() ?? range;
    if (!key || !current) return;
    const safe = safeCaretPosition({ ...current, offsetTop: 0 }, quill.getText());
    const bounds = safe && quill.getBounds(safe.index, 0);
    if (!safe || !bounds) return;
    range = { index: safe.index, length: safe.length };
    const offsetTop = Math.max(0, quill.container.getBoundingClientRect().top + bounds.top);
    writeCaretPosition(key, { ...safe, offsetTop });
  }
  function schedule() {
    window.clearTimeout(timer);
    timer = window.setTimeout(flush, 250);
  }
  function selection(next: { index: number; length: number } | null) {
    if (next) { range = next; schedule(); }
  }
  quill.on("selection-change", selection);
  window.addEventListener("scroll", schedule, { passive: true });
  return {
    open(scope: EditorScope, restore = true) {
      window.clearTimeout(timer);
      key = threadScopeKey(scope);
      range = null;
      pending = restore ? readCaretPosition(key) : null;
      window.history.scrollRestoration = pending ? "manual" : previousRestoration;
      if (pending) apply(pending);
      return pending !== null;
    },
    finish() {
      if (pending) apply({ ...pending, ...(quill.getSelection() ?? range) });
      pending = null;
    },
    flush,
    close() {
      flush();
      quill.off("selection-change", selection);
      window.removeEventListener("scroll", schedule);
      window.history.scrollRestoration = previousRestoration;
    },
  };
}
