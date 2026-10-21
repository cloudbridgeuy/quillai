import Quill from "quill";
import type { LoreMatchEntry } from "../lore_match";
import { Delta } from "quill/core";
import "quill/dist/quill.snow.css";
import {
  forwardRef,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";

import {
  buildAiGenerateUrl,
  buildAiRequestPreview,
  type AiAction,
  type AiRequestPreview,
  type PromptSectionOverrides,
} from "../ai_contract";
import {
  continueClickAction,
  canApplyEdit,
  getSavePresentation,
  getStatusPresentation,
  isTextOnlyDeltaDocument,
  rememberCursorRange,
  rememberEditSelection,
  resolveContinueSelection,
  resolveEditSelection,
  shouldClearFailedRunState,
  shouldPreservePendingEditSelection,
  shouldRestoreCancelledEditSelection,
  shouldRollbackFailedEditSelection,
} from "../editor_ai";
import { finalizeAiStreamRemainder, parseAiStreamChunk } from "../ai_stream";
import { toContractDeltaDocument, toQuillDeltaDocument } from "../delta_bridge";
import {
  failRun,
  finishRun,
  idleAiRunState,
  resetRunState,
  startCancelling,
  startContinuing,
  startEditing,
  type AiRunState,
} from "../ai_state";
import { analyzeMarkdownContext, isPatternComplete } from "../markdown";
import {
  PERSISTED_EDITOR_STATE_KEY,
  parsePersistedEditorState,
  serializePersistedEditorState,
  type PersistedEditorState,
} from "../editor_persistence";
import type {
  DeltaT,
  OnTextChangeFn,
  OnSelectionChangeFn,
  RangeT,
} from "../types";
import {
  finishCancelledThread,
  finishGenerateThread,
  getThreadPresentation,
  historyForResume,
  invalidateThreadForManualEdit,
  manualEditFromDelta,
  resetThread,
  resetThreadOnInstructionChange,
  shouldUseResumeThread,
  shouldContinueWithThread,
  updateThreadSettings,
  type AiThreadState,
} from "../ai_thread";
import { calculateDocumentWindow, type DocumentWindowMode } from "../window";
import { useChapterSync } from "../use_chapter_sync";
import { buildContextInspector } from "../context_inspector";
import { createSettingsClient } from "../../settings/client";
import { panelShortcut, transitionPanel, type PanelState } from "../editor_panels";
import { ManuscriptTree } from "./ManuscriptTree";
import { LorePanel } from "./LorePanel";
import { useLore } from "../../manuscripts/use_lore";
import { StickyNote } from "../../components/StickyNote";
import { appendContinueFragment, createContinueStream, transformContinueSelection, transformContinueStream, type ContinueStream } from "../continue_stream";
import { renderContinueTint } from "../continue_tint";
import { ContinueWindow } from "./ContinueWindow";
import { LoreWindow } from "./LoreWindow";
import { getApiBaseUrl } from "../../api_config";
import { parseModelCatalog, resolveModelChoice, type ModelCatalog } from "../model_catalog";
import { beginInspectorRun, buildInspectorCost, recordInspectorUsage, type InspectorRun } from "../inspector_cost";

function processMarkdownSyntax(quill: Quill, delta: DeltaT, source: string) {
  if (source !== "user") return;

  const text = quill.getText();
  const selection = quill.getSelection();

  if (!selection) return;

  const lastOp = delta.ops?.[delta.ops.length - 1];
  if (!lastOp || !lastOp.insert || typeof lastOp.insert !== "string") return;

  const lastChar = lastOp.insert.charAt(lastOp.insert.length - 1);
  if (lastChar !== "*" && lastChar !== "_") return;

  const currentPos = selection.index;
  const lineStart = text.lastIndexOf("\n", currentPos - 1) + 1;
  const lineEnd = text.indexOf("\n", currentPos);
  const lineText = text.substring(lineStart, lineEnd === -1 ? text.length : lineEnd);

  const relativePos = currentPos - lineStart;
  if (!isPatternComplete(lineText, relativePos)) {
    return;
  }

  const context = analyzeMarkdownContext(lineText, relativePos);
  if (!context) return;

  const absoluteStart = lineStart + context.start;
  const replacementDelta = new Delta()
    .retain(absoluteStart)
    .delete(context.match[0].length)
    .insert(context.content, context.pattern.format);

  quill.updateContents(replacementDelta, "silent");

  const newCursorPos = absoluteStart + context.content.length;
  quill.setSelection(newCursorPos, 0, "silent");

  const formatAttributes = Object.keys(context.pattern.format);
  for (const attr of formatAttributes) {
    quill.format(attr, false, "silent");
  }
}

export type ComponentPropsT = {
  readOnly: boolean;
  defaultValue: DeltaT;
  onTextChange: OnTextChangeFn;
  onSelectionChange: OnSelectionChangeFn;
};

function getStoredEditorState(): PersistedEditorState {
  if (typeof window === "undefined") {
    return parsePersistedEditorState(null);
  }

  try {
    return parsePersistedEditorState(
      window.localStorage.getItem(PERSISTED_EDITOR_STATE_KEY),
    );
  } catch {
    return parsePersistedEditorState(null);
  }
}

function getAiErrorMessage(responseText: string, status: number): string {
  const trimmed = responseText.trim();
  if (trimmed === "") {
    return `AI request failed (${status}).`;
  }

  try {
    const parsed = JSON.parse(trimmed) as { error?: unknown };
    if (typeof parsed.error === "string" && parsed.error.trim() !== "") {
      return parsed.error.trim();
    }
  } catch {
    return trimmed;
  }

  return trimmed;
}

export const Component = forwardRef<Quill, ComponentPropsT>(
  ({ readOnly, defaultValue, onTextChange, onSelectionChange }, ref) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const defaultValueRef = useRef(defaultValue);
    const initialReadOnlyRef = useRef(readOnly);
    const onTextChangeRef = useRef(onTextChange);
    const onSelectionChangeRef = useRef(onSelectionChange);

    useLayoutEffect(() => {
      onTextChangeRef.current = onTextChange;
      onSelectionChangeRef.current = onSelectionChange;
    });

    useEffect(() => {
      if (!ref || typeof ref === "function") return;
      ref.current?.enable(!readOnly);
    }, [ref, readOnly]);

    useEffect(() => {
      if (containerRef === null || !ref || typeof ref === "function") {
        return;
      }

      const container = containerRef.current;
      if (container === null) return;

      const editorContainer = container.appendChild(
        container.ownerDocument.createElement("div"),
      );

      const quill = new Quill(editorContainer, {
        theme: "snow",
        modules: { history: { userOnly: true } },
      });

      ref.current = quill;

      if (defaultValueRef.current) {
        quill.setContents(defaultValueRef.current);
      }

      quill.enable(!initialReadOnlyRef.current);

      quill.on(Quill.events.TEXT_CHANGE, (delta, oldDelta, source) => {
        processMarkdownSyntax(quill, delta, source);
        onTextChangeRef.current?.(delta, oldDelta, source);
      });

      quill.on(Quill.events.SELECTION_CHANGE, (range, oldRange, source) => {
        onSelectionChangeRef.current?.(range, oldRange, source);
      });

      return () => {
        quill.off(Quill.events.TEXT_CHANGE);
        quill.off(Quill.events.SELECTION_CHANGE);
        ref.current = null;
        container.innerHTML = "";
      };
    }, [ref]);

    return <div className="editor-host" ref={containerRef}></div>;
  },
);

