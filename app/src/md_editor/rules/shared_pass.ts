export type Span = { start: number; end: number };
export type Zone = "narration" | "dialogue";
export type Word = Span & { text: string; lower: string; zone: Zone };
export type ParagraphAnalysis = { text: string; sentences: Span[]; words: Word[]; dialogue: Span[] };
export type Paragraph = { start: number; text: string };

const SENTENCE = /\S.*?(?:[.!?…]+["”’)\]]*(?=\s|$)|(?=\s*$))/gsu;
const WORD = /[\p{L}\p{M}\p{N}]+(?:['’][\p{L}\p{M}\p{N}]+)*/gu;

export function splitParagraphs(text: string): Paragraph[] {
  let start = 0;
  return text.split("\n").map((line) => {
    const paragraph = { start, text: line };
    start += line.length + 1;
    return paragraph;
  });
}

export function analyzeParagraph(text: string): ParagraphAnalysis {
  const dialogue = dialogueSpans(text);
  const inDialogue = (offset: number) => dialogue.some((span) => offset >= span.start && offset < span.end);
  const words = Array.from(text.matchAll(WORD), (match): Word => {
    const start = match.index;
    return {
      start,
      end: start + match[0].length,
      text: match[0],
      lower: match[0].toLowerCase(),
      zone: inDialogue(start) ? "dialogue" : "narration",
    };
  });
  const sentences = Array.from(text.matchAll(SENTENCE), (match) => ({ start: match.index, end: match.index + match[0].length }));
  return { text, sentences, words, dialogue };
}

function dialogueSpans(text: string): Span[] {
  const spans: Span[] = [];
  let open = -1;
  for (let i = 0; i < text.length; i++) {
    const mark = text[i];
    if (open < 0 && (mark === '"' || mark === "“")) open = i;
    else if (open >= 0 && (mark === '"' || mark === "”")) {
      spans.push({ start: open, end: i + 1 });
      open = -1;
    }
  }
  if (open >= 0) spans.push({ start: open, end: text.length });
  return spans;
}
