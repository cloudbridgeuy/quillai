import { useEffect, useRef, useState, type RefObject } from "react";
import type Quill from "quill";
import Delta from "quill-delta";
import { createManuscriptClient } from "../manuscripts/client";
import type { EditorScope, PieceScope, ScriptGroup } from "../manuscripts/contract";
import { createChapterSession, type SyncProblem } from "./chapter_session";
import { chapterDelta } from "./chapter_sync";
import { diffDocuments } from "./document_diff";
import { selectionBoundary } from "./selection_boundary";
import { composeScenes, placeRemote, SCENE_BREAK, splitChange, type ScriptLayout } from "./script_layout";
import { createScenePoll } from "./script_poll";
import { sceneTitles } from "./scene_break";
import { crossesBreak, offBreak, scriptKeyAction } from "./script_keys";
import { editorUrl, LAST_EDITED_KEY, landingUrl } from "../landing/routes";

const RECOVERY_KEY = "quillai.chapter.recovery.v1";
const SCENE_RECOVERY_KEY = "quillai.script.recovery.v1";
const BREAK_NOTICE = "Edits across a scene break are not saved";
const EDIT_ACROSS_BREAK = "Select text inside one scene to use Edit.";

type SceneState = {
  id: number;
  title: string;
  session: ReturnType<typeof createChapterSession>;
  dirty: boolean;
  problem: SyncProblem | null;
};

function withNewline(document: Delta): Delta {
  const last = document.ops[document.ops.length - 1]?.insert;
  return typeof last === "string" && last.endsWith("\n") ? document : document.concat(new Delta().insert("\n"));
}

function breaksIntact(document: Delta, layout: ScriptLayout): boolean {
  const last = layout[layout.length - 1];
  return last !== undefined && last.start + last.length === document.length() && layout.every((span) => {
    const ops = document.slice(span.start - 1, span.start).ops;
    return ops.length === 1 && ops[0]!.insert === "\n"
      && JSON.stringify(ops[0]!.attributes) === JSON.stringify({ [SCENE_BREAK]: span.sceneId });
  });
}

