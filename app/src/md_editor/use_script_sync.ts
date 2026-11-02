import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";
import type Quill from "quill";
import Delta from "quill-delta";
import { createManuscriptClient, ManuscriptApiError } from "../manuscripts/client";
import type { EditorScope, PieceScope, ScriptGroup } from "../manuscripts/contract";
import { createChapterSession, type SyncProblem } from "./chapter_session";
import { chapterDelta } from "./chapter_sync";
import { diffDocuments } from "./document_diff";
import { selectionBoundary } from "./selection_boundary";
import { composeScenes, placeRemote, SCENE_BREAK, splitChange, type ScriptLayout } from "./script_layout";
import { createScenePoll } from "./script_poll";
import { createSyncStream } from "./sync_push";
import { checkScript, scriptNudge, scriptRefetch, scriptSchedule, scriptSuspend } from "./script_scheduler";
import { sceneTitles } from "./scene_break";
import { crossesBreak, offBreak, scriptKeyAction, startsList } from "./script_keys";
import { blockAttribute, cycleBlock, parseBlock, SCRIPT_BLOCK, splitLineDelta, type BlockType } from "./script_blocks";
import { toFdx } from "./fdx";
import { toFountain } from "./fountain";
import { editorUrl, LAST_EDITED_KEY, landingUrl } from "../landing/routes";
import { createCaretTracker } from "./caret_position";

const RECOVERY_KEY = "quillai.chapter.recovery.v1";
const SCENE_RECOVERY_KEY = "quillai.script.recovery.v1";
const BREAK_NOTICE = "Edits across a scene break are not saved";
const SCENES_CHANGED = "Scenes changed; reload";
const EDIT_ACROSS_BREAK = "Select text inside one scene to use Edit.";

