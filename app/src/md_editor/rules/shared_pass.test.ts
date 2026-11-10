import { expect, test } from "bun:test";
import { analyzeParagraph, splitParagraphs } from "./shared_pass";

const slices = (text: string, spans: { start: number; end: number }[]) => spans.map((s) => text.slice(s.start, s.end));

test("splits chapter text on newlines with document offsets", () => {
  expect(splitParagraphs("One.\nTwo two.\n")).toEqual([
    { start: 0, text: "One." },
    { start: 5, text: "Two two." },
    { start: 14, text: "" },
  ]);
});

test("splits sentences on terminal punctuation, closing quotes included", () => {
  const text = "She left. Did he stay? “No!” he said. Wait...";
  expect(slices(text, analyzeParagraph(text).sentences)).toEqual([
    "She left.",
    "Did he stay?",
    "“No!”",
    "he said.",
    "Wait...",
  ]);
});

test("keeps an unterminated last sentence", () => {
  const text = "First one. and then";
  expect(slices(text, analyzeParagraph(text).sentences)).toEqual(["First one.", "and then"]);
  expect(slices("  and then  ", analyzeParagraph("  and then  ").sentences)).toEqual(["and then"]);
});

test("words carry offsets, lower case and contractions", () => {
  const text = "Ana’s DOG ran";
  expect(analyzeParagraph(text).words).toEqual([
    { start: 0, end: 5, text: "Ana’s", lower: "ana’s", zone: "narration" },
    { start: 6, end: 9, text: "DOG", lower: "dog", zone: "narration" },
    { start: 10, end: 13, text: "ran", lower: "ran", zone: "narration" },
  ]);
});

test("Spanish letters stay inside one word", () => {
  const text = "El niño miró la canción";
  expect(analyzeParagraph(text).words.map((w) => w.text)).toEqual(["El", "niño", "miró", "la", "canción"]);
});

test("emoji before a word keeps UTF-16 offsets", () => {
  const text = "🙂 hi";
  expect(analyzeParagraph(text).words).toEqual([{ start: 3, end: 5, text: "hi", lower: "hi", zone: "narration" }]);
});

test("curly quotes mark dialogue, quotes included", () => {
  const text = "“Go home,” she said. “Now.”";
  const result = analyzeParagraph(text);
  expect(slices(text, result.dialogue)).toEqual(["“Go home,”", "“Now.”"]);
  expect(result.words.map((w) => [w.text, w.zone])).toEqual([
    ["Go", "dialogue"],
    ["home", "dialogue"],
    ["she", "narration"],
    ["said", "narration"],
    ["Now", "dialogue"],
  ]);
});

test("straight quotes toggle dialogue", () => {
  const text = "He said \"stop\" and \"wait\" twice.";
  expect(slices(text, analyzeParagraph(text).dialogue)).toEqual(["\"stop\"", "\"wait\""]);
});

test("Spanish curly-quote dialogue uses the same scan", () => {
  const text = "“¿Vienes?” preguntó ella.";
  expect(slices(text, analyzeParagraph(text).dialogue)).toEqual(["“¿Vienes?”"]);
});

test("an unclosed quote runs to the end of the paragraph", () => {
  const text = "She said, “I will not go";
  expect(slices(text, analyzeParagraph(text).dialogue)).toEqual(["“I will not go"]);
});

test("state never crosses paragraphs", () => {
  const texts = splitParagraphs("“Open speech\nPlain narration.").map((p) => p.text);
  expect(texts.map((t) => analyzeParagraph(t).dialogue)).toEqual([[{ start: 0, end: 12 }], []]);
});

test("apostrophes and single quotes are not dialogue marks", () => {
  const text = "It’s Ana’s ‘book’ and 'pen' now.";
  const result = analyzeParagraph(text);
  expect(result.dialogue).toEqual([]);
  expect(result.words.every((w) => w.zone === "narration")).toBe(true);
});

test("a stray closing quote in narration is ignored", () => {
  const text = "Plain” text “then speech”";
  expect(slices(text, analyzeParagraph(text).dialogue)).toEqual(["“then speech”"]);
});

test("empty paragraph has no sentences, words or dialogue", () => {
  expect(analyzeParagraph("")).toEqual({ text: "", sentences: [], words: [], dialogue: [] });
});