Component.displayName = "EditorComponent";

export const Container = ({ loreEntries: suppliedLoreEntries }: { loreEntries?: readonly LoreMatchEntry[] }) => {
  const initialEditorStateRef = useRef<PersistedEditorState | null>(null);
  if (initialEditorStateRef.current === null) {
    initialEditorStateRef.current = getStoredEditorState();
  }
  const initialEditorState = initialEditorStateRef.current;

  const [range, setRange] = useState<RangeT | null>(null);
  const [panel, setPanel] = useState<PanelState>(null);
  const settingsRef = useRef<HTMLDivElement>(null);
  const [editBoxOpen, setEditBoxOpen] = useState(false);
  const editInstructionRef = useRef<HTMLTextAreaElement>(null);
  const [storyContext, setStoryContext] = useState(initialEditorState.storyContext);
  const [instruction, setInstruction] = useState(initialEditorState.instruction);
  const [windowSize, setWindowSize] = useState(initialEditorState.windowSize);
  const [windowMode, setWindowMode] = useState<DocumentWindowMode>(initialEditorState.windowMode);
  const [aiTextTint, setAiTextTint] = useState(initialEditorState.aiTextTint);
  const [showContinueWindow, setShowContinueWindow] = useState(initialEditorState.showContinueWindow);
  const [desktop, setDesktop] = useState(() => window.matchMedia("(min-width: 1100px)").matches);
  const continueStreamRef = useRef<ContinueStream | null>(null);
  const applyingContinueRef = useRef(false);
  const tintTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const tintEnabledRef = useRef(aiTextTint);
  tintEnabledRef.current = aiTextTint;
  const [aiRunState, setAiRunState] = useState<AiRunState>(idleAiRunState);
  const [activeContinueWithThread, setActiveContinueWithThread] = useState<boolean | null>(null);
  const [sectionOverrides, setSectionOverrides] = useState<Record<AiAction, PromptSectionOverrides>>({ generate: {}, edit: {} });
  const [inspectorAction, setInspectorAction] = useState<AiAction>("generate");
  const [settingsClient] = useState(createSettingsClient);
  const [authorGuidance, setAuthorGuidance] = useState("");
  const [guidanceDraft, setGuidanceDraft] = useState("");
  const [guidanceStatus, setGuidanceStatus] = useState("Loading author guidance…");
  const [guidancePending, setGuidancePending] = useState(true);
  const [guidanceLoaded, setGuidanceLoaded] = useState(false);
  useEffect(() => {
    let active = true;
    settingsClient.getAuthorGuidance().then(({ text }) => {
      if (!active) return;
      setAuthorGuidance(text);
      setGuidanceDraft(text);
      setGuidanceLoaded(true);
      setGuidanceStatus("");
    }).catch((error: unknown) => {
      if (active) setGuidanceStatus(getUnknownAiFailureMessage(error));
    }).finally(() => {
      if (active) setGuidancePending(false);
    });
    return () => { active = false; };
  }, [settingsClient]);

  async function saveAuthorGuidance() {
    setGuidancePending(true);
    try {
      const { text } = await settingsClient.putAuthorGuidance({ text: guidanceDraft });
      setAuthorGuidance(text);
      setGuidanceDraft(text);
      setGuidanceStatus("Author guidance saved.");
    } catch (error) {
      setGuidanceStatus(getUnknownAiFailureMessage(error));
    } finally {
      setGuidancePending(false);
    }
  }
  const [threadState, setThreadState] = useState<AiThreadState>(initialEditorState.threadState);
  const [modelCatalog, setModelCatalog] = useState<ModelCatalog | null>(null);
  const inspectorRunId = useRef(0);
  const [inspectorRuns, setInspectorRuns] = useState<Record<AiAction, InspectorRun | null>>({ generate: null, edit: null });
  const [modelsPending, setModelsPending] = useState(true);
  useEffect(() => {
    const controller = new AbortController();
    fetch(`${getApiBaseUrl()}/ai/models`, { signal: controller.signal })
      .then(async (response) => response.ok ? parseModelCatalog(await response.json()) : null)
      .then((catalog) => { if (!controller.signal.aborted) setModelCatalog(catalog); })
      .catch(() => { if (!controller.signal.aborted) setModelCatalog(null); })
      .finally(() => { if (!controller.signal.aborted) setModelsPending(false); });
    return () => controller.abort();
  }, []);
  const [, setContextRevision] = useState(0);

  const quillRef = useRef<Quill | null>(null);
  const aiRunStateRef = useRef(aiRunState);
  const abortControllerRef = useRef<AbortController | null>(null);
  const threadStateRef = useRef(threadState);
  const isMountedRef = useRef(true);
  const lastCursorRangeRef = useRef<{ index: number; length: number } | null>(null);
  const lastEditSelectionRef = useRef<{ index: number; length: number } | null>(null);
  const chapterSync = useChapterSync(quillRef, () =>
    ["continuing", "editing", "cancelling"].includes(aiRunStateRef.current.status), () => {
      lastCursorRangeRef.current = null;
      lastEditSelectionRef.current = null;
      clearContinueTint();
      setSectionOverrides({ generate: {}, edit: {} });
      setRange(quillRef.current?.getSelection() ?? null);
      updateThreadState(resetThread(threadStateRef.current));
    });

  const lore = useLore(chapterSync.scope?.seriesId ?? null);
  const loreEntries = suppliedLoreEntries ?? lore.entries;

  function persistEditorState() {
    if (typeof window === "undefined") {
      return;
    }

    try {
      window.localStorage.setItem(
        PERSISTED_EDITOR_STATE_KEY,
        serializePersistedEditorState({
          document: { ops: [] },
          storyContext,
          instruction,
          windowSize,
          windowMode,
          aiTextTint,
          showContinueWindow,
          threadState: threadStateRef.current,
        }),
      );
    } catch {}
  }

  useEffect(() => {
    aiRunStateRef.current = aiRunState;
  }, [aiRunState]);

  useEffect(() => {
    threadStateRef.current = threadState;
  }, [threadState]);

  useEffect(() => {
    const quill = quillRef.current;
    if (!quill) return;
    const refreshContext = (event: string, delta: Delta) => {
      if (event === Quill.events.TEXT_CHANGE && continueStreamRef.current && !applyingContinueRef.current) {
        if (lastCursorRangeRef.current) lastCursorRangeRef.current = transformContinueSelection(lastCursorRangeRef.current, delta);
        if (lastEditSelectionRef.current) lastEditSelectionRef.current = transformContinueSelection(lastEditSelectionRef.current, delta);
        continueStreamRef.current = transformContinueStream(continueStreamRef.current, delta);
        renderContinueTint(quill, continueStreamRef.current.removal, tintEnabledRef.current);
      }
      setContextRevision((revision) => revision + 1);
    };
    quill.on(Quill.events.EDITOR_CHANGE, refreshContext);
    return () => {
      quill.off(Quill.events.EDITOR_CHANGE, refreshContext);
    };
  }, []);

  useEffect(() => {
    persistEditorState();
  }, [storyContext, instruction, windowSize, windowMode, threadState, aiTextTint, showContinueWindow]);

  useEffect(() => {
    const media = window.matchMedia("(min-width: 1100px)");
    const changed = () => setDesktop(media.matches);
    media.addEventListener("change", changed);
    return () => media.removeEventListener("change", changed);
  }, []);

  useEffect(() => {
    const quill = quillRef.current;
    if (quill && continueStreamRef.current) renderContinueTint(quill, continueStreamRef.current.removal, aiTextTint && desktop);
  }, [aiTextTint, desktop]);

  useEffect(() => {
    isMountedRef.current = true;

    return () => {
      isMountedRef.current = false;
      abortControllerRef.current?.abort();
      clearContinueTint();
    };
  }, []);

  useEffect(() => {
    if (editBoxOpen) editInstructionRef.current?.focus();
  }, [editBoxOpen]);

  useEffect(() => {
    if (panel !== "settings") return;
    if (!desktop) {
      setPanel(null);
      return;
    }
    settingsRef.current?.querySelector<HTMLInputElement>("input:not(:disabled)")?.focus({ preventScroll: true });
    const dismiss = (event: PointerEvent) => {
      if (event.target instanceof Element && !settingsRef.current?.contains(event.target) && !event.target.closest(".editor-settings-button")) setPanel(null);
    };
    document.addEventListener("pointerdown", dismiss);
    return () => document.removeEventListener("pointerdown", dismiss);
  }, [panel, desktop]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      const action = panelShortcut(event);
      if (action === null) return;
      if (action === "close") {
        if (!panel && !editBoxOpen) return;
        event.preventDefault();
        closePanels();
        return;
      }
      event.preventDefault();
      togglePanel(action);
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [panel, editBoxOpen]);

  function closePanels() {
    setEditBoxOpen(false);
    setPanel(transitionPanel(panel, "close"));
    const selection = lastCursorRangeRef.current;
    if (selection) {
      quillRef.current?.setSelection(selection.index, selection.length, "silent");
      setRange(selection);
    } else {
      quillRef.current?.focus();
    }
  }

  function clearContinueTint() {
    if (tintTimerRef.current !== null) clearTimeout(tintTimerRef.current);
    tintTimerRef.current = null;
    continueStreamRef.current = null;
    const quill = quillRef.current;
    if (quill) {
      renderContinueTint(quill, new Delta(), false);
      delete quill.root.dataset.aiTint;
    }
  }

  function applyContinueChange(quill: Quill, change: Delta) {
    const selection = quill.getSelection();
    if (lastCursorRangeRef.current) lastCursorRangeRef.current = transformContinueSelection(lastCursorRangeRef.current, change);
    if (lastEditSelectionRef.current) lastEditSelectionRef.current = transformContinueSelection(lastEditSelectionRef.current, change);
    applyingContinueRef.current = true;
    try {
      quill.updateContents(change, "api");
      if (selection) {
        const next = transformContinueSelection(selection, change);
        quill.setSelection(next.index, next.length, "silent");
        lastCursorRangeRef.current = next;
        setRange(lastCursorRangeRef.current);
      }
    } finally {
      applyingContinueRef.current = false;
    }
  }

  function togglePanel(nextPanel: Exclude<PanelState, null>) {
    if (panel === nextPanel) closePanels();
    else setPanel(transitionPanel(panel, nextPanel));
  }

  const isBusy =
    aiRunState.status === "continuing" ||
    aiRunState.status === "editing" ||
    aiRunState.status === "cancelling";
  const hasSelection = Boolean(
    resolveEditSelection(
      range ? { index: range.index, length: range.length } : null,
      lastEditSelectionRef.current,
    ),
  );

  function updateAiRunState(nextState: AiRunState) {
    aiRunStateRef.current = nextState;
    if (isMountedRef.current) {
      setAiRunState(nextState);
    }
  }

  function setSelectionState(
    quill: Quill,
    selection: { index: number; length: number },
  ) {
    setRange(selection);
    lastEditSelectionRef.current = selection;
    lastCursorRangeRef.current = selection;
    quill.setSelection(selection.index, selection.length, "silent");
  }

  function setCursorState(quill: Quill, index: number) {
    quill.setSelection(index, 0, "silent");
    setRange({ index, length: 0 });
    lastCursorRangeRef.current = { index, length: 0 };
  }

  function preservePendingEditSelectionIfNeeded(
    quill: Quill,
    action: AiAction,
    didDeleteSelection: boolean,
    deletedLength: number,
    anchor: number,
  ): boolean {
    if (!shouldPreservePendingEditSelection(action, didDeleteSelection, deletedLength)) {
      return false;
    }

    setSelectionState(quill, { index: anchor, length: deletedLength });
    return true;
  }

  function getUnknownAiFailureMessage(error: unknown): string {
    return error instanceof Error ? error.message : "Unknown AI run failure";
  }

  function setAiFailure(action: AiAction | null, error: string) {
    updateAiRunState({ status: "failed", action, error });
  }

  function updateThreadState(nextState: AiThreadState) {
    threadStateRef.current = nextState;
    if (isMountedRef.current) {
      setThreadState(nextState);
    }
  }

  function normalizedInstructionText(): string | null {
    const trimmed = instruction.trim();
    return trimmed === "" ? null : trimmed;
  }

  function setInstructionAndResetThread(nextInstruction: string) {
    setInstruction(nextInstruction);
    updateThreadState(resetThreadOnInstructionChange(threadStateRef.current, nextInstruction.trim() || null));
  }

  function clearFailedRunState() {
    if (!shouldClearFailedRunState(aiRunStateRef.current, "user")) {
      return;
    }

    const reset = resetRunState(aiRunStateRef.current);
    if (reset.ok) {
      updateAiRunState(reset.state);
    }
  }

  function resetCurrentThread() {
    updateThreadState(resetThread(threadStateRef.current));
  }

  function buildRequestPreview(
    action: AiAction,
    quill: Quill,
    normalizedSelection: { index: number; length: number } | null,
  ): AiRequestPreview {
    const documentLength = Math.max(quill.getLength() - 1, 0);
    const configuredWindowSize = Number(windowSize);
    const documentWindow = calculateDocumentWindow({
      documentLength,
      selection: normalizedSelection,
      windowSize: configuredWindowSize,
      mode: windowMode,
    });

    const documentSlice = {
      delta: toContractDeltaDocument(
        quill.getContents(documentWindow.start, documentWindow.end - documentWindow.start),
      ),
      text: quill.getText(documentWindow.start, documentWindow.end - documentWindow.start),
    };

    const selectionSlice = normalizedSelection
      ? {
          delta: toContractDeltaDocument(
            quill.getContents(normalizedSelection.index, normalizedSelection.length),
          ),
          text: quill.getText(normalizedSelection.index, normalizedSelection.length),
          range: normalizedSelection,
        }
      : null;

    return action === "edit"
      ? buildAiRequestPreview({
          action: "edit",
          model: resolveModelChoice(modelCatalog, "edit", threadState.settings.editModel),
          loreEntries,
          sectionOverrides: sectionOverrides.edit,
          document: documentSlice,
          window: documentWindow,
          selection: selectionSlice!,
          storyContext,
          instruction,
          maxOutputTokens: threadState.settings.maxOutputTokens,
        })
      : buildAiRequestPreview({
          action: "generate",
          model: resolveModelChoice(modelCatalog, "generate", threadState.settings.continueModel),
          loreEntries,
          sectionOverrides: sectionOverrides.generate,
          document: documentSlice,
          window: documentWindow,
          selection: selectionSlice,
          storyContext,
          instruction,
          messages: shouldUseResumeThread(threadStateRef.current, normalizedInstructionText())
            ? historyForResume(threadStateRef.current)
            : [],
          resumePrompt: shouldUseResumeThread(threadStateRef.current, normalizedInstructionText())
            ? threadState.settings.resumePrompt
            : null,
          maxOutputTokens: threadState.settings.maxOutputTokens,
        });
  }

  async function runAiAction(action: AiAction) {
    if (!chapterSync.canStartAi()) {
      setAiFailure(action, "Wait for chapter saving and polling to finish before starting AI.");
      return;
    }
    const quill = quillRef.current;
    if (!quill) {
      setAiFailure(null, "Editor is not ready yet.");
      return;
    }

    const currentState = aiRunStateRef.current;
    const baseState =
      currentState.status === "finished" || currentState.status === "failed"
        ? resetRunState(currentState)
        : { ok: true as const, state: currentState };

    if (!baseState.ok) {
      setAiFailure(null, baseState.reason);
      return;
    }

    const started =
      action === "generate"
        ? startContinuing(baseState.state)
        : startEditing(baseState.state);

    if (!started.ok) {
      setAiFailure(null, started.reason);
      return;
    }

    const liveSelection = quill.getSelection() ?? range;
    const currentSelection = liveSelection
      ? { index: liveSelection.index, length: liveSelection.length }
      : null;
    const selection =
      action === "edit"
        ? resolveEditSelection(currentSelection, lastEditSelectionRef.current)
        : resolveContinueSelection(currentSelection, lastCursorRangeRef.current);

    if (action === "edit" && (!selection || selection.length === 0)) {
      const failed = failRun(started.state, "Select text before using Edit.");
      if (failed.ok) {
        updateAiRunState(failed.state);
      } else {
        setAiFailure(action, "Select text before using Edit.");
      }
      return;
    }

    const normalizedSelection = selection
      ? { index: selection.index, length: selection.length }
      : null;
    const documentLength = Math.max(quill.getLength() - 1, 0);
    const anchor =
      action === "edit"
        ? normalizedSelection?.index ?? 0
        : normalizedSelection?.index ?? documentLength;
    const deleteLength = action === "edit" ? normalizedSelection?.length ?? 0 : 0;
    const originalSelectionDelta =
      deleteLength > 0 ? new Delta(quill.getContents(anchor, deleteLength).ops) : null;
    let didDeleteSelection = false;
    const controller = new AbortController();
    const concurrentContinue = action === "generate" && desktop;
    clearContinueTint();
    if (concurrentContinue) {
      continueStreamRef.current = createContinueStream(quill.getText(), anchor);
      quill.root.dataset.aiTint = "streaming";
    }

    const rollbackContinue = () => {
      const stream = continueStreamRef.current;
      if (!concurrentContinue || !stream || !isActiveEditor()) return;
      applyContinueChange(quill, stream.removal);
      clearContinueTint();
    };

    abortControllerRef.current = controller;
    updateAiRunState(started.state);
    quill.enable(concurrentContinue);
    lastEditSelectionRef.current = null;

    const isActiveEditor = () => isMountedRef.current && quillRef.current === quill;
    let nextInsertIndex = anchor;

    try {
      const assertEditorActive = () => {
        if (!isActiveEditor()) {
          throw new DOMException("Editor is no longer active.", "AbortError");
        }
      };

      const preparedRequest = buildRequestPreview(action, quill, normalizedSelection);
      const request = preparedRequest.request;
      const runId = ++inspectorRunId.current;
      const inspectorRun = beginInspectorRun(runId, modelCatalog, request.model);
      setInspectorRuns(current => ({ ...current, [action]: inspectorRun }));
      if (action === "generate") {
        setActiveContinueWithThread(buildContextInspector(request).history.length > 0);
      }

      const response = await fetch(
        buildAiGenerateUrl(),
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(request),
          signal: controller.signal,
        },
      );

      if (!response.ok) {
        throw new Error(getAiErrorMessage(await response.text(), response.status));
      }

      setSectionOverrides((current) => ({ ...current, [action]: {} }));

      assertEditorActive();

      if (!response.body) {
        throw new Error("AI stream did not return a response body.");
      }

      if (deleteLength > 0) {
        quill.deleteText(anchor, deleteLength, "api");
        didDeleteSelection = true;
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let remainder = "";
      let terminalOutcome: "done" | "cancelled" | null = null;
      let finishReason: "stop" | "length" | "unknown" = "unknown";
      let generatedText = "";

      const applyEvents = (events: ReturnType<typeof parseAiStreamChunk>["events"]) => {
        for (const event of events) {
          if (event.type === "delta") {
            if (!isTextOnlyDeltaDocument(event.delta)) {
              throw new Error("AI stream delta must contain text inserts only.");
            }

            assertEditorActive();
            const fragmentText = event.delta.ops
              .map((op) => (typeof op.insert === "string" ? op.insert : ""))
              .join("");
            generatedText += fragmentText;
            const fragment = new Delta(toQuillDeltaDocument(event.delta));
            if (concurrentContinue && continueStreamRef.current) {
              const appended = appendContinueFragment(continueStreamRef.current, fragment);
              continueStreamRef.current = appended.state;
              applyContinueChange(quill, appended.change);
              renderContinueTint(quill, appended.state.removal, tintEnabledRef.current);
              nextInsertIndex = appended.state.end;
              continue;
            }
            quill.updateContents(
              new Delta().retain(nextInsertIndex).concat(fragment),
              "api",
            );
            nextInsertIndex += fragment.length();
            continue;
          }

          if (event.type === "error") {
            throw new Error(event.error);
          }

          if (event.type === "usage") {
            if (isActiveEditor()) setInspectorRuns(current => ({
              ...current, [action]: recordInspectorUsage(current[action], runId, event),
            }));
            continue;
          }

          if (event.type === "done") {
            finishReason = event.finishReason;
          }
          terminalOutcome = event.type === "cancelled" ? "cancelled" : "done";
          return;
        }
      };

      while (terminalOutcome === null) {
        const { value, done } = await reader.read();
        if (done) {
          break;
        }

        const parsedChunk = parseAiStreamChunk(
          decoder.decode(value, { stream: true }),
          remainder,
        );
        remainder = parsedChunk.remainder;

        if (!parsedChunk.ok) {
          throw new Error(parsedChunk.error.message);
        }

        applyEvents(parsedChunk.events);
      }

      if (terminalOutcome === null) {
        const parsedFinalChunk = parseAiStreamChunk(decoder.decode(), remainder);
        remainder = parsedFinalChunk.remainder;

        if (!parsedFinalChunk.ok) {
          throw new Error(parsedFinalChunk.error.message);
        }

        applyEvents(parsedFinalChunk.events);
      }

      if (terminalOutcome === null) {
        const finalized = finalizeAiStreamRemainder(remainder);
        if (!finalized.ok) {
          throw new Error(finalized.error.message);
        }

        applyEvents(finalized.events);
      }

      if (terminalOutcome === null) {
        throw new Error("AI stream ended before a terminal event arrived.");
      }

      const finished = finishRun(aiRunStateRef.current, terminalOutcome);
      if (!finished.ok) {
        throw new Error(finished.reason);
      }

      updateAiRunState(finished.state);
      if (concurrentContinue && terminalOutcome === "cancelled") {
        rollbackContinue();
        updateThreadState(finishCancelledThread(threadStateRef.current));
        return;
      }
      if (action === "generate") {
        updateThreadState(
          finishGenerateThread({
            state: threadStateRef.current,
            finishReason,
            instruction: request.instruction,
            userPrompt: request.resumePrompt ?? request.instruction,
            generatedText,
            generatedRange: { start: continueStreamRef.current?.anchor ?? anchor, end: continueStreamRef.current?.end ?? nextInsertIndex },
          }),
        );
      }
      assertEditorActive();

      if (
        originalSelectionDelta &&
        terminalOutcome === "cancelled" &&
        shouldRestoreCancelledEditSelection(
          action,
          didDeleteSelection,
          deleteLength,
          nextInsertIndex - anchor,
        )
      ) {
        quill.updateContents(
          new Delta().retain(anchor).concat(originalSelectionDelta),
          "api",
        );
        nextInsertIndex = anchor + originalSelectionDelta.length();
        setSelectionState(quill, { index: anchor, length: deleteLength });
        return;
      }

      if (!concurrentContinue) setCursorState(quill, nextInsertIndex);
    } catch (error) {
      const wasAborted =
        (error instanceof DOMException && error.name === "AbortError") ||
        (error instanceof Error && error.name === "AbortError");

      if (wasAborted) {
        const finished = finishRun(aiRunStateRef.current, "cancelled");
        if (finished.ok) {
          updateAiRunState(finished.state);
          updateThreadState(finishCancelledThread(threadStateRef.current));
        } else {
          setAiFailure(action, "Cancellation did not complete cleanly.");
        }

        if (concurrentContinue) {
          rollbackContinue();
          return;
        }

        if (
          originalSelectionDelta &&
          shouldRestoreCancelledEditSelection(
            action,
            didDeleteSelection,
            deleteLength,
            nextInsertIndex - anchor,
          )
          && isActiveEditor()
        ) {
          quill.updateContents(
            new Delta().retain(anchor).concat(originalSelectionDelta),
            "api",
          );
          setRange({ index: anchor, length: deleteLength });
          lastEditSelectionRef.current = { index: anchor, length: deleteLength };
          lastCursorRangeRef.current = { index: anchor, length: deleteLength };
          quill.setSelection(anchor, deleteLength, "silent");
          return;
        }

        if (isMountedRef.current && quillRef.current === quill) {
          if (
            preservePendingEditSelectionIfNeeded(
              quill,
              action,
              didDeleteSelection,
              deleteLength,
              anchor,
            )
          ) {
            return;
          }

          setCursorState(quill, nextInsertIndex);
        }
      } else {
        const failureMessage = getUnknownAiFailureMessage(error);
        const failed = failRun(aiRunStateRef.current, failureMessage);

        if (failed.ok) {
          updateAiRunState(failed.state);
        } else {
          setAiFailure(action, failureMessage);
        }

        if (concurrentContinue) {
          rollbackContinue();
          updateThreadState(finishCancelledThread(threadStateRef.current));
          return;
        }

        if (
          originalSelectionDelta &&
          shouldRollbackFailedEditSelection(action, didDeleteSelection, deleteLength)
          && isActiveEditor()
        ) {
          quill.updateContents(
            new Delta()
              .retain(anchor)
              .delete(nextInsertIndex - anchor)
              .concat(originalSelectionDelta),
            "api",
          );
          nextInsertIndex = anchor + originalSelectionDelta.length();
        }

        if (isMountedRef.current && quillRef.current === quill) {
          if (
            preservePendingEditSelectionIfNeeded(
              quill,
              action,
              didDeleteSelection,
              deleteLength,
              anchor,
            )
          ) {
            return;
          }

          setCursorState(quill, nextInsertIndex);
        }
      }
    } finally {
      if (concurrentContinue && continueStreamRef.current && isActiveEditor()) {
        quill.root.dataset.aiTint = "fading";
        tintTimerRef.current = setTimeout(clearContinueTint, 2050);
      }
      abortControllerRef.current = null;
      if (isMountedRef.current) setActiveContinueWithThread(null);
      if (isMountedRef.current && quillRef.current === quill) {
        quill.enable(true);
      }
    }
  }

  function cancelRun() {
    if (!abortControllerRef.current) {
      return;
    }

    const cancelling = startCancelling(aiRunStateRef.current);
    if (cancelling.ok) {
      updateAiRunState(cancelling.state);
    }

    abortControllerRef.current.abort();
  }

  function continueOrStop() {
    if (continueClickAction(aiRunStateRef.current) === "stop") cancelRun();
    else void runAiAction("generate");
  }

  const editAvailable = canApplyEdit(isBusy, hasSelection, chapterSync.ready, chapterSync.blocked);

  function applyEdit() {
    if (!canApplyEdit(continueClickAction(aiRunStateRef.current) === "stop", hasSelection, chapterSync.ready, chapterSync.blocked)) return;
    setEditBoxOpen(false);
    void runAiAction("edit");
  }

  const statusPresentation = getStatusPresentation(aiRunState);
  const savePresentation = getSavePresentation(chapterSync.status, chapterSync.title, chapterSync.blocked);
  const withThread = activeContinueWithThread ?? shouldContinueWithThread(
    threadState, normalizedInstructionText(), sectionOverrides.generate.history,
  );
  const isResumingOutput =
    aiRunState.status === "continuing" &&
    shouldUseResumeThread(threadState, normalizedInstructionText());
  const threadPresentation = getThreadPresentation(
    threadState,
    normalizedInstructionText(),
    isResumingOutput,
  );
  const contextInspector = (() => {
    const quill = quillRef.current;
    if (!quill || !chapterSync.ready) return "Editor is not ready yet.";
    try {
      const selection = (inspectorAction === "edit" ? resolveEditSelection : resolveContinueSelection)(
        quill.getSelection() ?? range,
        inspectorAction === "edit" ? lastEditSelectionRef.current : lastCursorRangeRef.current,
      );
      if (inspectorAction === "edit" && (!selection || selection.length === 0)) {
        return "Select text to inspect the Edit request.";
      }
      return buildContextInspector(buildRequestPreview(inspectorAction, quill, selection).request, authorGuidance, loreEntries);
    } catch (error) {
      return `Context unavailable: ${getUnknownAiFailureMessage(error)}`;
    }
  })();

  const inspectorCost = buildInspectorCost(modelCatalog,
    resolveModelChoice(modelCatalog, inspectorAction, inspectorAction === "edit" ? threadState.settings.editModel : threadState.settings.continueModel),
    typeof contextInspector === "string" ? null : contextInspector.totalTokens, inspectorRuns[inspectorAction]);

  const windowSettings = <>
    <label className="editor-field">
      <span>Window Size</span>
      <input className="editor-input" type="number" min="0" step="1" value={windowSize}
        onChange={(event) => setWindowSize(event.target.value)} disabled={isBusy} />
    </label>
    <label className="editor-toggle">
      <input type="checkbox" checked={windowMode === "document"}
        onChange={(event) => setWindowMode(event.target.checked ? "document" : "window")} disabled={isBusy} />
      <span>Whole document</span>
    </label>
  </>;

  const markerQuill = quillRef.current;
  const markerDocumentLength = Math.max((markerQuill?.getLength() ?? 1) - 1, 0);
  const markerSelection = markerQuill ? resolveContinueSelection(markerQuill.getSelection() ?? range, lastCursorRangeRef.current) : null;
  const markerWindow = calculateDocumentWindow({
    documentLength: markerDocumentLength,
    selection: markerSelection,
    windowSize: Number(windowSize),
    mode: windowMode,
  });

  const continueInspector = (() => {
    if (!desktop || !chapterSync.ready || !markerQuill) return null;
    try {
      return inspectorAction === "generate" && typeof contextInspector !== "string" ? contextInspector
        : buildContextInspector(buildRequestPreview("generate", markerQuill, markerSelection).request, authorGuidance, loreEntries);
    } catch {
      return null;
    }
  })();

  return (
    <div className="editor-shell">
      {desktop && showContinueWindow && chapterSync.ready && markerQuill ? <ContinueWindow
        quill={markerQuill} start={markerWindow.start} end={markerWindow.end} /> : null}
      <nav className="editor-rail editor-rail-left" aria-label="Navigation">
        <button type="button" aria-label="Manuscript" title="Manuscript (Ctrl/Cmd+[)"
          aria-controls="manuscript-drawer" aria-expanded={panel === "manuscript"}
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => togglePanel("manuscript")}>☰</button>
      </nav>
      <nav className="editor-rail editor-rail-right" aria-label="AI tools">
        <button type="button" className="editor-continue" aria-label={isBusy ? "Stop AI" : withThread ? "Continue writing with thread" : "Continue writing"}
          title={isBusy ? "Stop AI" : withThread ? "Continue writing with thread" : "Continue writing"}
          disabled={!isBusy && (!chapterSync.ready || chapterSync.blocked)}
          onMouseDown={(event) => event.preventDefault()}
          onClick={continueOrStop}>
          <span aria-hidden="true">{isBusy ? "■" : "➤"}</span>
          <span className="editor-continue-label">{isBusy ? "Stop" : "Continue"}</span>
          {withThread ? <span className="editor-thread-indicator">with thread</span> : null}
        </button>
        <button type="button" aria-label="Edit selection" title="Edit selection"
          disabled={isBusy || !hasSelection || !chapterSync.ready || chapterSync.blocked}
          onMouseDown={(event) => event.preventDefault()}
          aria-controls="edit-instruction-box" aria-expanded={editBoxOpen}
          onClick={() => setEditBoxOpen(true)}>✎</button>
        <button type="button" aria-label="Context inspector" title="Context inspector (Ctrl/Cmd+])"
          aria-controls="context-drawer" aria-expanded={panel === "context"}
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => togglePanel("context")}>◫</button>
        {desktop ? <button type="button" className="editor-settings-button" aria-label="Settings" title="Settings"
          aria-controls="editor-settings" aria-expanded={panel === "settings"} aria-haspopup="dialog"
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => togglePanel("settings")}>⚙</button> : null}
      </nav>
      {desktop && panel === "settings" ? <div id="editor-settings" ref={settingsRef}
        className="editor-settings-popover" role="dialog" aria-labelledby="editor-settings-title">
        <div className="editor-drawer-header">
          <h2 id="editor-settings-title">Settings</h2>
          <button className="editor-button editor-button-small" type="button" onClick={closePanels}>Close settings</button>
        </div>
        {windowSettings}
        <label className="editor-toggle">
          <input type="checkbox" checked={showContinueWindow} onChange={(event) => setShowContinueWindow(event.target.checked)} />
          <span>Show Continue window</span>
        </label>
        <label className="editor-toggle">
          <input type="checkbox" checked={aiTextTint} onChange={(event) => setAiTextTint(event.target.checked)} />
          <span>Tint AI text</span>
        </label>
      </div> : null}
      {editBoxOpen ? <div id="edit-instruction-box" className="editor-edit-box" role="dialog" aria-label="Edit instruction">
        <div className="editor-edit-flag">Edit selection</div>
        <StickyNote tone="pink" ref={editInstructionRef} aria-label="Edit instruction" rows={4}
          disabled={isBusy || !chapterSync.ready || chapterSync.blocked}
          aria-describedby="edit-instruction-hint" value={instruction}
          placeholder="How should the selected text change?"
          onChange={(event) => setInstructionAndResetThread(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
            event.preventDefault();
            applyEdit();
          }} />
        <p id="edit-instruction-hint" className="editor-note">Enter applies · Esc cancels · Shift+Enter adds a line</p>
        <div className="editor-actions">
          <button className="editor-button editor-button-small" type="button" onClick={closePanels}>Cancel edit</button>
          <button className="editor-button editor-button-primary editor-button-small" type="button"
            onClick={applyEdit}
            disabled={!editAvailable}>Apply</button>
        </div>
      </div> : null}
      {panel === "manuscript" ? <aside id="manuscript-drawer" className="editor-drawer editor-drawer-left"
        aria-labelledby="manuscript-title">
        <div className="editor-drawer-header">
          <h2 id="manuscript-title">Manuscript</h2>
        </div>
        <ManuscriptTree selectedChapter={chapterSync.scope} disabled={isBusy || !chapterSync.ready || chapterSync.blocked}
          onSelect={(scope) => void chapterSync.switchChapter(scope)} />
        {chapterSync.scope ? <LorePanel key={chapterSync.scope.seriesId} seriesId={chapterSync.scope.seriesId}
          lore={lore} disabled={isBusy || !chapterSync.ready || chapterSync.blocked} /> : <p>Select a chapter to manage series lore.</p>}
        <a className="editor-button" download="chapter-local.json"
          href="#" onClick={(event) => {
            event.currentTarget.href = `data:application/json;charset=utf-8,${encodeURIComponent(chapterSync.exportDocument())}`;
          }}>
          Export local chapter
        </a>
      </aside> : null}
      <div className="editor-topbar">
        <div className="editor-heading">
          <h1>QuillAI</h1>
          {chapterSync.needsChapter ? <button className="editor-button" type="button"
            onClick={() => void chapterSync.bootstrap()}>Create chapter</button> : null}
          {chapterSync.recovery ? <div role="alert">
            Previous unsaved work is available. Export it before closing this tab.
            <a download="chapter-recovery.json"
              href={`data:application/json;charset=utf-8,${encodeURIComponent(chapterSync.recovery)}`}>
              Export recovery
            </a>
          </div> : null}
        </div>
        <p className={`editor-save-status${savePresentation.failed ? " editor-save-failed" : ""}`}
          role="status" aria-label="Chapter save status" aria-live={savePresentation.failed ? "assertive" : "polite"}
          title={savePresentation.detail} aria-describedby={savePresentation.failed ? "chapter-save-detail" : undefined}>
          <span>{savePresentation.label}</span>
          {savePresentation.failed ? <span id="chapter-save-detail" style={{
            position: "absolute", width: 1, height: 1, overflow: "hidden", clipPath: "inset(50%)", whiteSpace: "nowrap",
          }}>{savePresentation.detail}</span> : null}
        </p>
        {aiRunState.status !== "idle" && aiRunState.status !== "finished" || aiRunState.status === "finished" && aiRunState.outcome === "cancelled" ? (
          <p className={`editor-run-status editor-status-${statusPresentation.tone}`} role="status">{statusPresentation.label}</p>
        ) : null}
      </div>

      <div className="editor-workspace">
        <div className="editor-page">
          {chapterSync.title !== null && <h2 className="editor-chapter-title">{chapterSync.title}</h2>}
          <div className="editor-canvas">
            <Component
              ref={quillRef}
              readOnly={(isBusy && !(desktop && aiRunState.status === "continuing" && continueStreamRef.current !== null)) || !chapterSync.ready || chapterSync.blocked}
              defaultValue={new Delta()}
              onSelectionChange={(nextRange) => {
                setRange(nextRange);
                lastCursorRangeRef.current = rememberCursorRange(
                  lastCursorRangeRef.current,
                  nextRange ? { index: nextRange.index, length: nextRange.length } : null,
                );
                lastEditSelectionRef.current = rememberEditSelection(
                  lastEditSelectionRef.current,
                  nextRange ? { index: nextRange.index, length: nextRange.length } : null,
                );
              }}
              onTextChange={(delta, _oldDelta, source) => {
                chapterSync.documentChanged();
                const quill = quillRef.current;
                if (quill) {
                  persistEditorState();
                }

                if (source === "user") {
                  updateThreadState(
                    invalidateThreadForManualEdit(threadStateRef.current, manualEditFromDelta(delta)),
                  );
                }

                if (!shouldClearFailedRunState(aiRunStateRef.current, source)) {
                  return;
                }

                clearFailedRunState();
              }}
            />
          </div>
        </div>
        <aside id="context-drawer" hidden={panel !== "context"}
          className="editor-drawer editor-drawer-right editor-context-inspector editor-preview-section" aria-labelledby="context-inspector-title">
          <div className="editor-drawer-header">
            <h2 id="context-inspector-title">Live Prompt Context</h2>
          </div>
          <div className="editor-context-body">
          {continueInspector && markerQuill ? <LoreWindow quill={markerQuill} start={markerWindow.start} lore={continueInspector.lore} /> : null}
        <label className="editor-field editor-field-full">
          <span>Story Context</span>
          <StickyNote
            tone="yellow"
            value={storyContext}
            onChange={(event) => setStoryContext(event.target.value)}
            placeholder="Series premise, tone, point of view, or scene notes"
            rows={4}
            disabled={isBusy || !chapterSync.ready || chapterSync.blocked}
          />
        </label>

        <div className="editor-controls-grid">
          <label className="editor-field editor-field-grow">
            <span>Instruction</span>
            <StickyNote
              tone="yellow"
              value={instruction}
              onChange={(event) => setInstructionAndResetThread(event.target.value)}
              placeholder="Continue the scene, tighten the selected paragraph, sharpen dialogue..."
              rows={2}
              disabled={isBusy}
            />
          </label>

          {!desktop ? windowSettings : null}
        </div>

        <div className={`editor-thread-card editor-thread-${threadPresentation.mode}`}>
          <div>
            <span className="editor-thread-label">AI Thread</span>
            <strong>{threadPresentation.label}</strong>
            <span>{threadPresentation.detail}</span>
          </div>
          <button
            className="editor-button editor-button-small"
            type="button"
            onClick={resetCurrentThread}
            disabled={isBusy || !threadPresentation.canReset}
          >
            Reset Thread
          </button>
        </div>

        <p className="editor-note">
          Edit replaces the current selection as text arrives. Stopping an Edit
          keeps text that already landed; failed edits restore the original passage.
          Desktop Continue Stop removes only AI text and keeps your typing.
        </p>
          <details className="editor-advanced-section">
            <summary>Advanced AI controls</summary>
            {(["generate", "edit"] as const).map((action) => {
              const setting = action === "generate" ? "continueModel" : "editModel";
              return <label className="editor-field" key={action}>
                <span>{action === "generate" ? "Continue model" : "Edit model"}</span>
                <select className="editor-input"
                  value={resolveModelChoice(modelCatalog, action, threadState.settings[setting]) ?? ""}
                  disabled={isBusy || !modelCatalog}
                  onChange={(event) => updateThreadState(updateThreadSettings(threadStateRef.current, {
                    [setting]: event.target.value,
                  }))}>
                  {!modelCatalog ? <option value="">Unavailable</option> : modelCatalog.models.map((model) =>
                    <option key={model.id} value={model.id}>{model.label}</option>)}
                </select>
              </label>;
            })}
            {!modelCatalog ? <p className="editor-note">{modelsPending ? "Loading models…" : "Models unavailable. Using server defaults."}</p> : null}
            <label className="editor-field">
              <span>Max Output Tokens</span>
              <input className="editor-input" type="number" min="1" max="4096" step="1"
                value={threadState.settings.maxOutputTokens} disabled={isBusy}
                onChange={(event) => updateThreadState(updateThreadSettings(threadStateRef.current, {
                  maxOutputTokens: Number(event.target.value),
                }))} />
            </label>
            <label className="editor-field">
              <span>Thread continuation prompt</span>
              <StickyNote tone="yellow" rows={2} value={threadState.settings.resumePrompt} disabled={isBusy}
                onChange={(event) => updateThreadState(updateThreadSettings(threadStateRef.current, {
                  resumePrompt: event.target.value,
                }))} />
            </label>
          </details>
          <label className="editor-field">
            <span>Author guidance</span>
            <StickyNote tone="yellow" aria-label="Author guidance" rows={5}
              value={guidanceDraft} disabled={isBusy || guidancePending || !guidanceLoaded}
              onChange={(event) => setGuidanceDraft(event.target.value)} />
          </label>
          <button className="editor-button editor-button-small" type="button"
            disabled={isBusy || guidancePending || !guidanceLoaded || guidanceDraft === authorGuidance}
            onClick={() => void saveAuthorGuidance()}>Save author guidance</button>
          <p className="editor-note">Saved on the server for all requests. Empty text removes guidance. Limit: 65,536 UTF-8 bytes. System rules are read-only.</p>
          <p className="editor-note" role="status">{guidanceStatus}</p>
          <label className="editor-field">
            <span>Request to inspect</span>
            <select className="editor-input" value={inspectorAction} disabled={isBusy}
              onChange={(event) => setInspectorAction(event.target.value === "edit" ? "edit" : "generate")}>
              <option value="generate">{withThread ? "Continue · with thread" : "Continue"}</option>
              <option value="edit">Edit selection</option>
            </select>
          </label>
          <p className="editor-note">
            Edit content for the next selected request. Tagged content is escaped; section tags and selection ranges stay fixed. History keeps separate speaker turns. Overrides reset after accepted sends or chapter changes. Limits: 65,536 UTF-8 bytes per section, 262,144 total.
          </p>
          <section className="editor-inspector-cost" aria-label="Model and cost">
            <h3>Pending estimate</h3>
            <dl>
              <dt>Model</dt><dd>{inspectorCost.model}</dd>
              <dt>Estimated input cost</dt><dd>{inspectorCost.estimate}</dd>
              <dt>Output rate</dt><dd>{inspectorCost.outputRate}</dd>
            </dl>
            <h3>Most recent run · actual</h3>
            <dl>
              <dt>Request model</dt><dd>{inspectorCost.actualModel}</dd>
              <dt>Observed usage</dt><dd>{inspectorCost.usage}</dd>
              <dt>Actual cost</dt><dd>{inspectorCost.actual}</dd>
            </dl>
            <p className="editor-note">Observed usage only, not a final bill. No cache discount assumed.</p>
          </section>
          {typeof contextInspector === "string" ? (
            <p className="editor-note">{contextInspector}</p>
          ) : (
            <>
              <p className="editor-note">{contextInspector.requestLabel}</p>
              <p className="editor-note">{contextInspector.tokenEstimateLabel}</p>
              <strong>Total: {contextInspector.totalTokens} estimated tokens</strong>
              <section className="editor-preview-section" aria-label="Injected lore">
                <div className="editor-preview-section-header"><h3>Injected lore</h3></div>
                <p className="editor-note">Next request · {contextInspector.lore.maxCharacters.toLocaleString()} UTF-16 character budget. Pins can exceed it.</p>
                <p className="editor-note">Tokens: UTF-8 bytes ÷ 4, rounded up per entry including standalone Lore tags; not additive.</p>
                {contextInspector.lore.injected.length === 0 ? <p>No lore injected.</p> : <ul>
                  {contextInspector.lore.injected.map(entry => <li key={entry.id}>
                    <strong>{entry.name}</strong> · {entry.kind} · {entry.reason === "pinned" ? "pin" : "keyword"} · {entry.level === "known_dont_reveal" ? "Known, don't reveal" : "Include"} · {entry.tokens} estimated tokens
                  </li>)}
                </ul>}
                <h4>Skipped by budget</h4>
                {contextInspector.lore.skipped.length === 0 ? <p>No entries skipped by budget.</p> : <ul>
                  {contextInspector.lore.skipped.map(entry => <li key={entry.id}>
                    <strong>{entry.name}</strong> · {entry.kind} · keyword · {entry.tokens} estimated tokens
                  </li>)}
                </ul>}
              </section>
              {contextInspector.sections.map((section) => (
                <section className="editor-preview-section" key={section.id}>
                  <div className="editor-preview-section-header">
                    <h3>{section.label}</h3>
                    <span>{section.tokens} estimated tokens</span>
                  </div>
                  {section.id === "system" || section.id === "author_guidance" || section.id === "lore" ? (
                    <textarea className="editor-textarea" aria-label={`${section.label} prompt section`}
                      value={section.text} rows={5} readOnly />
                  ) : section.id === "history" ? (
                    <>
                      {contextInspector.history.map((message, index) => (
                        <label className="editor-field" key={index}>
                          <span>{message.role} turn {index + 1}</span>
                          <textarea className="editor-textarea" value={message.text} rows={3}
                            disabled={isBusy || chapterSync.blocked}
                            onChange={(event) => setSectionOverrides((current) => ({
                              ...current,
                              [inspectorAction]: {
                                ...current[inspectorAction],
                                history: contextInspector.history.map((turn, turnIndex) =>
                                  turnIndex === index ? { ...turn, text: event.target.value } : turn),
                              },
                            }))} />
                        </label>
                      ))}
                      {contextInspector.history.length === 0 ? <p>No history turns.</p> : null}
                      {(["user", "assistant"] as const).map((role) => (
                        <button className="editor-button editor-button-small" type="button" key={role}
                          disabled={isBusy || chapterSync.blocked}
                          onClick={() => setSectionOverrides((current) => ({
                            ...current,
                            [inspectorAction]: {
                              ...current[inspectorAction],
                              history: [...contextInspector.history, { role, text: "" }],
                            },
                          }))}>Add {role} turn</button>
                      ))}
                    </>
                  ) : <textarea
                    className="editor-textarea"
                    aria-label={`${section.label} prompt section`}
                    value={section.editableText}
                    rows={5}
                    disabled={isBusy || chapterSync.blocked}
                    onChange={(event) => setSectionOverrides((current) => ({
                      ...current, [inspectorAction]: {
                        ...current[inspectorAction], [section.id]: event.target.value,
                      },
                    }))}
                  />}
                  {section.id !== "system" && section.id !== "author_guidance" && section.id !== "lore" && sectionOverrides[inspectorAction][section.id] !== undefined ? (
                    <button className="editor-button editor-button-small" type="button"
                      disabled={isBusy}
                      onClick={() => setSectionOverrides((current) => {
                        const next = { ...current[inspectorAction] };
                        if (section.id !== "system" && section.id !== "author_guidance" && section.id !== "lore") delete next[section.id];
                        return { ...current, [inspectorAction]: next };
                      })}>Reset {section.label}</button>
                  ) : null}
                </section>
              ))}
            </>
          )}
          </div>
        </aside>
      </div>

    </div>
  );
};

Container.displayName = "EditorContainer";

export default Container;