type SceneState = {
  id: number;
  title: string;
  position: number;
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
  const [detail, setDetail] = useState<string>();
  const [title, setTitle] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const [blocked, setBlocked] = useState(false);
  const [recovery, setRecovery] = useState<string | null>(null);
  const [scope, setScope] = useState<PieceScope | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [changed, setChanged] = useState(false);
  const [adding, setAdding] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);
  const [caretScene, setCaretScene] = useState<number | null>(null);
  const [blockType, setBlockType] = useState<BlockType>("action");
  const caretTrackerRef = useRef<ReturnType<typeof createCaretTracker> | null>(null);
  const caretRef = useRef<number | null>(null);
  const rangeRef = useRef<{ index: number; length: number } | null>(null);
  const scenesRef = useRef<SceneState[]>([]);
  const streamRef = useRef<ReturnType<typeof createSyncStream> | null>(null);
  const layoutRef = useRef<ScriptLayout>([]);
  const applyingRef = useRef(false);
  const leavingRef = useRef(false);
  const openedRef = useRef<string | null>(null);
  const activeRef = useRef(isAiActive);
  const remoteRef = useRef(onRemoteChange);
  activeRef.current = isAiActive;
  remoteRef.current = onRemoteChange;

  useEffect(() => {
    const tracker = quillRef.current ? createCaretTracker(quillRef.current) : null;
    caretTrackerRef.current = tracker;
    let disposed = false;
    let storageFailed = false;
    let pageHidden = false;
    let hidden = document.hidden;
    let schedule = scriptSchedule(Date.now());
    if (hidden) schedule = scriptSuspend(schedule, true);
    let membership = "";
    let running = false;
    let queued = false;
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
    const poll = createScenePoll(async (since) => {
      const response = await client.pollScriptGroup({ pieceId: groupId }, since);
      if (!disposed && response.children.join() !== scenesRef.current.map(({ id }) => id).join()) setChanged(true);
      return response;
    });
    const sessionClient = {
      postDelta: client.postDelta,
      getDeltas: (scope: EditorScope, since: number) => {
        if (!("pieceId" in scope)) throw new Error("Script scenes sync by piece.");
        return poll.getDeltas(scope.pieceId, since);
      },
    };

    function check() {
      if (disposed) return;
      const scenes = scenesRef.current;
      const ids = scenes.map(({ id }) => id);
      const nextMembership = ids.join(",");
      if (nextMembership !== membership || streamRef.current === null) {
        membership = nextMembership;
        streamRef.current?.close();
        streamRef.current = null;
        if (!hidden && !pageHidden && ids.length > 0 && ids.length <= 256 && !leavingRef.current) {
          streamRef.current = createSyncStream(ids, {
            refetch() { schedule = scriptRefetch(schedule); if (running) queued = true; else check(); },
            changed(id, revision) { schedule = scriptNudge(schedule, id, revision); if (running) queued = true; else check(); },
          });
        }
      }
      const result = checkScript(schedule, scenes.map(({ id, session }) => ({
        id, revision: session.revision(), pending: session.hasPendingWork(),
        busy: running || session.isBusy(), reserved: session.isReserved() || activeRef.current() || leavingRef.current,
      })), Date.now());
      schedule = result.state;
      const targets = scenes.filter(({ id }) => result.ids.includes(id));
      if (targets.length === 0) return;
      running = true;
      const ticks: Promise<void>[] = [];
      poll.tick(targets.map(({ id, session }) => ({ id, tick: () => {
        const tick = session.tick();
        ticks.push(tick);
        return tick;
      } })));
      void Promise.all(ticks).finally(() => {
        running = false;
        if (queued) { queued = false; check(); }
      });
    }
    function visibility() {
      const suspended = document.hidden || pageHidden;
      if (hidden === suspended) return;
      hidden = suspended;
      schedule = scriptSuspend(schedule, hidden);
      if (hidden) { streamRef.current?.close(); streamRef.current = null; }
      membership = "";
      if (!hidden) check();
    }
    const pageHide = () => { tracker?.flush(); pageHidden = true; visibility(); };
    const pageShow = () => { pageHidden = false; visibility(); };

    function refresh() {
      const scenes = scenesRef.current;
      const failing = scenes.find((scene) => scene.problem !== null);
      setBlocked(scenes.some((scene) => scene.problem?.kind === "blocked"));
      setDetail(failing?.problem?.detail);
      setStatus(failing?.problem
        ? `${failing.title}: ${failing.problem.message}`
        : `${groupTitle} · ${scenes.some((scene) => scene.dirty) ? "Unsaved changes" : "Saved"}${storageFailed ? " · Recovery storage unavailable; export before closing" : ""}`);
    }

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
      const restored = tracker?.open(groupScope, !composed.layout.some((span) => span.sceneId === requested));
      quill.history.clear();
      applyingRef.current = false;
      layoutRef.current = composed.layout;
      if (restored) selection(quill.getSelection(), null);
      caretRef.current = restored ? null : (composed.layout.find((span) => span.sceneId === requested) ?? composed.layout[0])?.start ?? null;
      remoteRef.current();
      groupTitle = group.title;
      scenesRef.current = group.scenes.map((scene) => {
        const scope = { universeId: scene.universeId, pieceId: scene.id };
        const state: SceneState = { id: scene.id, title: scene.title, position: scene.position, session: null!, dirty: false, problem: null };
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
      check();
    }

    queueMicrotask(() => {
      if (disposed) return;
      void client.getScriptGroup({ pieceId: groupId }).then(open).catch((error: unknown) => {
        if (!disposed) {
          setDetail(error instanceof ManuscriptApiError && error.detail !== error.message ? error.detail : undefined);
          setStatus(error instanceof Error ? error.message : "Script load failed.");
        }
      });
    });
    const timer = window.setInterval(check, 1000);
    document.addEventListener("visibilitychange", visibility);
    window.addEventListener("pagehide", pageHide);
    window.addEventListener("pageshow", pageShow);
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
      if (action.kind === "pass") {
        if (event.altKey || event.ctrlKey || event.metaKey || !quill.isEnabled()) return;
        if (event.key === " " && range.length === 0) {
          const [line, offset] = quill.getLine(range.index);
          if (line && startsList(quill.getText(range.index - offset, offset))) event.stopPropagation();
          return;
        }
        if (event.key === "Tab") {
          event.preventDefault();
          event.stopPropagation();
          applyBlock(cycleBlock(blockAt(range.index), event.shiftKey ? -1 : 1), range);
        } else if (event.key === "Enter" && !event.shiftKey && range.length === 0) {
          event.preventDefault();
          event.stopPropagation();
          const [line, offset] = quill.getLine(range.index);
          if (!line) return;
          quill.updateContents(splitLineDelta(range.index, line.length() - offset - 1, blockAt(range.index)), "user");
          quill.setSelection(range.index + 1, 0, "user");
        }
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      if (action.kind === "move") quill.setSelection(action.index, 0, "user");
      else block();
    };
    const refuse = (event: Event) => {
      event.preventDefault();
      event.stopPropagation();
      block();
    };
    const clipboard = (event: ClipboardEvent) => {
      const range = quill?.getSelection();
      if (range && crossesBreak(range, layoutRef.current)) refuse(event);
    };
    const drop = (event: DragEvent) => {
      if (event.target instanceof Element && event.target.closest(".ql-scene-break")) refuse(event);
    };
    const selection = (range: { index: number; length: number } | null, previous: { index: number } | null) => {
      const index = range && range.length === 0 ? offBreak(range.index, previous?.index ?? null, layoutRef.current) : null;
      if (quill && index !== null) quill.setSelection(index, 0, "user");
      const span = range && layoutRef.current.find((candidate) => range.index >= candidate.start && range.index < candidate.start + candidate.length);
      if (span) setCaretScene(span.sceneId);
      if (range) {
        rangeRef.current = range;
        setBlockType(blockAt(range.index));
      }
    };
    const textChange = () => {
      const range = quill?.getSelection();
      if (range) setBlockType(blockAt(range.index));
    };
    host?.addEventListener("keydown", keydown, true);
    host?.addEventListener("paste", clipboard, true);
    host?.addEventListener("cut", clipboard, true);
    host?.addEventListener("drop", drop, true);
    quill?.on("selection-change", selection);
    quill?.on("text-change", textChange);
    return () => {
      tracker?.close();
      caretTrackerRef.current = null;
      disposed = true;
      host?.removeEventListener("keydown", keydown, true);
      host?.removeEventListener("paste", clipboard, true);
      host?.removeEventListener("cut", clipboard, true);
      host?.removeEventListener("drop", drop, true);
      quill?.off("selection-change", selection);
      quill?.off("text-change", textChange);
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", visibility);
      window.removeEventListener("pagehide", pageHide);
      window.removeEventListener("pageshow", pageShow);
      streamRef.current?.close();
      streamRef.current = null;
      window.removeEventListener("beforeunload", beforeUnload);
      for (const scene of scenesRef.current) scene.session.stop();
      scenesRef.current = [];
    };
  }, [quillRef, groupId]);

  useLayoutEffect(() => { caretTrackerRef.current?.finish(); }, [ready, title]);

  useEffect(() => {
    const quill = quillRef.current;
    if (!ready || !quill || caretRef.current === null) return;
    quill.setSelection(caretRef.current, 0, "api");
    quill.scrollSelectionIntoView();
    caretRef.current = null;
  }, [quillRef, ready]);

  function block() {
    setNotice(BREAK_NOTICE);
  }

  function blockAt(index: number): BlockType {
    return parseBlock(quillRef.current?.getFormat(index, 0)[SCRIPT_BLOCK]);
  }

  function applyBlock(type: BlockType, range: { index: number; length: number }) {
    const quill = quillRef.current;
    if (!quill || !quill.isEnabled()) return;
    if (crossesBreak(range, layoutRef.current) || offBreak(range.index, null, layoutRef.current) !== null) return block();
    quill.formatLine(range.index, range.length, SCRIPT_BLOCK, blockAttribute(type), "user");
    quill.setSelection(range.index, range.length, "silent");
    setBlockType(type);
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
    caretTrackerRef.current?.flush();
    leavingRef.current = true;
    setReady(false);
    setDetail(undefined);
    setStatus(message);
    try {
      for (const scene of scenesRef.current) await scene.session.flush();
      streamRef.current?.close();
      streamRef.current = null;
      return true;
    } catch (error) {
      setDetail(scenesRef.current.find((scene) => scene.problem !== null)?.problem?.detail);
      setStatus(`Leaving stopped: ${error instanceof Error ? error.message : "Unknown error"} Current script retained.`);
      leavingRef.current = false;
      setReady(true);
      return false;
    }
  }

  async function addScene() {
    const scenes = scenesRef.current;
    const quill = quillRef.current;
    if (adding || leavingRef.current || activeRef.current() || scenes.length === 0 || !quill) return;
    const found = scenes.findIndex(({ id }) => id === caretScene);
    const after = found === -1 ? scenes.length - 1 : found;
    const previous = scenes[after]!;
    const universeId = scope?.universeId;
    if (universeId === undefined) return;
    const client = createManuscriptClient();
    setAdding(true);
    setAddError(null);
    const later = scenes.slice(after + 1);
    let created;
    try {
      for (let index = later.length - 1; index >= 0; index--) {
        const scene = later[index]!;
        const position = Math.max(scene.position, previous.position + 1 + index) + 1;
        await client.updatePiece({ pieceId: scene.id }, { position });
        scene.position = position;
      }
      created = await client.createChild({ pieceId: groupId }, { title: `Scene ${scenes.length + 1}`, position: previous.position + 1, label: "scene" });
    } catch (error) {
      setAddError(`New scene not added: ${error instanceof Error ? error.message : "Unknown error"}`);
      setAdding(false);
      return;
    }
    if (await flushAll("Saving script before opening the new scene…")) window.location.assign(editorUrl(window.location.href, { universeId, pieceId: created.id }));
    else setAdding(false);
  }

  return {
    addScene,
    blockType,
    setBlock(type: BlockType) {
      const quill = quillRef.current;
      if (!quill || !rangeRef.current) return;
      quill.focus();
      applyBlock(type, rangeRef.current);
    },
    fountain: () => {
      const quill = quillRef.current;
      return quill ? toFountain(layoutRef.current.map((span) => quill.getContents(span.start, span.length))) : "";
    },
    fdx: () => {
      const quill = quillRef.current;
      return toFdx(quill ? layoutRef.current.map((span) => quill.getContents(span.start, span.length)) : []);
    },
    adding,
    ready,
    blocked,
    status,
    detail,
    title,
    scope,
    treeScope: scope && caretScene !== null ? { universeId: scope.universeId, pieceId: caretScene } : scope,
    focusScene(pieceId: number) {
      const quill = quillRef.current;
      const span = layoutRef.current.find((candidate) => candidate.sceneId === pieceId);
      if (!quill || !span || leavingRef.current) return false;
      quill.setSelection(span.start, 0, "user");
      quill.scrollSelectionIntoView();
      return true;
    },
    async switchChapter(target: EditorScope) {
      if (await flushAll("Saving script before opening…")) window.location.assign(editorUrl(window.location.href, target));
    },
    async leave(target: URL = landingUrl(window.location.href)) {
      if (await flushAll("Saving before leaving…")) window.location.assign(target);
    },
    notice: addError ?? (changed ? SCENES_CHANGED : notice),
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
