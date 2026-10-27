import type Delta from "quill-delta";
import { scriptLines, type BlockType, type ScriptLine } from "./script_blocks";

const PARAGRAPH_TYPE: Record<BlockType, string> = {
  "scene-heading": "Scene Heading",
  action: "Action",
  character: "Character",
  dialogue: "Dialogue",
  parenthetical: "Parenthetical",
  transition: "Transition",
};

const ENTITY: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };

function xmlText(text: string): string {
  return text.replace(/[\x00-\x08\x0B\x0C\x0E-\x1F￾￿]/g, "").replace(/[&<>"]/g, (character) => ENTITY[character]!);
}

function lineText({ type, text }: ScriptLine): string {
  return type === "parenthetical" && !(text.startsWith("(") && text.endsWith(")")) ? `(${text})` : text;
}

export function toFdx(scenes: readonly Delta[]): string {
  const paragraphs = scenes
    .flatMap(scriptLines)
    .filter((line) => line.text !== "")
    .map((line) => `<Paragraph Type="${PARAGRAPH_TYPE[line.type]}"><Text>${xmlText(lineText(line))}</Text></Paragraph>\n`);
  return `<?xml version="1.0" encoding="UTF-8" standalone="no" ?>\n<FinalDraft DocumentType="Script" Template="No" Version="5">\n<Content>\n${paragraphs.join("")}</Content>\n</FinalDraft>\n`;
}
