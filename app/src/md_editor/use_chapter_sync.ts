import { useEffect, useRef, useState, type RefObject } from "react";
import type Quill from "quill";
import { createManuscriptClient, ManuscriptApiError } from "../manuscripts/client";
import { bootstrapEditorChapter, canSwitchChapter, createChapterSession, loadEditorChapter, loadEditorDocument, NoEditorChapter, type EditorDocument } from "./chapter_session";
import type { EditorScope } from "../manuscripts/contract";
import { chapterDelta } from "./chapter_sync";
import { diffDocuments } from "./document_diff";
import { selectionBoundary } from "./selection_boundary";
import { LAST_EDITED_KEY, landingUrl } from "../landing/routes";
import { createSyncPush } from "./sync_push";

const RECOVERY_KEY = "quillai.chapter.recovery.v1";

export function useChapterSync(
  quillRef: RefObject<Quill | null>,
  isAiActive: () => boolean,
  onRemoteChange: () => void,
) {
  const [status, setStatus] = useState("Loading chapter…");
  const [detail, setDetail] = useState<string>();
  const [title, setTitle] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const [blocked, setBlocked] = useState(false);
  const [recovery, setRecovery] = useState<string | null>(null);
  const [needsChapter, setNeedsChapter] = useState(false);
  const [scope, setScope] = useState<EditorScope | null>(null);
  const sessionRef = useRef<ReturnType<typeof createChapterSession> | null>(null);
  const pushRef = useRef<ReturnType<typeof createSyncPush> | null>(null);
  const switchRef = useRef<((target: EditorScope) => Promise<void>) | null>(null);
  const switchingRef = useRef(false);
  const applyingRef = useRef(false);
  const dirtyRef = useRef(false);
  const openedRef = useRef<string | null>(null);
  const activeRef = useRef(isAiActive);
  const remoteRef = useRef(onRemoteChange);
  activeRef.current = isAiActive;
  remoteRef.current = onRemoteChange;

  useEffect(() => {
    let disposed = false;
    let pageHidden = false;
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
    let currentScope: EditorScope | null = null;
    function openChapter(scope: EditorScope, chapter: EditorDocument) {
      const quill = quillRef.current;
      if (disposed || !quill) return;
      pushRef.current?.close();
      sessionRef.current?.stop();
      const url = new URL(window.location.href);
      for (const key of ["seriesId", "bookId", "chapterId", "universeId", "pieceId"]) url.searchParams.delete(key);
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
        const change = diffDocuments(quill.getContents(), document);
        if (change.ops.length > 0) {
          const selection = quill.getSelection();
          applyingRef.current = true;
          quill.updateContents(change, "api");
          if (selection) {
            const text = quill.getText();
            const index = selectionBoundary(text, change.transformPosition(selection.index, true));
            const end = selectionBoundary(text, change.transformPosition(selection.index + selection.length, true));
            quill.setSelection(index, Math.max(0, end - index), "silent");
          }
          applyingRef.current = false;
          remoteRef.current();
        }
        setBlocked(problem?.kind === "blocked");
        setDetail(problem?.detail);
        setStatus(problem?.message ?? `${chapter.title} · ${dirty ? "Unsaved changes" : "Saved"}${storageFailed ? " · Recovery storage unavailable; export before closing" : ""}`);
        if (diffDocuments(quill.getContents(), document).ops.length > 0) session.edit(quill.getContents());
      });
      let latestRecovery: string | null = null;
      sessionRef.current = session;
      currentScope = scope;
      pushRef.current = createSyncPush(scope, session, () => activeRef.current() || switchingRef.current, pageHidden);
      openedRef.current = JSON.stringify({ scope, title: chapter.title });
      setScope(scope);
      setBlocked(false);
      setDetail(undefined);
      setTitle(chapter.title);
      setStatus(`${chapter.title} · Saved${storageFailed ? " · Recovery storage unavailable" : ""}`);
      setReady(true);
      session.edit(quill.getContents());
    }
    switchRef.current = async (target) => {
      const quill = quillRef.current;
      const session = sessionRef.current;
      if (!quill || !session || !canSwitchChapter(currentScope, target, switchingRef.current || activeRef.current())) return;
      switchingRef.current = true;
      setReady(false);
      setDetail(undefined);
      setStatus("Saving current chapter before opening…");
      try {
        session.edit(quill.getContents());
        await session.flush();
        if (disposed) return;
        pushRef.current?.close();
        pushRef.current = null;
        setStatus("Loading chapter…");
        const chapter = await loadEditorDocument(client, target);
        if (disposed) return;
        openChapter(target, chapter);
      } catch (error) {
        if (!disposed && !pushRef.current && currentScope) pushRef.current = createSyncPush(currentScope, session, () => activeRef.current() || switchingRef.current, pageHidden);
        if (!disposed && error instanceof ManuscriptApiError && error.detail !== error.message) setDetail(error.detail);
        if (!disposed) setStatus(`Chapter switch failed: ${error instanceof Error ? error.message : "Unknown error"} Current chapter retained.`);
      } finally {
        switchingRef.current = false;
        if (!disposed) setReady(true);
      }
    };
    queueMicrotask(() => {
      if (disposed) return;
      void loadEditorChapter(client, window.location.search).then(({ scope, chapter }) => openChapter(scope, chapter)).catch((error: unknown) => {
        if (!disposed) {
          setDetail(error instanceof ManuscriptApiError && error.detail !== error.message ? error.detail : undefined);
          setStatus(error instanceof Error ? error.message : "Chapter load failed.");
          setNeedsChapter(error instanceof NoEditorChapter);
        }
      });
    });
    const timer = window.setInterval(() => {
      pushRef.current?.check();
    }, 1000);
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (dirtyRef.current) {
        event.preventDefault();
        event.returnValue = "";
      }
    };
    const pageHide = () => { pageHidden = true; pushRef.current?.hide(); };
    const pageShow = () => { pageHidden = false; pushRef.current?.show(); };
    window.addEventListener("pagehide", pageHide);
    window.addEventListener("pageshow", pageShow);
    window.addEventListener("beforeunload", beforeUnload);
    return () => {
      disposed = true;
      window.clearInterval(timer);
      window.removeEventListener("beforeunload", beforeUnload);
      window.removeEventListener("pagehide", pageHide);
      window.removeEventListener("pageshow", pageShow);
      pushRef.current?.close();
      pushRef.current = null;
      sessionRef.current?.stop();
      sessionRef.current = null;
      switchRef.current = null;
    };
  }, [quillRef]);

  return {
    ready,
    blocked,
    status,
    detail,
    title,
    scope,
    switchChapter: (target: EditorScope) => switchRef.current?.(target),
    async leave(target: URL = landingUrl(window.location.href)) {
      const quill = quillRef.current;
      const session = sessionRef.current;
      if (switchingRef.current || activeRef.current()) return;
      if (quill && session) {
        switchingRef.current = true;
        setReady(false);
        setStatus("Saving before leaving…");
        try {
          session.edit(quill.getContents());
          await session.flush();
        } catch (error) {
          setStatus(`Leaving stopped: ${error instanceof Error ? error.message : "Unknown error"} Current chapter retained.`);
          switchingRef.current = false;
          setReady(true);
          return;
        }
      }
      pushRef.current?.close();
      pushRef.current = null;
      window.location.assign(target);
    },
    notice: null,
    recovery,
    needsChapter,
    async bootstrap() {
      setNeedsChapter(false);
      setDetail(undefined);
      setStatus("Creating chapter…");
      try {
        const scope = await bootstrapEditorChapter(createManuscriptClient());
        const url = new URL(window.location.href);
        for (const [key, value] of Object.entries(scope)) url.searchParams.set(key, String(value));
        window.location.assign(url);
      } catch (error) {
        setDetail(error instanceof ManuscriptApiError && error.detail !== error.message ? error.detail : undefined);
        setStatus(error instanceof Error ? error.message : "Chapter creation failed.");
        setNeedsChapter(true);
      }
    },
    canStartAi: () => !switchingRef.current && (sessionRef.current?.canStartAi() ?? false),
    editRejection: (): string | null => null,
    reserveAi() {
      const session = sessionRef.current;
      if (switchingRef.current || !session) return null;
      const reservation = session.reserveAi();
      if (!reservation) return null;
      return {
        async wait(signal: AbortSignal) {
          await reservation.wait(signal);
          if (switchingRef.current || sessionRef.current !== session) throw new Error("Chapter session changed before starting AI.");
        },
        release: reservation.release,
      };
    },
    documentChanged() {
      if (!applyingRef.current && quillRef.current) {
        sessionRef.current?.edit(quillRef.current.getContents());
        try {
          if (openedRef.current) window.localStorage.setItem(LAST_EDITED_KEY, openedRef.current);
        } catch {}
      }
    },
    exportDocument: () => JSON.stringify({ document: quillRef.current?.getContents() }),
  };
}
