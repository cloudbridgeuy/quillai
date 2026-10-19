import { describe, expect, test } from "bun:test";
import type { AiGenerateRequest } from "./ai_contract";
import {
  assemblePromptSections,
  estimateTokens,
  TOKEN_ESTIMATE_LABEL,
  totalEstimatedTokens,
} from "./prompt_sections";

function request(overrides: Partial<AiGenerateRequest> = {}): AiGenerateRequest {
  return {
    action: "generate",
    document: { delta: { ops: [{ insert: "Hello" }] }, text: "Hello" },
    selection: null,
    storyContext: "fantasy",
    instruction: "continue",
    messages: [],
    resumePrompt: null,
    maxOutputTokens: 512,
    ...overrides,
  };
}

describe("assemblePromptSections", () => {
  test("generate golden matches provider content order and the complete server prompt", () => {
    const sections = assemblePromptSections(request({
      messages: [
        { role: "user", content: "  previous <prompt>  " },
        { role: "assistant", content: "previous & output" },
      ],
      resumePrompt: " resume from cutoff ",
    }));

    expect(sections.map(({ id, label, text }) => ({ id, label, text }))).toEqual([
      { id: "system", label: "System", text: "You are QuillAI, a concise fiction-writing assistant. Reply with plain text only." },
      { id: "history", label: "History", text: "previous <prompt>previous & output" },
      { id: "instruction", label: "Instruction", text: "You are helping with a fiction-writing task. Respond with plain text only, and do not exceed two sentences. <action>generate</action>\n<instruction>\nresume from cutoff\n</instruction>\n" },
      { id: "story_context", label: "Story context", text: "<story_context>\nfantasy\n</story_context>\n" },
      { id: "selection", label: "Selection", text: "<selection>\nnone\n</selection>\n" },
      { id: "document", label: "Document", text: "<document>\nHello\n</document>" },
    ]);
    expect(sections.slice(2).map(({ text }) => text).join("")).toBe(
      "You are helping with a fiction-writing task. Respond with plain text only, and do not exceed two sentences. <action>generate</action>\n<instruction>\nresume from cutoff\n</instruction>\n<story_context>\nfantasy\n</story_context>\n<selection>\nnone\n</selection>\n<document>\nHello\n</document>",
    );
  });

  test("edit golden escapes current-turn fields, keeps UTF-16 ranges, and drops generate-only fields", () => {
    const text = "Hi 💡 世界 & <end>\n";
    const sections = assemblePromptSections(request({
      action: "edit",
      document: { delta: { ops: [{ insert: text }] }, text },
      selection: {
        delta: { ops: [{ insert: "💡 世界" }] },
        text: "💡 世界",
        range: { index: 3, length: 5 },
      },
      instruction: " tighten & <polish> ",
      storyContext: " notes </story_context> &amp; ",
      messages: [{ role: "assistant", content: "must not appear" }],
      resumePrompt: "must not appear either",
    }));

    expect(sections.map(({ text }) => text)).toEqual([
      "You are QuillAI, a concise fiction-writing assistant. Reply with plain text only.",
      "",
      "You are helping with a fiction-writing task. Respond with plain text only, and do not exceed two sentences. <action>edit</action>\n<instruction>\ntighten &amp; &lt;polish&gt;\n</instruction>\n",
      "<story_context>\nnotes &lt;/story_context&gt; &amp;amp;\n</story_context>\n",
      "<selection>\nindex=3, length=5, text=💡 世界\n</selection>\n",
      "<document>\nHi 💡 世界 &amp; &lt;end&gt;\n\n</document>",
    ]);
  });

  test("absent and blank optional text uses none, and blank resume falls back to instruction", () => {
    const sections = assemblePromptSections(request({
      instruction: null,
      storyContext: " \n\t ",
    }));
    expect(sections[2]?.text).toContain("<instruction>\nnone\n</instruction>");
    expect(sections[3]?.text).toBe("<story_context>\nnone\n</story_context>\n");
    expect(assemblePromptSections(request({ resumePrompt: " \n " }))[2]?.text)
      .toContain("<instruction>\ncontinue\n</instruction>");
  });

  test("normalizes Unicode whitespace like Rust, not JavaScript trim", () => {
    const sections = assemblePromptSections(request({
      instruction: "\u0085continue\u0085",
      storyContext: "\uFEFFnotes\uFEFF",
      messages: [{ role: "user", content: "\u0085💡 世界\u0085" }],
    }));
    expect(sections[1]?.text).toBe("💡 世界");
    expect(sections[2]?.text).toContain("<instruction>\ncontinue\n</instruction>");
    expect(sections[3]?.text).toContain("\uFEFFnotes\uFEFF");
  });

  test("is deterministic and does not mutate the request", () => {
    const input = request();
    const before = structuredClone(input);
    expect(assemblePromptSections(input)).toEqual(assemblePromptSections(input));
    expect(input).toEqual(before);
  });
});

describe("token estimates", () => {
  test.each([
    ["", 0],
    ["abcd", 1],
    ["abcde", 2],
    ["💡", 1],
    ["世界", 2],
    ["💡 世界", 3],
  ])("estimates UTF-8 bytes for %j", (text, tokens) => {
    expect(estimateTokens(text)).toBe(tokens);
  });

  test("labels estimates and sums rounded section estimates for both actions", () => {
    expect(TOKEN_ESTIMATE_LABEL).toContain("Estimated tokens");
    for (const action of ["generate", "edit"] as const) {
      const sections = assemblePromptSections(request({
        action,
        selection: {
          delta: { ops: [{ insert: "Hello" }] },
          text: "Hello",
          range: { index: 0, length: 5 },
        },
      }));
      for (const section of sections) {
        expect(section.tokens).toBe(estimateTokens(section.text));
      }
      expect(sections.map(({ tokens }) => tokens)).toEqual([
        21, 0, action === "generate" ? 43 : 42, 11, 14, 7,
      ]);
      expect(totalEstimatedTokens(sections)).toBe(action === "generate" ? 96 : 95);
      expect(totalEstimatedTokens(sections)).toBe(sections.reduce((sum, { tokens }) => sum + tokens, 0));
    }
    expect(totalEstimatedTokens([])).toBe(0);
  });
});
