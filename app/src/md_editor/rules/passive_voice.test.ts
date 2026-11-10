import { expect, test } from "bun:test";
import { formatMessage } from "../../i18n/messages";
import { createEngine, RULES, type Scope } from "./engine";

const ID = "passive-voice";
const hints = (text: string, stored: { scope?: Scope; maxAdverbs?: number } = {}, language: "en" | "es" = "en") =>
  createEngine()
    .run(text, language, { [ID]: { scope: stored.scope, params: stored.maxAdverbs === undefined ? undefined : { maxAdverbs: stored.maxAdverbs } } })
    .filter((hint) => hint.ruleId === ID);
const analyzeProse = (text: string) =>
  hints(text).map((hint) => ({ kind: "passive", index: hint.start, length: hint.length, message: formatMessage("en", hint.messageKey) }));

test("flags was written with exact UTF-16 offsets", () => {
  const text = "The letter was written by Ana.";
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 11, length: 11, message: "Passive voice" },
  ]);
  expect(text.slice(11, 22)).toBe("was written");
});

test("emoji before a match keeps UTF-16 offsets", () => {
  const text = "😀 The letter was written by Ana.";
  expect("😀".length).toBe(2);
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 14, length: 11, message: "Passive voice" },
  ]);
  expect(text.slice(14, 25)).toBe("was written");
});

test("reports every passive span in order", () => {
  const text = "The letter was written and the book was read.";
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 11, length: 11, message: "Passive voice" },
    { kind: "passive", index: 36, length: 8, message: "Passive voice" },
  ]);
  expect(text.slice(11, 22)).toBe("was written");
  expect(text.slice(36, 44)).toBe("was read");
});

test("matching is case-insensitive and keeps the original span", () => {
  const text = "THE LETTER WAS WRITTEN BY ANA.";
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 11, length: 11, message: "Passive voice" },
  ]);
  expect(text.slice(11, 22)).toBe("WAS WRITTEN");
});

test("active voice is not passive", () => {
  expect(analyzeProse("Ana wrote the letter.")).toEqual([]);
  expect(analyzeProse("Ana has written the letter.")).toEqual([]);
  expect(analyzeProse("She edited the chapter.")).toEqual([]);
  expect(analyzeProse("He's written a letter.")).toEqual([]);
  expect(analyzeProse("The letter got written.")).toEqual([]);
});

test("common adjectival participles are not passive", () => {
  expect(analyzeProse("I was tired.")).toEqual([]);
  expect(analyzeProse("She was excited.")).toEqual([]);
  expect(analyzeProse("They were interested.")).toEqual([]);
  expect(analyzeProse("He is bored.")).toEqual([]);
});

test("a be-verb with no participle is not passive", () => {
  expect(analyzeProse("She was here.")).toEqual([]);
  expect(analyzeProse("He was a writer.")).toEqual([]);
  expect(analyzeProse("They were seven.")).toEqual([]);
});

test("words that merely end in -ed are not participles", () => {
  expect(analyzeProse("I need the bed.")).toEqual([]);
  expect(analyzeProse("The bed was soft.")).toEqual([]);
  expect(analyzeProse("It was bed and it was need.")).toEqual([]);
  expect(analyzeProse("She made the bed before they need lunch.")).toEqual([]);
  expect(analyzeProse("She was naked.")).toEqual([]);
  expect(analyzeProse("The grove was sacred.")).toEqual([]);
});

test("a nearby bed does not hide a real passive", () => {
  const text = "The bed was made.";
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 8, length: 8, message: "Passive voice" },
  ]);
  expect(text.slice(8, 16)).toBe("was made");
});

test("needed is still a participle", () => {
  const text = "The form was needed.";
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 9, length: 10, message: "Passive voice" },
  ]);
  expect(text.slice(9, 19)).toBe("was needed");
});

test("adverbs may sit between the be-verb and the participle", () => {
  const text = "The letter was carefully written by Ana.";
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 11, length: 21, message: "Passive voice" },
  ]);
  expect(text.slice(11, 32)).toBe("was carefully written");
});

test("commas do not split the verb from its participle", () => {
  const text = "The letter was, however, written by Ana.";
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 11, length: 21, message: "Passive voice" },
  ]);
  expect(text.slice(11, 32)).toBe("was, however, written");
});

test("sentence punctuation blocks a false passive", () => {
  expect(analyzeProse("She was. Written by Ana.")).toEqual([]);
  expect(analyzeProse("Was she late? Written later, the note arrived.")).toEqual([]);
});

test("parentheses and a trailing comma still flag the span", () => {
  const wrapped = "(The letter was written.)";
  expect(analyzeProse(wrapped)).toEqual([
    { kind: "passive", index: 12, length: 11, message: "Passive voice" },
  ]);
  expect(wrapped.slice(12, 23)).toBe("was written");

  const comma = "The letter was written, then filed.";
  expect(analyzeProse(comma)).toEqual([
    { kind: "passive", index: 11, length: 11, message: "Passive voice" },
  ]);
  expect(comma.slice(11, 22)).toBe("was written");
});

