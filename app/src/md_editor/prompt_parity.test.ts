import { expect, test } from "bun:test";
import fixture from "../../../fixtures/prompt/parity.json";
import type { AiGenerateRequest } from "./ai_contract";
import { assemblePromptSections } from "./prompt_sections";

for (const example of fixture.cases) {
  test(`prompt parity: ${example.name}`, () => {
    const input = structuredClone(example.input);
    const sections = assemblePromptSections(input as AiGenerateRequest);
    expect(example.sections.map(({ id, text }) => ({ id, text }))).toEqual(
      sections.map(({ id, text }) => ({ id, text })),
    );
    expect(sections.map(({ id }) => id)).toEqual([
      "system", "history", "instruction", "story_context", "selection", "document",
    ]);
    for (const [index, section] of sections.entries()) {
      if (index >= 2) {
        expect([...section.text.matchAll(/<([a-z_]+)>/g)].map((match) => match[1]))
          .toEqual(example.sections[index]!.tags);
      }
      expect(section.tokens).toBe(Math.ceil(new TextEncoder().encode(section.text).length / 4));
    }
    expect(sections[1]!.text).toBe(example.history.map(({ content }) => content).join(""));
    expect(input).toEqual(example.input);
  });
}
