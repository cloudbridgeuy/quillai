import Quill from "quill";
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
  buildAiRequest,
  type AiAction,
} from "../ai_contract";
import {
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
import type {
  DeltaT,
  OnTextChangeFn,
  OnSelectionChangeFn,
  RangeT,
} from "../types";
import { calculateDocumentWindow, type DocumentWindowMode } from "../window";

function processMarkdownSyntax(quill: Quill, delta: DeltaT, source: string) {
  if (source !== "user") return;

  const text = quill.getText();
  const selection = quill.getSelection();

  if (!selection) return;

  const lastOp = delta.ops?.[delta.ops.length - 1];
  if (!lastOp || !lastOp.insert || typeof lastOp.insert !== "string") return;

  const lastChar = lastOp.insert.charAt(lastOp.insert.length - 1);
  if (lastChar !== "*") return;

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

const INITIAL_DOCUMENT = new Delta()
  .insert("Chapter One\n", { header: 1 })
  .insert(
    "The rain had started before dusk, and Mara still had not decided whether the letter in her coat pocket was a warning or an invitation.\n\n",
  );

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

export const Container = () => {
  const [range, setRange] = useState<RangeT | null>(null);
  const [storyContext, setStoryContext] = useState("");
  const [instruction, setInstruction] = useState("");
  const [windowSize, setWindowSize] = useState("1200");
  const [windowMode, setWindowMode] = useState<DocumentWindowMode>("window");
  const [aiRunState, setAiRunState] = useState<AiRunState>(idleAiRunState);

  const quillRef = useRef<Quill | null>(null);
  const aiRunStateRef = useRef(aiRunState);
  const abortControllerRef = useRef<AbortController | null>(null);
  const isMountedRef = useRef(true);
  const lastCursorRangeRef = useRef<{ index: number; length: number } | null>(null);
  const lastEditSelectionRef = useRef<{ index: number; length: number } | null>(null);

  useEffect(() => {
    aiRunStateRef.current = aiRunState;
  }, [aiRunState]);

  useEffect(() => {
    isMountedRef.current = true;

    return () => {
      isMountedRef.current = false;
      abortControllerRef.current?.abort();
    };
  }, []);

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

  function clearFailedRunState() {
    if (!shouldClearFailedRunState(aiRunStateRef.current, "user")) {
      return;
    }

    const reset = resetRunState(aiRunStateRef.current);
    if (reset.ok) {
      updateAiRunState(reset.state);
    }
  }

  async function runAiAction(action: AiAction) {
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

    const documentLength = Math.max(quill.getLength() - 1, 0);
    const normalizedSelection = selection
      ? { index: selection.index, length: selection.length }
      : null;
    const anchor =
      action === "edit"
        ? normalizedSelection?.index ?? 0
        : normalizedSelection?.index ?? documentLength;
    const deleteLength = action === "edit" ? normalizedSelection?.length ?? 0 : 0;
    const originalSelectionDelta =
      deleteLength > 0 ? new Delta(quill.getContents(anchor, deleteLength).ops) : null;
    let didDeleteSelection = false;
    const controller = new AbortController();

    abortControllerRef.current = controller;
    updateAiRunState(started.state);
    quill.enable(false);
    lastEditSelectionRef.current = null;

    const isActiveEditor = () => isMountedRef.current && quillRef.current === quill;
    let nextInsertIndex = anchor;

    try {
      const assertEditorActive = () => {
        if (!isActiveEditor()) {
          throw new DOMException("Editor is no longer active.", "AbortError");
        }
      };

      const configuredWindowSize = Number(windowSize);
      const window = calculateDocumentWindow({
        documentLength,
        selection: normalizedSelection,
        windowSize: configuredWindowSize,
        mode: windowMode,
      });

      const documentSlice = {
        delta: toContractDeltaDocument(
          quill.getContents(window.start, window.end - window.start),
        ),
        text: quill.getText(window.start, window.end - window.start),
      };

      const selectionSlice = normalizedSelection
        ? {
            delta: toContractDeltaDocument(
              quill.getContents(
                normalizedSelection.index,
                normalizedSelection.length,
              ),
            ),
            text: quill.getText(
              normalizedSelection.index,
              normalizedSelection.length,
            ),
            range: normalizedSelection,
          }
        : null;

      const request =
        action === "edit"
          ? buildAiRequest({
              action: "edit",
              document: documentSlice,
              window,
              selection: selectionSlice!,
              storyContext,
              instruction,
            })
          : buildAiRequest({
              action: "generate",
              document: documentSlice,
              window,
              selection: selectionSlice,
              storyContext,
              instruction,
            });

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

      const applyEvents = (events: ReturnType<typeof parseAiStreamChunk>["events"]) => {
        for (const event of events) {
          if (event.type === "delta") {
            if (!isTextOnlyDeltaDocument(event.delta)) {
              throw new Error("AI stream delta must contain text inserts only.");
            }

            assertEditorActive();
            const fragment = new Delta(toQuillDeltaDocument(event.delta));
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

      setCursorState(quill, nextInsertIndex);
    } catch (error) {
      const wasAborted =
        (error instanceof DOMException && error.name === "AbortError") ||
        (error instanceof Error && error.name === "AbortError");

      if (wasAborted) {
        const finished = finishRun(aiRunStateRef.current, "cancelled");
        if (finished.ok) {
          updateAiRunState(finished.state);
        } else {
          setAiFailure(action, "Cancellation did not complete cleanly.");
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
      abortControllerRef.current = null;
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

  const statusPresentation = getStatusPresentation(aiRunState);

  return (
    <div className="editor-shell">
      <div className="editor-topbar">
        <div>
          <h1>QuillAI</h1>
          <p>Draft fiction, then steer the stub AI inline.</p>
        </div>
        <div className={`editor-status editor-status-${statusPresentation.tone}`}>
          <span className="editor-status-label">Status</span>
          <span>{statusPresentation.label}</span>
        </div>
      </div>

      <div className="editor-controls-panel">
        <label className="editor-field editor-field-full">
          <span>Story Context</span>
          <textarea
            className="editor-textarea"
            value={storyContext}
            onChange={(event) => setStoryContext(event.target.value)}
            placeholder="Series premise, tone, point of view, or scene notes"
            rows={4}
            disabled={isBusy}
          />
        </label>

        <div className="editor-controls-grid">
          <label className="editor-field editor-field-grow">
            <span>Instruction</span>
            <textarea
              className="editor-textarea editor-textarea-compact"
              value={instruction}
              onChange={(event) => setInstruction(event.target.value)}
              placeholder="Continue the scene, tighten the selected paragraph, sharpen dialogue..."
              rows={2}
              disabled={isBusy}
            />
          </label>

          <label className="editor-field editor-field-number">
            <span>Window Size</span>
            <input
              className="editor-input"
              type="number"
              min="0"
              step="1"
              value={windowSize}
              onChange={(event) => setWindowSize(event.target.value)}
              disabled={isBusy}
            />
          </label>

          <label className="editor-toggle">
            <input
              type="checkbox"
              checked={windowMode === "document"}
              onChange={(event) =>
                setWindowMode(event.target.checked ? "document" : "window")
              }
              disabled={isBusy}
            />
            <span>Whole document</span>
          </label>
        </div>

        <div className="editor-actions">
          <button
            className="editor-button editor-button-primary"
            type="button"
            onClick={() => void runAiAction("generate")}
            disabled={isBusy}
          >
            Continue
          </button>
          <button
            className="editor-button"
            type="button"
            onClick={() => void runAiAction("edit")}
            disabled={isBusy || !hasSelection}
          >
            Edit
          </button>
          {isBusy ? (
            <button
              className="editor-button editor-button-danger"
              type="button"
              onClick={cancelRun}
            >
              Cancel
            </button>
          ) : null}
        </div>

        <p className="editor-note">
          Edit replaces the current selection as text arrives. Cancel keeps text
          that already landed, and failed edits roll back to the original
          passage.
        </p>
      </div>

      <div className="editor-canvas">
        <Component
          ref={quillRef}
          readOnly={isBusy}
          defaultValue={INITIAL_DOCUMENT}
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
          onTextChange={(_delta, _oldDelta, source) => {
            if (!shouldClearFailedRunState(aiRunStateRef.current, source)) {
              return;
            }

            clearFailedRunState();
          }}
        />
      </div>
    </div>
  );
};

Container.displayName = "EditorContainer";

export default Container;
