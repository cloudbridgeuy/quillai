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

const dialogueOf = (text: string, language: "en" | "es") => slices(text, analyzeParagraph(text, language).dialogue);

test("es dash paragraph: leading dash opens dialogue, tag dashes are narration", () => {
  expect(dialogueOf("—Hola —dijo ella—. ¿Qué tal?", "es")).toEqual(["—Hola ", ". ¿Qué tal?"]);
});

test("es dash paragraph: word zones follow the dash spans", () => {
  const words = analyzeParagraph("—Hola —dijo ella—. ¿Qué tal?", "es").words;
  expect(words.map((w) => `${w.text}:${w.zone}`)).toEqual([
    "Hola:dialogue",
    "dijo:narration",
    "ella:narration",
    "Qué:dialogue",
    "tal:dialogue",
  ]);
});

test("es dash paragraph: an unclosed tag runs to the paragraph end", () => {
  expect(dialogueOf("—Hola —dijo ella.", "es")).toEqual(["—Hola "]);
});

test("es dash paragraph: leading whitespace is allowed and later dashes toggle", () => {
  expect(dialogueOf("  —Uno —dijo— dos —dijo— tres", "es")).toEqual(["—Uno ", " dos ", " tres"]);
});

test("es dash paragraph: the quote scan is skipped", () => {
  expect(dialogueOf("—Dijo “no y calló", "es")).toEqual(["—Dijo “no y calló"]);
});

test("es: a dash not at paragraph start is not dash dialogue", () => {
  expect(dialogueOf("Ella dijo —no— y se fue", "es")).toEqual([]);
});

test("en: a leading dash is not dialogue", () => {
  expect(dialogueOf("—Hola —dijo ella—. ¿Qué tal?", "en")).toEqual([]);
});

test("es: only U+2014 starts a dash paragraph", () => {
  expect(dialogueOf("-Hola -dijo ella-.", "es")).toEqual([]);
  expect(dialogueOf("–Hola –dijo ella–.", "es")).toEqual([]);
});

test("es: guillemets are quote marks and ignore inner quote marks", () => {
  expect(dialogueOf("Ella «dijo “no” y calló» fin", "es")).toEqual(["«dijo “no” y calló»"]);
  expect(dialogueOf("Ella «dijo \"no\" y calló» fin", "es")).toEqual(["«dijo \"no\" y calló»"]);
});

test("es: an unclosed guillemet runs to the paragraph end and state does not cross paragraphs", () => {
  expect(dialogueOf("Ella «dijo no", "es")).toEqual(["«dijo no"]);
  expect(dialogueOf("fin»", "es")).toEqual([]);
});

test("en: guillemets are not quote marks and quote zones are unchanged", () => {
  expect(dialogueOf("Ella «dijo “no” y calló» fin", "en")).toEqual(["“no”"]);
});

test("es: straight and curly quotes still work outside guillemets", () => {
  expect(dialogueOf("Ella dijo “no” y \"sí\" fin", "es")).toEqual(["“no”", '"sí"']);
});
