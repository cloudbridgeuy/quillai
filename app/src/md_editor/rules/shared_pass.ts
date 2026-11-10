export type Span = { start: number; end: number };
export type Language = "en" | "es";
export type Zone = "narration" | "dialogue";
export type Word = Span & { text: string; lower: string; zone: Zone };
export type ParagraphAnalysis = { text: string; sentences: Span[]; words: Word[]; dialogue: Span[] };
export type Paragraph = { start: number; text: string };

const SENTENCE = /\S.*?(?:[.!?…]+["”’)\]]*(?=\s|$)|(?=\s*$))/gsu;
const DASH_PARAGRAPH = /^\s*—/u;
const WORD = /[\p{L}\p{M}\p{N}]+(?:['’][\p{L}\p{M}\p{N}]+)*/gu;

export function splitParagraphs(text: string): Paragraph[] {
  let start = 0;
  return text.split("\n").map((line) => {
    const paragraph = { start, text: line };
    start += line.length + 1;
    return paragraph;
  });
}

export function analyzeParagraph(text: string, language: Language = "en"): ParagraphAnalysis {
  const dialogue = language === "es" && DASH_PARAGRAPH.test(text) ? dashSpans(text) : quoteSpans(text, language);
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

function dashSpans(text: string): Span[] {
  const spans: Span[] = [];
  let open = text.indexOf("—");
  let inTag = false;
  for (let i = open + 1; i < text.length; i++) {
    if (text[i] !== "—") continue;
    if (inTag) open = i + 1;
    else spans.push({ start: open, end: i });
    inTag = !inTag;
  }
  if (!inTag) spans.push({ start: open, end: text.length });
  return spans.filter((span) => span.end > span.start);
}

function quoteSpans(text: string, language: Language): Span[] {
  const guillemets = language === "es";
  const spans: Span[] = [];
  let open = -1;
  let inGuillemets = false;
  for (let i = 0; i < text.length; i++) {
    const mark = text[i];
    if (inGuillemets) {
      if (mark === "»") {
        spans.push({ start: open, end: i + 1 });
        open = -1;
        inGuillemets = false;
      }
    } else if (open < 0 && guillemets && mark === "«") {
      open = i;
      inGuillemets = true;
    } else if (open < 0 && (mark === '"' || mark === "“")) open = i;
    else if (open >= 0 && (mark === '"' || mark === "”")) {
      spans.push({ start: open, end: i + 1 });
      open = -1;
    }
  }
  if (open >= 0) spans.push({ start: open, end: text.length });
  return spans;
}