export function useScriptSync(
  quillRef: RefObject<Quill | null>,
  isAiActive: () => boolean,
  onRemoteChange: () => void,
  groupId: number,
) {
  const [status, setStatus] = useState("Loading script…");
  const [title, setTitle] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const [blocked, setBlocked] = useState(false);
  const [recovery, setRecovery] = useState<string | null>(null);
  const [scope, setScope] = useState<EditorScope | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const caretRef = useRef<number | null>(null);
  const scenesRef = useRef<SceneState[]>([]);
  const layoutRef = useRef<ScriptLayout>([]);
  const applyingRef = useRef(false);
  const leavingRef = useRef(false);
  const openedRef = useRef<string | null>(null);
  const noticeRef = useRef<string | null>(null);
  const refreshRef = useRef(() => {});
  const activeRef = useRef(isAiActive);
  const remoteRef = useRef(onRemoteChange);
  activeRef.current = isAiActive;
  remoteRef.current = onRemoteChange;

  useEffect(() => {
    let disposed = false;
    let storageFailed = false;
    let groupTitle = "";
    try {
      const copies: string[] = [];
      for (let index = window.sessionStorage.length - 1; index >= 0; index--) {
        const key = window.sessionStorage.key(index);
        if (key?.startsWith(`${SCENE_RECOVERY_KEY}.`)) {
          window.sessionStorage.setItem(`${RECOVERY_KEY}.${crypto.randomUUID()}`, window.sessionStorage.getItem(key) ?? "");
          window.sessionStorage.removeItem(key);
        }
      }
      for (let index = 0; index < window.sessionStorage.length; index++) {
        const key = window.sessionStorage.key(index);
        const copy = key?.startsWith(`${RECOVERY_KEY}.`) ? window.sessionStorage.getItem(key) : null;
        if (copy) copies.push(copy);
      }
      setRecovery(copies.length ? JSON.stringify(copies) : null);
    } catch {
      storageFailed = true;
    }
    const client = createManuscriptClient();
    const poll = createScenePoll((since) => client.pollScriptGroup({ pieceId: groupId }, since));
    const sessionClient = {
      postDelta: client.postDelta,
      getDeltas: (scope: EditorScope, since: number) => {
        if (!("pieceId" in scope)) throw new Error("Script scenes sync by piece.");
        return poll.getDeltas(scope.pieceId, since);
      },
    };

    function refresh() {
      const scenes = scenesRef.current;
      const failing = scenes.find((scene) => scene.problem !== null);
      setBlocked(scenes.some((scene) => scene.problem?.kind === "blocked"));
      setStatus(failing?.problem
        ? `${failing.title}: ${failing.problem.message}`
        : `${groupTitle} · ${noticeRef.current ?? (scenes.some((scene) => scene.dirty) ? "Unsaved changes" : "Saved")}${storageFailed ? " · Recovery storage unavailable; export before closing" : ""}`);
    }

    refreshRef.current = refresh;

    function publish(scene: SceneState, scope: PieceScope, document: Delta, dirty: boolean, problem: SyncProblem | null) {
      const quill = quillRef.current;
      if (disposed || !quill) return;
      scene.dirty = dirty;
      scene.problem = problem;
      try {
        const key = `${SCENE_RECOVERY_KEY}.${scene.id}`;
        if (dirty) window.sessionStorage.setItem(key, JSON.stringify({ scope, document }));
        else window.sessionStorage.removeItem(key);
      } catch {
        storageFailed = true;
      }
      const view = withNewline(document);
      const span = layoutRef.current.find((candidate) => candidate.sceneId === scene.id);
      const change = span ? diffDocuments(quill.getContents(span.start, span.length), view) : new Delta();
      if (change.ops.length > 0) {
        const placed = placeRemote(scene.id, change, layoutRef.current);
        const selection = quill.getSelection();
        applyingRef.current = true;
        quill.updateContents(placed.change, "api");
        layoutRef.current = placed.layout;
        if (selection) {
          const text = quill.getText();
          const index = selectionBoundary(text, placed.change.transformPosition(selection.index, true));
          const end = selectionBoundary(text, placed.change.transformPosition(selection.index + selection.length, true));
          quill.setSelection(index, Math.max(0, end - index), "silent");
        }
        applyingRef.current = false;
        remoteRef.current();
      }
      refresh();
      if (diffDocuments(document, view).ops.length > 0) scene.session.edit(view);
    }

    function open(group: ScriptGroup) {
      const quill = quillRef.current;
      if (disposed || !quill) return;
      const groupScope = { universeId: group.universeId, pieceId: group.id };
      const url = new URL(window.location.href);
      const requested = Number(url.searchParams.get("pieceId"));
      for (const key of ["seriesId", "bookId", "chapterId", "universeId", "pieceId"]) url.searchParams.delete(key);
      for (const [key, value] of Object.entries(groupScope)) url.searchParams.set(key, String(value));
      window.history.replaceState(null, "", url);
      for (const scene of group.scenes) sceneTitles.set(scene.id, scene.title);
      const composed = composeScenes(group.scenes.map((scene) => ({ id: scene.id, content: chapterDelta(scene.content) })));
      applyingRef.current = true;
      quill.setContents(composed.document, "api");
      quill.history.clear();
      applyingRef.current = false;
      layoutRef.current = composed.layout;
      caretRef.current = (composed.layout.find((span) => span.sceneId === requested) ?? composed.layout[0])?.start ?? null;
      remoteRef.current();
      groupTitle = group.title;
      scenesRef.current = group.scenes.map((scene) => {
        const scope = { universeId: scene.universeId, pieceId: scene.id };
        const state: SceneState = { id: scene.id, title: scene.title, session: null!, dirty: false, problem: null };
        state.session = createChapterSession(sessionClient, scope, { content: scene.content, head_rev: scene.headRev },
          (document, dirty, problem) => publish(state, scope, document, dirty, problem));
        return state;
      });
      openedRef.current = JSON.stringify({ scope: groupScope, title: group.title });
      setScope(groupScope);
      setTitle(group.title);
      refresh();
      setReady(true);
      for (const scene of scenesRef.current) scene.session.edit(withNewline(scene.session.document()));
    }

    queueMicrotask(() => {
      if (disposed) return;
      void client.getScriptGroup({ pieceId: groupId }).then(open).catch((error: unknown) => {
        if (!disposed) setStatus(error instanceof Error ? error.message : "Script load failed.");
      });
    });
    const timer = window.setInterval(() => {
      if (!activeRef.current() && !leavingRef.current) poll.tick(scenesRef.current.map(({ id, session }) => ({ id, tick: session.tick })));
    }, 1000);
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (scenesRef.current.some((scene) => scene.dirty)) {
        event.preventDefault();
        event.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", beforeUnload);
    const quill = quillRef.current;
    const host = quill?.root.parentElement;
    const keydown = (event: KeyboardEvent) => {
      const range = quill?.getSelection();
      if (!quill || !range || event.isComposing) return;
      const action = scriptKeyAction(event, range, layoutRef.current);
      if (action.kind === "pass") return;
      event.preventDefault();
      event.stopPropagation();
      if (action.kind === "move") quill.setSelection(action.index, 0, "user");
      else block();
    };
    const paste = (event: ClipboardEvent) => {
      const range = quill?.getSelection();
      if (!range || !crossesBreak(range, layoutRef.current)) return;
      event.preventDefault();
      event.stopPropagation();
      block();
    };
    const selection = (range: { index: number; length: number } | null, previous: { index: number } | null) => {
      const index = range && range.length === 0 ? offBreak(range.index, previous?.index ?? null, layoutRef.current) : null;
      if (quill && index !== null) quill.setSelection(index, 0, "user");
    };
    host?.addEventListener("keydown", keydown, true);
    host?.addEventListener("paste", paste, true);
    quill?.on("selection-change", selection);
    return () => {
      disposed = true;
      host?.removeEventListener("keydown", keydown, true);
      host?.removeEventListener("paste", paste, true);
      quill?.off("selection-change", selection);
      window.clearInterval(timer);
      window.removeEventListener("beforeunload", beforeUnload);
      for (const scene of scenesRef.current) scene.session.stop();
      scenesRef.current = [];
    };
  }, [quillRef, groupId]);

  useEffect(() => {
    const quill = quillRef.current;
    if (!ready || !quill || caretRef.current === null) return;
    quill.setSelection(caretRef.current, 0, "api");
    quill.scrollSelectionIntoView();
    caretRef.current = null;
  }, [quillRef, ready]);

  function block() {
    noticeRef.current = BREAK_NOTICE;
    setNotice(BREAK_NOTICE);
    refreshRef.current();
  }

  function revert() {
    queueMicrotask(() => {
      const quill = quillRef.current;
      if (!quill) return;
      const composed = composeScenes(scenesRef.current.map(({ id, session }) => ({ id, content: session.document() })));
      applyingRef.current = true;
      quill.updateContents(diffDocuments(quill.getContents(), composed.document), "api");
      quill.history.clear();
      applyingRef.current = false;
      layoutRef.current = composed.layout;
      block();
    });
  }

  async function flushAll(message: string): Promise<boolean> {
    if (leavingRef.current || activeRef.current()) return false;
    leavingRef.current = true;
    setReady(false);
    setStatus(message);
    try {
      for (const scene of scenesRef.current) await scene.session.flush();
      return true;
    } catch (error) {
      setStatus(`Leaving stopped: ${error instanceof Error ? error.message : "Unknown error"} Current script retained.`);
      leavingRef.current = false;
      setReady(true);
      return false;
    }
  }

  return {
    ready,
    blocked,
    status,
    title,
    scope,
    async switchChapter(target: EditorScope) {
      if (await flushAll("Saving script before opening…")) window.location.assign(editorUrl(window.location.href, target));
    },
    async leave(target: URL = landingUrl(window.location.href)) {
      if (await flushAll("Saving before leaving…")) window.location.assign(target);
    },
    notice,
    recovery,
    needsChapter: false,
    async bootstrap() {},
    editRejection: (range: { index: number; length: number }) => crossesBreak(range, layoutRef.current) ? EDIT_ACROSS_BREAK : null,
    canStartAi: () => !leavingRef.current && scenesRef.current.length > 0 && scenesRef.current.every(({ session }) => session.canStartAi()),
    reserveAi() {
      if (leavingRef.current) return null;
      const reservations: NonNullable<ReturnType<SceneState["session"]["reserveAi"]>>[] = [];
      for (const { session } of scenesRef.current) {
        const reservation = session.reserveAi();
        if (!reservation) {
          for (const held of reservations) held.release();
          return null;
        }
        reservations.push(reservation);
      }
      return {
        async wait(signal: AbortSignal) {
          await Promise.all(reservations.map((reservation) => reservation.wait(signal)));
          if (leavingRef.current) throw new Error("Script session changed before starting AI.");
        },
        release: () => { for (const reservation of reservations) reservation.release(); },
      };
    },
    documentChanged(delta: Delta, oldDelta: Delta) {
      const quill = quillRef.current;
      if (applyingRef.current || !quill) return;
      const document = quill.getContents();
      let split: ReturnType<typeof splitChange> = { kind: "crossesBreak" };
      try {
        split = splitChange(delta, layoutRef.current);
        if (split.kind === "crossesBreak") split = splitChange(diffDocuments(oldDelta, document), layoutRef.current);
      } catch {}
      if (split.kind === "crossesBreak" || !breaksIntact(document, split.layout)) {
        revert();
        return;
      }
      layoutRef.current = split.layout;
      noticeRef.current = null;
      setNotice(null);
      for (const scene of scenesRef.current) {
        const span = split.layout.find((candidate) => candidate.sceneId === scene.id);
        if (span && split.changes.has(scene.id)) scene.session.edit(document.slice(span.start, span.start + span.length));
      }
      try {
        if (openedRef.current) window.localStorage.setItem(LAST_EDITED_KEY, openedRef.current);
      } catch {}
    },
    exportDocument: () => JSON.stringify({
      scenes: scenesRef.current.map(({ id, session }) => ({ pieceId: id, document: session.document() })),
    }),
  };
}