test("simple contractions expose the be-verb", () => {
  const been = "It's been written by Ana.";
  expect(analyzeProse(been)).toEqual([
    { kind: "passive", index: 5, length: 12, message: "Passive voice" },
  ]);
  expect(been.slice(5, 17)).toBe("been written");

  const its = "It's written by Ana.";
  expect(analyzeProse(its)).toEqual([
    { kind: "passive", index: 0, length: 12, message: "Passive voice" },
  ]);
  expect(its.slice(0, 12)).toBe("It's written");

  const negated = "It's not written.";
  expect(analyzeProse(negated)).toEqual([
    { kind: "passive", index: 0, length: 16, message: "Passive voice" },
  ]);
  expect(negated.slice(0, 16)).toBe("It's not written");
});

test("negative and subject contractions count", () => {
  const wasnt = "The letter wasn't written.";
  expect(analyzeProse(wasnt)).toEqual([
    { kind: "passive", index: 11, length: 14, message: "Passive voice" },
  ]);
  expect(wasnt.slice(11, 25)).toBe("wasn't written");

  const told = "I'm told a secret.";
  expect(analyzeProse(told)).toEqual([
    { kind: "passive", index: 0, length: 8, message: "Passive voice" },
  ]);
  expect(told.slice(0, 8)).toBe("I'm told");

  const they = "They're written by Ana.";
  expect(analyzeProse(they)).toEqual([
    { kind: "passive", index: 0, length: 15, message: "Passive voice" },
  ]);
  expect(they.slice(0, 15)).toBe("They're written");
});

test("a curly apostrophe does not shift the UTF-16 span", () => {
  const text = "It’s written.";
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 0, length: 12, message: "Passive voice" },
  ]);
  expect(text.slice(0, 12)).toBe("It’s written");
});

test("passive progressive starts at being", () => {
  const text = "The book was being written.";
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 13, length: 13, message: "Passive voice" },
  ]);
  expect(text.slice(13, 26)).toBe("being written");
});

test("an adjectival clause does not swallow a later passive", () => {
  const text = "They were interested in the letter that was written by Ana.";
  expect(analyzeProse(text)).toEqual([
    { kind: "passive", index: 40, length: 11, message: "Passive voice" },
  ]);
  expect(text.slice(40, 51)).toBe("was written");
});

test("empty text has no hints", () => {
  expect(analyzeProse("")).toEqual([]);
});

test("a plain passive is flagged from the be-verb through the participle, with no message args", () => {
  const text = "The door was opened by her";
  expect(hints(text)).toEqual([{ ruleId: ID, start: 9, length: 10, messageKey: "hints.passive" }]);
  expect(text.slice(9, 19)).toBe("was opened");
});

test("up to maxAdverbs adverbs may sit inside, one more is not caught", () => {
  expect(hints("The letter was quickly, quietly, slowly written.")).toHaveLength(1);
  expect(hints("The letter was quickly, quietly, slowly, gently written.")).toEqual([]);
  expect(hints("The letter was quickly, quietly, slowly, gently written.", { maxAdverbs: 4 })).toHaveLength(1);
  expect(hints("The letter was carefully written.", { maxAdverbs: 0 })).toEqual([]);
  expect(hints("The letter was written.", { maxAdverbs: 0 })).toHaveLength(1);
});

test("a stative be + adjective is not flagged", () => {
  expect(hints("She was tired.")).toEqual([]);
});

test("hints are shifted to the document offset of their paragraph", () => {
  expect(hints("Hi.\nThe door was opened.").map((h) => h.start)).toEqual([13]);
});

test("default scope is narration; dialogue passives need dialogue or both", () => {
  const dialogue = '"The door was opened," she said.';
  const narration = "The door was opened.";
  expect(hints(dialogue)).toEqual([]);
  expect(hints(dialogue, { scope: "narration" })).toEqual([]);
  expect(hints(dialogue, { scope: "dialogue" })).toHaveLength(1);
  expect(hints(dialogue, { scope: "both" })).toHaveLength(1);
  expect(hints(narration)).toHaveLength(1);
  expect(hints(narration, { scope: "dialogue" })).toEqual([]);
});

test("the rule is English only", () => {
  expect(hints("The door was opened.", {}, "es")).toEqual([]);
});

test("the rule is registered with the spec defaults and the agreed bounds", () => {
  expect(RULES.find((rule) => rule.id === ID)).toMatchObject({
    labelKey: "rules.passive.label",
    languages: ["en"],
    defaultEnabled: true,
    defaultScope: "narration",
    params: [{ key: "maxAdverbs", kind: "int", labelKey: "rules.passive.maxAdverbs", default: 3, min: 0, max: 6 }],
  });
});

test("the message renders in both locales with no placeholder left", () => {
  const [hint] = hints("The door was opened.");
  expect(formatMessage("en", hint!.messageKey, hint!.messageArgs)).toBe("Passive voice");
  expect(formatMessage("es", hint!.messageKey, hint!.messageArgs)).toBe("Voz pasiva");
});
