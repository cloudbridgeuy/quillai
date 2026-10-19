import { useEffect, useRef, useState, type RefObject } from "react";
import type Quill from "quill";
import { createManuscriptClient } from "../manuscripts/client";
import { bootstrapEditorChapter, createChapterSession, loadEditorChapter, NoEditorChapter } from "./chapter_session";
import { chapterDelta } from "./chapter_sync";

const RECOVERY_KEY = "quillai.chapter.recovery.v1";

export function useChapterSync(
  quillRef: RefObject<Quill | null>,
  isAiActive: () => boolean,
  onRemoteChange: () => void,
) {
  const [status, setStatus] = useState("Loading chapter…");
  const [ready, setReady] = useState(false);
  const [blocked, setBlocked] = useState(false);
  const [recovery, setRecovery] = useState<string | null>(null);
  const [needsChapter, setNeedsChapter] = useState(false);
  const sessionRef = useRef<ReturnType<typeof createChapterSession> | null>(null);
  const applyingRef = useRef(false);
  const dirtyRef = useRef(false);
  const activeRef = useRef(isAiActive);
  const remoteRef = useRef(onRemoteChange);
  activeRef.current = isAiActive;
  remoteRef.current = onRemoteChange;

  useEffect(() => {
    let disposed = false;
    let storageFailed = false;
    try {
      const previous = window.sessionStorage.getItem(RECOVERY_KEY);
      if (previous) {
        window.sessionStorage.setItem(`${RECOVERY_KEY}.${crypto.randomUUID()}`, previous);
        window.sessionStorage.removeItem(RECOVERY_KEY);
      }
      const copies: string[] = [];
      for (let index = 0; index < window.sessionStorage.length; index++) {
        const key = window.sessionStorage.key(index);
        if (key?.startsWith(`${RECOVERY_KEY}.`)) {
          const copy = window.sessionStorage.getItem(key);
          if (copy) copies.push(copy);
        }
      }
      setRecovery(copies.length ? JSON.stringify(copies) : null);
    } catch {
      storageFailed = true;
    }
    const client = createManuscriptClient();
    void loadEditorChapter(client, window.location.search).then(({ scope, chapter }) => {
      const quill = quillRef.current;
      if (disposed || !quill) return;
      const url = new URL(window.location.href);
      for (const [key, value] of Object.entries(scope)) url.searchParams.set(key, String(value));
      window.history.replaceState(null, "", url);
      applyingRef.current = true;
      quill.setContents(chapterDelta(chapter.content), "api");
      quill.history.clear();
      applyingRef.current = false;
      remoteRef.current();
      const session = createChapterSession(client, scope, chapter, (document, dirty, problem) => {
        if (disposed) return;
        dirtyRef.current = dirty;
        try {
          if (dirty) {
            window.sessionStorage.setItem(RECOVERY_KEY, JSON.stringify({ scope, document }));
          } else if (window.sessionStorage.getItem(RECOVERY_KEY) === latestRecovery) {
            window.sessionStorage.removeItem(RECOVERY_KEY);
          }
          latestRecovery = dirty ? window.sessionStorage.getItem(RECOVERY_KEY) : null;
        } catch {
          storageFailed = true;
        }
        const change = quill.getContents().diff(document);
        if (change.ops.length > 0) {
          const selection = quill.getSelection();
          applyingRef.current = true;
          quill.updateContents(change, "api");
          if (selection) {
            const index = change.transformPosition(selection.index, true);
            const end = change.transformPosition(selection.index + selection.length, true);
            quill.setSelection(index, Math.max(0, end - index), "silent");
          }
          applyingRef.current = false;
          remoteRef.current();
        }
        setBlocked(problem?.kind === "blocked");
        setStatus(problem?.message ?? `${chapter.title} · ${dirty ? "Unsaved changes" : "Saved"}${storageFailed ? " · Recovery storage unavailable; export before closing" : ""}`);
        if (quill.getContents().diff(document).ops.length > 0) session.edit(quill.getContents());
      });
      let latestRecovery: string | null = null;
      sessionRef.current = session;
      setStatus(`${chapter.title} · Saved${storageFailed ? " · Recovery storage unavailable" : ""}`);
      setReady(true);
      session.edit(quill.getContents());
    }).catch((error: unknown) => {
      if (!disposed) {
        setStatus(error instanceof Error ? error.message : "Chapter load failed.");
        setNeedsChapter(error instanceof NoEditorChapter);
      }
    });
    const timer = window.setInterval(() => {
      if (!activeRef.current()) void sessionRef.current?.tick();
    }, 1000);
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (dirtyRef.current) {
        event.preventDefault();
        event.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", beforeUnload);
    return () => {
      disposed = true;
      window.clearInterval(timer);
      window.removeEventListener("beforeunload", beforeUnload);
      sessionRef.current?.stop();
      sessionRef.current = null;
    };
  }, [quillRef]);

  return {
    ready,
    blocked,
    status,
    recovery,
    needsChapter,
    async bootstrap() {
      setNeedsChapter(false);
      setStatus("Creating chapter…");
      try {
        const scope = await bootstrapEditorChapter(createManuscriptClient());
        const url = new URL(window.location.href);
        for (const [key, value] of Object.entries(scope)) url.searchParams.set(key, String(value));
        window.location.assign(url);
      } catch (error) {
        setStatus(error instanceof Error ? error.message : "Chapter creation failed.");
        setNeedsChapter(true);
      }
    },
    canStartAi: () => sessionRef.current?.canStartAi() ?? false,
    documentChanged() {
      if (!applyingRef.current && quillRef.current) {
        sessionRef.current?.edit(quillRef.current.getContents());
      }
    },
    exportDocument: () => JSON.stringify({ document: quillRef.current?.getContents() }),
  };
}
