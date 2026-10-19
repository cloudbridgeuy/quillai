import { expect, test } from "bun:test";
import fixture from "../../../fixtures/prompt/parity.json";
import type { AiGenerateRequest } from "./ai_contract";
import { assemblePromptHistory, assemblePromptSections } from "./prompt_sections";

for (const example of fixture.cases) {
  test(`prompt parity: ${example.name}`, () => {
    const input = structuredClone(example.input);
    const guidance = "authorGuidance" in example ? example.authorGuidance : "";
    const sections = assemblePromptSections(input as AiGenerateRequest, guidance);
    expect(example.history).toEqual(assemblePromptHistory(input as AiGenerateRequest));
    expect(example.sections.map(({ id, text }) => ({ id, text }))).toEqual(
      sections.map(({ id, text }) => ({ id, text })),
    );
    expect(sections.map(({ id }) => id)).toEqual([
      "system", ...(guidance ? ["author_guidance" as const] : []), "history", "instruction", "story_context", "selection", "document",
    ]);
    for (const [index, section] of sections.entries()) {
      if (section.id !== "system" && section.id !== "history") {
        expect([...section.text.matchAll(/<([a-z_]+)>/g)].map((match) => match[1]))
          .toEqual(example.sections[index]!.tags);
      }
      expect(section.tokens).toBe(Math.ceil(new TextEncoder().encode(section.text).length / 4));
    }
    expect(sections.find(({ id }) => id === "history")!.text).toBe(example.history.map(({ content }) => content).join(""));
    expect(input).toEqual(example.input);
  });
}
