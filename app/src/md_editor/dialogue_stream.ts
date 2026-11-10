import Delta from "quill-delta";

import type { AiDocumentSlice } from "./ai_contract";
import type { ContinueStream } from "./continue_stream";
import { buildDialogueRequest, type DialogueRequest } from "./dialogue_request";
import type { LoreMatchEntry } from "./lore_match";
import { blockAttribute, parseBlock, SCRIPT_BLOCK, type ScriptLine } from "./script_blocks";
import { feedScriptExchange, finishScriptExchange, SCRIPT_EXCHANGE_START, type ScriptExchangeState } from "./script_exchange";
import type { DocumentWindow } from "./window";

export type DialogueChoice = { speakers: string[]; brief: string };
export type DialogueFeed = { script: ScriptExchangeState | null; prefix: Delta; line: string | null };

const lineDelta = (type: ScriptLine["type"]) => {
  const block = blockAttribute(type);
  return block ? { [SCRIPT_BLOCK]: block } : undefined;
};

export function dialogueCaret(selection: { index: number; length: number } | null, documentLength: number): number {
  return selection ? selection.index + selection.length : documentLength;
}

export function createDialogueStream(text: string, caret: number, script: Record<string, unknown> | null): { stream: ContinueStream; feed: DialogueFeed } {
  const lineStart = text.lastIndexOf("\n", caret - 1) + 1;
  const lineEnd = text.indexOf("\n", caret) < 0 ? text.length : text.indexOf("\n", caret);
  const before = text.slice(lineStart, caret).trim() !== "";
  const after = text.slice(caret, lineEnd).trim() !== "";
  const prefix = before ? new Delta().insert("\n", script ? lineDelta(parseBlock(script[SCRIPT_BLOCK])) : undefined) : new Delta();
  return {
    stream: { anchor: caret, end: caret, started: false, paragraph: false, removal: new Delta() },
    feed: { script: script ? SCRIPT_EXCHANGE_START : null, prefix, line: after && !script ? "" : null },
  };
}

export function feedDialogue(feed: DialogueFeed, text: string): { feed: DialogueFeed; fragment: Delta } {
  if (feed.script) {
    const { lines, state } = feedScriptExchange(feed.script, text);
    return withLines({ ...feed, script: state }, lines.map(line => new Delta().insert(line.text).insert("\n", lineDelta(line.type))));
  }
  if (feed.line === null) return withLines(feed, text ? [new Delta().insert(text)] : []);
  const parts = (feed.line + text).split("\n");
  const line = parts.pop() ?? "";
  return withLines({ ...feed, line }, parts.map(part => new Delta().insert(`${part}\n`)));
}

export function finishDialogue(feed: DialogueFeed): Delta {
  if (feed.script) return withLines(feed, finishScriptExchange(feed.script).map(line => new Delta().insert(line.text).insert("\n", lineDelta(line.type)))).fragment;
  return withLines(feed, feed.line ? [new Delta().insert(`${feed.line}\n`)] : []).fragment;
}

function withLines(feed: DialogueFeed, parts: Delta[]): { feed: DialogueFeed; fragment: Delta } {
  if (parts.length === 0) return { feed, fragment: new Delta() };
  const fragment = parts.reduce((delta, part) => delta.concat(part), new Delta(feed.prefix.ops));
  return { feed: { ...feed, prefix: new Delta() }, fragment };
}

export function dialogueRunRequest(choice: DialogueChoice, preview: { request: { document: AiDocumentSlice }; window: DocumentWindow }, caret: number, lore: readonly LoreMatchEntry[], storyContext: string | null): DialogueRequest {
  return buildDialogueRequest({ ...choice, loreEntries: lore, document: preview.request.document, window: preview.window, caret, storyContext });
}
