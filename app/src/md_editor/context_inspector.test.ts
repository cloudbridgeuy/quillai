import { expect, test } from "bun:test";
import type { AiGenerateRequest } from "./ai_contract";
import { buildContextInspector } from "./context_inspector";

function request(overrides: Partial<AiGenerateRequest> = {}): AiGenerateRequest {
  return {
    action: "generate",
    document: { delta: { ops: [] }, text: "" },
    selection: null,
    storyContext: null,
    instruction: null,
    messages: [],
    resumePrompt: null,
    maxOutputTokens: 512,
    ...overrides,
  };
}

test("empty context keeps all sections, empty history, and framed defaults", () => {
  const model = buildContextInspector(request());
  expect(model.sections.map(({ id, label }) => [id, label])).toEqual([
    ["system", "System"],
    ["history", "History"],
    ["instruction", "Instruction"],
    ["story_context", "Story context"],
    ["selection", "Selection"],
    ["document", "Document"],
  ]);
  expect(model.sections[1]?.text).toBe("");
  expect(model.sections[2]?.text).toContain("<instruction>\nnone\n</instruction>");
  expect(model.sections[3]?.text).toBe("<story_context>\nnone\n</story_context>\n");
  expect(model.sections[4]?.text).toBe("<selection>\nnone\n</selection>\n");
  expect(model.sections[5]?.text).toBe("<document>\n\n</document>");
  expect(model.sections.map(({ tokens }) => tokens)).toEqual([21, 0, 42, 10, 8, 6]);
  expect(model.totalTokens).toBe(87);
});

test("all sections use resume context and sum individually rounded estimates", () => {
  const input = request({
    document: { delta: { ops: [{ insert: "Hello" }] }, text: "Hello" },
    selection: {
      delta: { ops: [{ insert: "Hello" }] },
      text: "Hello",
      range: { index: 0, length: 5 },
    },
    storyContext: "fantasy",
    instruction: "continue",
    messages: [{ role: "user", content: "a" }, { role: "assistant", content: "b" }],
    resumePrompt: "resume",
  });
  const before = structuredClone(input);
  const model = buildContextInspector(input);
  expect(model.sections.map(({ text }) => text)).toEqual([
    "You are QuillAI, a concise fiction-writing assistant. Reply with plain text only.",
    "ab",
    "You are helping with a fiction-writing task. Respond with plain text only, and do not exceed two sentences. <action>generate</action>\n<instruction>\nresume\n</instruction>\n",
    "<story_context>\nfantasy\n</story_context>\n",
    "<selection>\nindex=0, length=5, text=Hello\n</selection>\n",
    "<document>\nHello\n</document>",
  ]);
  expect(model.sections.map(({ tokens }) => tokens)).toEqual([21, 1, 43, 11, 14, 7]);
  expect(model.totalTokens).toBe(97);
  expect(model.tokenEstimateLabel).toBe("Estimated tokens (UTF-8 bytes ÷ 4, rounded up per section; excludes chat framing)");
  expect(input).toEqual(before);
  expect(buildContextInspector(input)).toEqual(model);
});

test("multibyte text and emoji estimates count UTF-8 bytes, not UTF-16 units", () => {
  const model = buildContextInspector(request({
    document: { delta: { ops: [{ insert: "💡 世界" }] }, text: "💡 世界" },
    messages: [{ role: "assistant", content: "💡 世界" }],
  }));
  expect(model.sections[1]).toEqual({ id: "history", label: "History", text: "💡 世界", editableText: "💡 世界", tokens: 3 });
  expect(model.sections[5]).toEqual({ id: "document", label: "Document", text: "<document>\n💡 世界\n</document>", editableText: "💡 世界", tokens: 9 });
  expect(model.totalTokens).toBe(93);
});

test("Edit inspector exposes raw editable content and ordered history turns", () => {
  const history = [{ role: "assistant" as const, text: "Earlier answer" }, { role: "user" as const, text: "Now revise" }];
  const model = buildContextInspector(request({
    action: "edit",
    document: { delta: { ops: [{ insert: "Hello" }] }, text: "Hello" },
    selection: { delta: { ops: [{ insert: "Hello" }] }, text: "Hello", range: { index: 0, length: 5 } },
    sectionOverrides: { selection: "Edited </selection>", document: "New </document>", history },
  }));
  expect(model.history).toEqual(history);
  expect(model.sections[4]!.editableText).toBe("Edited </selection>");
  expect(model.sections[4]!.text).toBe("<selection>\nindex=0, length=5, text=Edited &lt;/selection&gt;\n</selection>\n");
  expect(model.sections[5]!.editableText).toBe("New </document>");
  expect(model.sections[5]!.text).toBe("<document>\nNew &lt;/document&gt;\n</document>");
});
