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
  editReplacementLength,
  emptyContinueMessage,
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
import { getEditFailurePresentation } from "../edit_failure";
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
  threadScopeKeyFromSearch,
  type AiThreadState,
} from "../ai_thread";
import { calculateDocumentWindow, type DocumentWindowMode } from "../window";
import { useChapterSync } from "../use_chapter_sync";
import { useScriptSync } from "../use_script_sync";
import { getTabTitle } from "../../tab_title";
import { buildContextInspector } from "../context_inspector";
import { continueSelectionNote } from "../prompt_sections";
import { createSettingsClient } from "../../settings/client";
import { panelShortcut, transitionPanel, type PanelState } from "../editor_panels";
import { ManuscriptTree } from "./ManuscriptTree";
import { LorePanel } from "./LorePanel";
import { useLore } from "../../manuscripts/use_lore";
import { StickyNote } from "../../ui/StickyNote";
import { IconButton } from "../../ui/IconButton";
import { Alert } from "../../ui/Alert";
import { Button } from "../../ui/Button";
import { ButtonLink } from "../../ui/ButtonLink";
import { TextField } from "../../ui/TextField";
import { Field } from "../../ui/Field";
import { Heading } from "../../ui/Heading";
import { Toggle } from "../../ui/Toggle";
import { Popover } from "../../ui/Popover";
import { BLOCK_TYPES, blockLabel, parseBlock } from "../script_blocks";
import { editorUrl } from "../../landing/routes";
import { appendContinueFragment, createContinueStream, transformContinueSelection, transformContinueStream, type ContinueStream } from "../continue_stream";
import { editTintRemoval, renderContinueTint } from "../continue_tint";
import { trackEditSelectionHighlight } from "../edit_selection_highlight";
import { ContinueWindow } from "./ContinueWindow";
import { LoreWindow } from "./LoreWindow";
import { defaultLocale, formatMessage } from "../../i18n/messages";
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
  markdownShortcuts: boolean;
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
  ({ readOnly, defaultValue, onTextChange, onSelectionChange, markdownShortcuts }, ref) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const defaultValueRef = useRef(defaultValue);
    const initialReadOnlyRef = useRef(readOnly);
    const markdownShortcutsRef = useRef(markdownShortcuts);
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
        if (markdownShortcutsRef.current) processMarkdownSyntax(quill, delta, source);
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

export const Container = ({ loreEntries: suppliedLoreEntries, scriptGroupId }: {
  loreEntries?: readonly LoreMatchEntry[];
  scriptGroupId?: number;
}) => {
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
  const editTriggerRef = useRef<HTMLButtonElement>(null);
  const editorCanvasRef = useRef<HTMLDivElement>(null);
  const editAnchor = editTriggerRef.current?.getBoundingClientRect();
  const [storyContext, setStoryContext] = useState(initialEditorState.storyContext);
  const [editInstruction, setEditInstruction] = useState(initialEditorState.editInstruction);
  const [continueInstruction, setContinueInstruction] = useState(initialEditorState.continueInstruction);
  const [windowSize, setWindowSize] = useState(initialEditorState.windowSize);
  const [windowMode, setWindowMode] = useState<DocumentWindowMode>(initialEditorState.windowMode);
  const [aiTextTint, setAiTextTint] = useState(initialEditorState.aiTextTint);
  const [showContinueWindow, setShowContinueWindow] = useState(initialEditorState.showContinueWindow);
  const [desktop, setDesktop] = useState(() => window.matchMedia("(min-width: 1100px)").matches);
  const continueStreamRef = useRef<ContinueStream | null>(null);
  const [waitingForSave, setWaitingForSave] = useState(false);
  const applyingContinueRef = useRef(false);
  const tintTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const tintEnabledRef = useRef(aiTextTint);
  tintEnabledRef.current = aiTextTint;
  const [aiRunState, setAiRunState] = useState<AiRunState>(idleAiRunState);
  const [emptyContinueNotice, setEmptyContinueNotice] = useState<string | null>(null);
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
  const threadsRef = useRef(initialEditorState.threads);
  const threadScopeRef = useRef<string | null>(null);
  const isMountedRef = useRef(true);
  const lastCursorRangeRef = useRef<{ index: number; length: number } | null>(null);
  const lastEditSelectionRef = useRef<{ index: number; length: number } | null>(null);
  const isAiActive = () => ["continuing", "editing", "cancelling"].includes(aiRunStateRef.current.status);
  const onRemoteChange = () => {
    lastCursorRangeRef.current = null;
    lastEditSelectionRef.current = null;
    clearContinueTint();
    setSectionOverrides({ generate: {}, edit: {} });
    setRange(quillRef.current?.getSelection() ?? null);
    const key = threadScopeKeyFromSearch(window.location.search);
    if (threadScopeRef.current !== key) {
      threadScopeRef.current = key;
      updateThreadState({ ...threadStateRef.current, thread: key ? threadsRef.current[key] ?? null : null });
    } else {
      updateThreadState(resetThread(threadStateRef.current));
    }
  };
  const chapterSync = scriptGroupId === undefined
    ? useChapterSync(quillRef, isAiActive, onRemoteChange)
    : useScriptSync(quillRef, isAiActive, onRemoteChange, scriptGroupId);

  const tabTitle = getTabTitle(chapterSync.title, chapterSync.ready);
  useEffect(() => {
    document.title = tabTitle;
    return () => { document.title = "QuillAI"; };
  }, [tabTitle]);

  const lore = useLore(chapterSync.scope);
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
          editInstruction,
          continueInstruction,
          windowSize,
          windowMode,
          aiTextTint,
          showContinueWindow,
          threadState: threadStateRef.current,
          threads: threadsRef.current,
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
      if (event === Quill.events.TEXT_CHANGE && tintTimerRef.current !== null) clearContinueTint();
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
  }, [storyContext, editInstruction, continueInstruction, windowSize, windowMode, threadState, aiTextTint, showContinueWindow]);

  useEffect(() => {
    const media = window.matchMedia("(min-width: 1100px)");
    const changed = () => setDesktop(media.matches);
    media.addEventListener("change", changed);
    return () => media.removeEventListener("change", changed);
  }, []);

  useEffect(() => {
    const quill = quillRef.current;
    if (!aiTextTint || !desktop) {
      if (tintTimerRef.current !== null) clearContinueTint();
    }
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
    const quill = quillRef.current;
    if (!editBoxOpen || !quill) return;
    return trackEditSelectionHighlight(quill, lastEditSelectionRef.current);
  }, [editBoxOpen]);

  useEffect(() => {
    if (panel !== "settings") return;
    if (!desktop) {
      setPanel(null);
      return;
    }
    settingsRef.current?.querySelector<HTMLInputElement>("input:not(:disabled)")?.focus({ preventScroll: true });
  }, [panel, desktop]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      if (event.key === "Escape" && event.target instanceof Element && event.target.closest("[data-manuscript-draft]")) return;
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
  const editSelection = resolveEditSelection(
    range ? { index: range.index, length: range.length } : null,
    lastEditSelectionRef.current,
  );
  const hasSelection = Boolean(editSelection);
  const editRejection = editSelection ? chapterSync.editRejection(editSelection) : null;

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
    const key = threadScopeRef.current;
    if (key) {
      if (nextState.thread) threadsRef.current = { ...threadsRef.current, [key]: nextState.thread };
      else {
        const threads = { ...threadsRef.current };
        delete threads[key];
        threadsRef.current = threads;
      }
    }
    threadStateRef.current = nextState;
    if (isMountedRef.current) {
      setThreadState(nextState);
    }
  }

  function normalizedInstructionText(): string | null {
    const trimmed = continueInstruction.trim();
    return trimmed === "" ? null : trimmed;
  }

  function setContinueInstructionAndResetThread(nextInstruction: string) {
    setContinueInstruction(nextInstruction);
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
          instruction: editInstruction,
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
          instruction: continueInstruction,
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
    const reservation = chapterSync.reserveAi();
    if (!reservation) {
      setAiFailure(action, chapterSync.status);
      return;
    }
    try {
      await runReservedAiAction(action, reservation);
    } finally {
      reservation?.release();
    }
  }

  async function runReservedAiAction(action: AiAction, reservation: ReturnType<typeof chapterSync.reserveAi>) {
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

    clearContinueTint();
    setEmptyContinueNotice(null);
    const controller = new AbortController();
    if (reservation) {
      setWaitingForSave(true);
      abortControllerRef.current = controller;
      updateAiRunState(started.state);
      try {
        await reservation.wait(controller.signal);
        if (!isMountedRef.current || quillRef.current !== quill) return;
      } catch (error) {
        if (isMountedRef.current) {
          if (controller.signal.aborted) {
            const finished = finishRun(aiRunStateRef.current, "cancelled");
            if (finished.ok) updateAiRunState(finished.state);
          } else {
            setAiFailure(action, getUnknownAiFailureMessage(error));
          }
        }
        return;
      } finally {
        if (isMountedRef.current) setWaitingForSave(false);
        abortControllerRef.current = null;
      }
    }

    const liveSelection = quill.getSelection() ?? range;
    const currentSelection = liveSelection
      ? { index: liveSelection.index, length: liveSelection.length }
      : null;
    const selection =
      action === "edit"
        ? resolveEditSelection(currentSelection, lastEditSelectionRef.current)
        : resolveContinueSelection(currentSelection, lastCursorRangeRef.current);

    const editFailure = action !== "edit" ? null
      : !selection || selection.length === 0 ? "Select text before using Edit." : chapterSync.editRejection(selection);
    if (editFailure) {
      const failed = failRun(started.state, editFailure);
      if (failed.ok) {
        updateAiRunState(failed.state);
      } else {
        setAiFailure(action, editFailure);
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
    const selectionLength = action === "edit" ? normalizedSelection?.length ?? 0 : 0;
    const selectedDelta =
      selectionLength > 0 ? new Delta(quill.getContents(anchor, selectionLength).ops) : null;
    const deleteLength = selectedDelta ? editReplacementLength(selectedDelta.ops) : 0;
    const originalSelectionDelta = selectedDelta?.slice(0, deleteLength) ?? null;
    const keepsLineEnding = deleteLength < selectionLength;
    let heldLineEnding: Delta | null = null;
    let didDeleteSelection = false;
    const concurrentContinue = action === "generate" && desktop;
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

      if (originalSelectionDelta) {
        if (deleteLength > 0) quill.deleteText(anchor, deleteLength, "api");
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
            let fragment = new Delta(toQuillDeltaDocument(event.delta));
            if (keepsLineEnding) {
              if (heldLineEnding) fragment = heldLineEnding.concat(fragment);
              const end = fragment.length() - 1;
              heldLineEnding = end >= 0 && fragment.slice(end).ops[0]?.insert === "\n" ? fragment.slice(end) : null;
              if (heldLineEnding) fragment = fragment.slice(0, end);
            }
            if (concurrentContinue && continueStreamRef.current) {
              const appended = appendContinueFragment(continueStreamRef.current, fragment, quill.getText());
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
      if (action === "generate" && terminalOutcome === "done" && generatedText.length === 0) {
        setEmptyContinueNotice(emptyContinueMessage(finishReason));
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
          selectionLength,
          nextInsertIndex - anchor,
        )
      ) {
        quill.updateContents(
          new Delta().retain(anchor).concat(originalSelectionDelta),
          "api",
        );
        nextInsertIndex = anchor + originalSelectionDelta.length();
        setSelectionState(quill, { index: anchor, length: selectionLength });
        return;
      }

      if (!concurrentContinue) setCursorState(quill, nextInsertIndex);
      if (action === "edit" && terminalOutcome === "done") {
        const removal = editTintRemoval(anchor, nextInsertIndex, tintEnabledRef.current && window.matchMedia("(min-width: 1100px)").matches);
        if (removal) {
          renderContinueTint(quill, removal, true);
          quill.root.dataset.aiTint = "streaming";
          getComputedStyle(quill.root).getPropertyValue("--ai-stream-color");
          quill.root.dataset.aiTint = "fading";
          tintTimerRef.current = setTimeout(clearContinueTint, 2050);
        }
      }
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
            selectionLength,
            nextInsertIndex - anchor,
          )
          && isActiveEditor()
        ) {
          quill.updateContents(
            new Delta().retain(anchor).concat(originalSelectionDelta),
            "api",
          );
          setSelectionState(quill, { index: anchor, length: selectionLength });
          return;
        }

        if (isMountedRef.current && quillRef.current === quill) {
          if (
            preservePendingEditSelectionIfNeeded(
              quill,
              action,
              didDeleteSelection,
              selectionLength,
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
          shouldRollbackFailedEditSelection(action, didDeleteSelection, selectionLength)
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
              selectionLength,
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

  const editAvailable = canApplyEdit(isBusy, hasSelection, chapterSync.ready, chapterSync.blocked) && editRejection === null;

  function applyEdit() {
    if (editRejection !== null || !canApplyEdit(continueClickAction(aiRunStateRef.current) === "stop", hasSelection, chapterSync.ready, chapterSync.blocked)) return;
    setEditBoxOpen(false);
    void runAiAction("edit");
  }

  const statusPresentation = getStatusPresentation(aiRunState);
  const editFailure = aiRunState.status === "failed" && aiRunState.action === "edit"
    ? getEditFailurePresentation(aiRunState.error)
    : null;
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
  const inspectorSelection = quillRef.current?.getSelection() ?? range ?? lastCursorRangeRef.current;
  const showContinueSelectionNote = inspectorAction === "generate" && (inspectorSelection?.length ?? 0) > 0
    && sectionOverrides.generate.selection === undefined;
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
      const inspector = buildContextInspector(buildRequestPreview(inspectorAction, quill, selection).request, authorGuidance, loreEntries);
      if (!showContinueSelectionNote || !inspectorSelection) return inspector;
      const note = continueSelectionNote(quill.getText(inspectorSelection.index, inspectorSelection.length), defaultLocale);
      return { ...inspector, sections: inspector.sections.map(section =>
        section.id === "selection" ? { ...section, editableText: note } : section) };
    } catch (error) {
      return `Context unavailable: ${getUnknownAiFailureMessage(error)}`;
    }
  })();

  const inspectorCost = buildInspectorCost(modelCatalog,
    resolveModelChoice(modelCatalog, inspectorAction, inspectorAction === "edit" ? threadState.settings.editModel : threadState.settings.continueModel),
    typeof contextInspector === "string" ? null : contextInspector.totalTokens, inspectorRuns[inspectorAction]);

  const windowSettings = <>
    <Field label="Window Size">
      <TextField type="number" min="0" step="1" value={windowSize}
        onChange={(event) => setWindowSize(event.target.value)} disabled={isBusy} />
    </Field>
    <Toggle label="Whole document" checked={windowMode === "document"}
      onChange={(event) => setWindowMode(event.target.checked ? "document" : "window")} disabled={isBusy} />
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
        <IconButton label="Library" title="Back to library" glyph="⌂" disabled={isBusy}
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => void chapterSync.leave()} />
        <IconButton label="Manuscript" glyph={<span aria-hidden="true">☰</span>} title="Manuscript (Ctrl/Cmd+[)"
          aria-controls="manuscript-drawer" aria-expanded={panel === "manuscript"}
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => togglePanel("manuscript")} />
      </nav>
      <nav className="editor-rail editor-rail-right" aria-label="AI tools">
        <IconButton kind="continue" className="editor-continue" label={waitingForSave ? formatMessage(defaultLocale, "action.saving") : isBusy ? "Stop" : "Continue"} glyph={<span aria-hidden="true">{isBusy ? "■" : "➤"}</span>}
          badge={withThread ? <span className="editor-thread-indicator text-micro">{formatMessage(defaultLocale, "thread.resumeIndicator")}</span> : null}
          aria-label={isBusy ? "Stop AI" : withThread ? formatMessage(defaultLocale, "thread.resumeAction") : "Continue writing"}
          title={waitingForSave ? formatMessage(defaultLocale, "action.saving") : isBusy ? "Stop AI" : withThread ? formatMessage(defaultLocale, "thread.resumeAction") : "Continue writing"}
          disabled={!isBusy && (!chapterSync.ready || chapterSync.blocked)}
          onMouseDown={(event) => event.preventDefault()}
          onClick={continueOrStop} />
        <Popover anchor={{ left: editAnchor?.left ?? 0, top: editAnchor?.top ?? 0 }}
          ignoreRefs={[editorCanvasRef]}
          open={editBoxOpen} onOpenChange={(open) => open ? setEditBoxOpen(true) : closePanels()}
          className="editor-edit-box" role="dialog" aria-label="Edit instruction"
          trigger={<IconButton ref={editTriggerRef} label="Edit selection" glyph={<span aria-hidden="true">✎</span>} title="Edit selection"
            disabled={isBusy || !hasSelection || !chapterSync.ready || chapterSync.blocked}
            onMouseDown={(event) => event.preventDefault()} />}>
        <div className="editor-edit-flag">Edit selection</div>
        <div className="-mx-4">
          <StickyNote tone="pink" overhang ref={editInstructionRef} aria-label="Edit instruction" rows={4}
            disabled={isBusy || !chapterSync.ready || chapterSync.blocked}
            aria-describedby="edit-instruction-hint" value={editInstruction}
            placeholder="How should the selected text change?"
            onChange={(event) => setEditInstruction(event.target.value)}
            onKeyDown={(event) => {
              if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
              event.preventDefault();
              applyEdit();
            }} />
        </div>
        <p id="edit-instruction-hint" className="editor-note">Enter applies · Esc or Edit closes · Shift+Enter adds a line</p>
        {editRejection ? <p id="edit-instruction-rejection" className="editor-note" role="status">{editRejection}</p> : null}
        <div className="editor-actions">
          <Button variant="primary" size="sm"
            onClick={applyEdit} aria-describedby={editRejection ? "edit-instruction-rejection" : undefined}
            disabled={!editAvailable}>Apply</Button>
        </div>
        </Popover>
        <IconButton label="Context inspector" glyph={<span aria-hidden="true">◫</span>} title="Context inspector (Ctrl/Cmd+])"
          aria-controls="context-drawer" aria-expanded={panel === "context"}
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => togglePanel("context")} />
        {desktop ? <Popover open={panel === "settings"} onOpenChange={(open) => { if (!open) setPanel(null); }}
          ref={settingsRef} className="editor-settings-popover flex flex-col gap-4" role="dialog" aria-labelledby="editor-settings-title"
          trigger={<IconButton className="editor-settings-button" label="Settings" glyph={<span aria-hidden="true">⚙</span>} title="Settings" aria-haspopup="dialog"
            onMouseDown={(event) => event.preventDefault()}
            onClick={(event) => { event.preventDefault(); togglePanel("settings"); }} />}>
          <div className="editor-drawer-header">
            <Heading level="section" id="editor-settings-title">Settings</Heading>
          </div>
          {windowSettings}
          <Toggle label="Show Continue window" checked={showContinueWindow} onChange={(event) => setShowContinueWindow(event.target.checked)} />
          <Toggle label="Tint AI text" checked={aiTextTint} onChange={(event) => setAiTextTint(event.target.checked)} />
        </Popover> : null}
      </nav>
      {panel === "manuscript" ? <aside id="manuscript-drawer" className="editor-drawer editor-drawer-left"
        aria-labelledby="manuscript-title">
        <div className="editor-drawer-header">
          <Heading level="section" id="manuscript-title">Manuscript</Heading>
        </div>
        <div className="editor-manuscript-outline" role="region" aria-label="Manuscript outline" tabIndex={0}>
          <ManuscriptTree selectedChapter={"treeScope" in chapterSync ? chapterSync.treeScope : chapterSync.scope} savesBeforeOpening disabled={isBusy || (!chapterSync.ready && !chapterSync.needsChapter) || chapterSync.blocked}
            onSelect={(scope, opensScript) => {
              if ("focusScene" in chapterSync && "pieceId" in scope && chapterSync.focusScene(scope.pieceId)) return;
              if (opensScript && !chapterSync.needsChapter) void chapterSync.leave(editorUrl(window.location.href, scope));
              else if (chapterSync.needsChapter) {
                const url = new URL(window.location.href);
                for (const [key, value] of Object.entries(scope)) url.searchParams.set(key, String(value));
                window.location.assign(url);
              } else void chapterSync.switchChapter(scope);
            }} />
        </div>
        {chapterSync.scope ? <LorePanel key={lore.universeId ?? "loading"}
          lore={lore} disabled={isBusy || !chapterSync.ready || chapterSync.blocked} /> : <p>Open a piece or chapter to manage its universe lore.</p>}
        <ButtonLink className="shrink-0 self-start" download="chapter-local.json"
          href="#" onClick={(event) => {
            event.currentTarget.href = `data:application/json;charset=utf-8,${encodeURIComponent(chapterSync.exportDocument())}`;
          }}>
          Export local chapter
        </ButtonLink>
      </aside> : null}
      <div className="editor-topbar">
        <div className="editor-heading">
          <Heading level="app">QuillAI</Heading>
          {chapterSync.needsChapter ? <Button variant="primary"
            onClick={() => void chapterSync.bootstrap()}>Create chapter</Button> : null}
          {chapterSync.recovery ? <Alert tone="inline">
            Previous unsaved work is available. Export it before closing this tab.
            <a download="chapter-recovery.json"
              href={`data:application/json;charset=utf-8,${encodeURIComponent(chapterSync.recovery)}`}>
              Export recovery
            </a>
          </Alert> : null}
        </div>
        {savePresentation.failed ? <Alert tone="fail" className="editor-save-failed" aria-label="Chapter save status" aria-live="assertive"
          title={savePresentation.detail} aria-describedby="chapter-save-detail">
          <span>{savePresentation.label}</span>
          <span id="chapter-save-detail" className="sr-only">{savePresentation.detail}</span>
        </Alert> : <p className="editor-save-status"
          role="status" aria-label="Chapter save status" aria-live="polite" title={savePresentation.detail}>
          <span>{savePresentation.label}</span>
        </p>}
        {"addScene" in chapterSync ? <Button size="sm" disabled={isBusy || !chapterSync.ready || chapterSync.blocked || chapterSync.adding}
          onMouseDown={(event) => event.preventDefault()} onClick={() => void chapterSync.addScene()}>New scene</Button> : null}
        {"addScene" in chapterSync ? <div className="editor-script-tools">
          <TextField as="select" aria-label="Block type" value={chapterSync.blockType}
            disabled={isBusy || !chapterSync.ready || chapterSync.blocked}
            onChange={(event) => chapterSync.setBlock(parseBlock(event.target.value))}>
            {BLOCK_TYPES.map((type) => <option key={type} value={type}>{blockLabel(type)}</option>)}
          </TextField>
          <ButtonLink size="sm" download={`${chapterSync.title || "script"}.fountain`} href="#"
            onClick={(event) => {
              event.currentTarget.href = `data:text/plain;charset=utf-8,${encodeURIComponent(chapterSync.fountain())}`;
            }}>Download Fountain</ButtonLink>
          <ButtonLink size="sm" download={`${chapterSync.title || "script"}.fdx`} href="#"
            onClick={(event) => {
              event.currentTarget.href = `data:application/xml;charset=utf-8,${encodeURIComponent(chapterSync.fdx())}`;
            }}>Download FDX</ButtonLink>
        </div> : null}
        {chapterSync.notice ? <Alert tone="fail" className="editor-scene-notice">{chapterSync.notice}</Alert> : null}
        {emptyContinueNotice ? <Alert tone="fail" className="editor-scene-notice">{emptyContinueNotice}</Alert> : null}
        {aiRunState.status !== "idle" && aiRunState.status !== "finished" || aiRunState.status === "finished" && aiRunState.outcome === "cancelled" ? (
          editFailure ? <Alert tone="fail" className={`editor-run-status editor-status-${statusPresentation.tone}`} title={editFailure.title}>{editFailure.label}</Alert>
            : <p className={`editor-run-status editor-status-${statusPresentation.tone}`} role="status">{statusPresentation.label}</p>
        ) : null}
      </div>

      <div className="editor-workspace">
        <div className="editor-page">
          {chapterSync.title !== null && <h2 className="editor-chapter-title">{chapterSync.title}</h2>}
          <div ref={editorCanvasRef} className="editor-canvas">
            <Component
              ref={quillRef}
              readOnly={(isBusy && !waitingForSave && !(desktop && aiRunState.status === "continuing" && continueStreamRef.current !== null)) || !chapterSync.ready || chapterSync.blocked}
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
              markdownShortcuts={scriptGroupId === undefined}
              onTextChange={(delta, oldDelta, source) => {
                chapterSync.documentChanged(delta, oldDelta);
                const quill = quillRef.current;
                if (quill) {
                  persistEditorState();
                }

                if (source === "user") {
                  if (quill && emptyContinueNotice) queueMicrotask(() => {
                    if (oldDelta.diff(quill.getContents()).ops.length > 0) setEmptyContinueNotice(null);
                  });
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
            <Heading level="section" id="context-inspector-title">Live Prompt Context</Heading>
          </div>
          <div className="editor-context-body">
          {continueInspector && markerQuill ? <LoreWindow quill={markerQuill} start={markerWindow.start} lore={continueInspector.lore} /> : null}
        <Field label="Story Context" className="w-full">
          <StickyNote
            tone="yellow"
            value={storyContext}
            onChange={(event) => setStoryContext(event.target.value)}
            placeholder="Series premise, tone, point of view, or scene notes"
            rows={4}
            disabled={isBusy || !chapterSync.ready || chapterSync.blocked}
          />
        </Field>

        <div className="editor-controls-grid">
          <Field label="Continue instruction" className="min-w-0">
            <StickyNote
              tone="yellow"
              value={continueInstruction}
              onChange={(event) => setContinueInstructionAndResetThread(event.target.value)}
              placeholder="Continue the scene, build suspense, introduce a new obstacle..."
              rows={2}
              disabled={isBusy}
            />
          </Field>

          {!desktop ? windowSettings : null}
        </div>

        {threadPresentation.mode !== "completion" ? <div className={`editor-thread-card editor-thread-${threadPresentation.mode}`}>
          <div>
            <strong>{formatMessage(defaultLocale, "thread.cutOff")}</strong>
          </div>
          <Button size="sm" onClick={resetCurrentThread} disabled={isBusy}>
            {formatMessage(defaultLocale, "thread.discard")}
          </Button>
        </div> : null}

        <p className="editor-note">
          Edit replaces the current selection as text arrives. Stopping an Edit
          keeps text that already landed; failed edits restore the original passage.
          Desktop Continue Stop removes only AI text and keeps your typing.
        </p>
          <details className="editor-advanced-section">
            <summary>Advanced AI controls</summary>
            {(["generate", "edit"] as const).map((action) => {
              const setting = action === "generate" ? "continueModel" : "editModel";
              return <Field className="mt-3" label={action === "generate" ? "Continue model" : "Edit model"} key={action}>
                <TextField as="select" square
                  value={resolveModelChoice(modelCatalog, action, threadState.settings[setting]) ?? ""}
                  disabled={isBusy || !modelCatalog}
                  onChange={(event) => updateThreadState(updateThreadSettings(threadStateRef.current, {
                    [setting]: event.target.value,
                  }))}>
                  {!modelCatalog ? <option value="">Unavailable</option> : modelCatalog.models.map((model) =>
                    <option key={model.id} value={model.id}>{model.label}</option>)}
                </TextField>
              </Field>;
            })}
            {!modelCatalog ? <p className="editor-note">{modelsPending ? "Loading models…" : "Models unavailable. Using server defaults."}</p> : null}
            <Field label="Max Output Tokens" className="mt-3">
              <TextField square type="number" min="1" max="4096" step="1"
                value={threadState.settings.maxOutputTokens} disabled={isBusy}
                onChange={(event) => updateThreadState(updateThreadSettings(threadStateRef.current, {
                  maxOutputTokens: Number(event.target.value),
                }))} />
            </Field>
            <Field label="Thread continuation prompt" className="mt-3">
              <StickyNote tone="yellow" rows={2} value={threadState.settings.resumePrompt} disabled={isBusy}
                onChange={(event) => updateThreadState(updateThreadSettings(threadStateRef.current, {
                  resumePrompt: event.target.value,
                }))} />
            </Field>
          </details>
          <Field label="Author guidance">
            <StickyNote tone="yellow" aria-label="Author guidance" rows={5}
              value={guidanceDraft} disabled={isBusy || guidancePending || !guidanceLoaded}
              onChange={(event) => setGuidanceDraft(event.target.value)} />
          </Field>
          <Button size="sm"
            disabled={isBusy || guidancePending || !guidanceLoaded || guidanceDraft === authorGuidance}
            onClick={() => void saveAuthorGuidance()}>Save author guidance</Button>
          <p className="editor-note">Saved on the server for all requests. Empty text removes guidance. Limit: 65,536 UTF-8 bytes. System rules are read-only.</p>
          <p className="editor-note" role="status">{guidanceStatus}</p>
          <Field label="Request to inspect">
            <TextField as="select" square value={inspectorAction} disabled={isBusy}
              onChange={(event) => setInspectorAction(event.target.value === "edit" ? "edit" : "generate")}>
              <option value="generate">{withThread ? formatMessage(defaultLocale, "thread.resumeOption") : "Continue"}</option>
              <option value="edit">Edit selection</option>
            </TextField>
          </Field>
          <p className="editor-note">
            Edit content for the next selected request. Tagged content is escaped; section tags and selection ranges stay fixed. History keeps separate speaker turns. Overrides reset after accepted sends or chapter changes. Limits: 65,536 UTF-8 bytes per section, 262,144 total.
          </p>
          <section className="space-y-3 text-ui break-normal" aria-label="Model and cost">
            <Heading level="section" as="h3">Pending estimate</Heading>
            <dl className="grid grid-cols-1 gap-1">
              <dt className="text-label text-graphite">Model</dt><dd title={inspectorCost.modelId}>{inspectorCost.model}</dd>
              <dt className="text-label text-graphite">Estimated input cost</dt><dd>{inspectorCost.estimate}</dd>
              <dt className="text-label text-graphite">Output rate</dt><dd>{inspectorCost.outputRate}</dd>
            </dl>
            <Heading level="section" as="h3">Most recent run · actual</Heading>
            <dl className="grid grid-cols-1 gap-1">
              {inspectorCost.actualModel !== null && <><dt className="text-label text-graphite">Request model</dt><dd>{inspectorCost.actualModel}</dd></>}
              <dt className="text-label text-graphite">Observed usage</dt><dd>{inspectorCost.usage}</dd>
              <dt className="text-label text-graphite">Actual cost</dt><dd>{inspectorCost.actual}</dd>
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
                <div className="editor-preview-section-header"><Heading level="section" as="h3">Injected lore</Heading></div>
                <p className="editor-note">Next request · {contextInspector.lore.maxCharacters.toLocaleString()} UTF-16 character budget. Pins can exceed it.</p>
                <p className="editor-note">Tokens: UTF-8 bytes ÷ 4, rounded up per entry including standalone Lore tags; not additive.</p>
                {contextInspector.lore.injected.length === 0 ? <p>No lore injected.</p> : <ul>
                  {contextInspector.lore.injected.map(entry => <li key={entry.id}>
                    <strong>{entry.name}</strong> · {entry.kind} · {entry.reason === "pinned" ? "pin" : "keyword"} · {entry.level === "known_dont_reveal" ? "Known, don't reveal" : "Include"} · {entry.tokens} estimated tokens
                  </li>)}
                </ul>}
                <Heading level="section" as="h4">Skipped by budget</Heading>
                {contextInspector.lore.skipped.length === 0 ? <p>No entries skipped by budget.</p> : <ul>
                  {contextInspector.lore.skipped.map(entry => <li key={entry.id}>
                    <strong>{entry.name}</strong> · {entry.kind} · keyword · {entry.tokens} estimated tokens
                  </li>)}
                </ul>}
              </section>
              {contextInspector.sections.map((section) => (
                <section className="editor-preview-section" key={section.id}>
                  <div className="editor-preview-section-header">
                    <Heading level="section" as="h3">{section.label}</Heading>
                    <span>{section.tokens} estimated tokens</span>
                  </div>
                  {section.id === "system" || section.id === "author_guidance" || section.id === "lore" ? (
                    <TextField as="textarea" square className="font-mono text-graphite" aria-label={`${section.label} prompt section`}
                      value={section.text} rows={5} readOnly />
                  ) : section.id === "history" ? (
                    <>
                      {contextInspector.history.map((message, index) => (
                        <Field label={`${message.role} turn ${index + 1}`} key={index}>
                          <StickyNote tone="yellow" value={message.text} rows={3}
                            disabled={isBusy || chapterSync.blocked}
                            onChange={(event) => setSectionOverrides((current) => ({
                              ...current,
                              [inspectorAction]: {
                                ...current[inspectorAction],
                                history: contextInspector.history.map((turn, turnIndex) =>
                                  turnIndex === index ? { ...turn, text: event.target.value } : turn),
                              },
                            }))} />
                        </Field>
                      ))}
                      {contextInspector.history.length === 0 ? <p>No history turns.</p> : null}
                      {(["user", "assistant"] as const).map((role) => (
                        <Button size="sm" key={role}
                          disabled={isBusy || chapterSync.blocked}
                          onClick={() => setSectionOverrides((current) => ({
                            ...current,
                            [inspectorAction]: {
                              ...current[inspectorAction],
                              history: [...contextInspector.history, { role, text: "" }],
                            },
                          }))}>Add {role} turn</Button>
                      ))}
                    </>
                  ) : <StickyNote
                    tone="yellow"
                    aria-label={`${section.label} prompt section`}
                    value={section.editableText}
                    rows={5}
                    readOnly={section.id === "selection" && showContinueSelectionNote}
                    disabled={isBusy || chapterSync.blocked}
                    onChange={(event) => setSectionOverrides((current) => ({
                      ...current, [inspectorAction]: {
                        ...current[inspectorAction], [section.id]: event.target.value,
                      },
                    }))}
                  />}
                  {section.id !== "system" && section.id !== "author_guidance" && section.id !== "lore" && sectionOverrides[inspectorAction][section.id] !== undefined ? (
                    <Button size="sm"
                      disabled={isBusy}
                      onClick={() => setSectionOverrides((current) => {
                        const next = { ...current[inspectorAction] };
                        if (section.id !== "system" && section.id !== "author_guidance" && section.id !== "lore") delete next[section.id];
                        return { ...current, [inspectorAction]: next };
                      })}>Reset {section.label}</Button>
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
