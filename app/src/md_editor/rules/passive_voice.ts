// Passive-voice hints. Offsets are UTF-16 code units, the same units Quill uses.
// A be-verb, then up to maxAdverbs adverbs, then a participle: "was carefully written".
// The span runs from the be-verb through the participle.

import type { Rule } from "./engine";

const BE_VERBS = new Set([
  "am",
  "is",
  "are",
  "was",
  "were",
  "be",
  "been",
  "being",
  "i'm",
  "you're",
  "we're",
  "they're",
  "it's",
  "that's",
  "what's",
  "here's",
  "there's",
  "isn't",
  "aren't",
  "wasn't",
  "weren't",
]);

// Stative "be + word" readings. Kept small: these are rarely real passives.
const ADJECTIVAL = new Set([
  "tired",
  "excited",
  "interested",
  "bored",
  "worried",
  "scared",
  "surprised",
  "pleased",
  "ashamed",
  "supposed",
  "confused",
  "embarrassed",
  "frustrated",
  "relaxed",
  "disappointed",
  "exhausted",
]);

// Whole words that end in -ed and are not past participles.
const NOT_PARTICIPLES = new Set([
  "bed",
  "red",
  "need",
  "feed",
  "seed",
  "weed",
  "speed",
  "bleed",
  "breed",
  "greed",
  "creed",
  "indeed",
  "proceed",
  "exceed",
  "succeed",
  "naked",
  "sacred",
  "wicked",
  "wretched",
  "ragged",
  "rugged",
  "jagged",
  "crooked",
  "hundred",
  "biped",
  "moped",
  "coed",
]);

// Past participles the -ed rule cannot see, plus -eed forms the noun filter rejects.
// Omitted on purpose: gone, drunk, hurt, fallen, risen. After be, those are usually
// states, and fall/rise do not passivize.
const IRREGULAR = new Set([
  "agreed",
  "awoken",
  "beaten",
  "begun",
  "bent",
  "beset",
  "bet",
  "bid",
  "bidden",
  "bitten",
  "blown",
  "born",
  "borne",
  "bought",
  "bound",
  "broadcast",
  "broken",
  "brought",
  "built",
  "burnt",
  "burst",
  "cast",
  "caught",
  "chosen",
  "clung",
  "cost",
  "crept",
  "cut",
  "dealt",
  "decreed",
  "done",
  "drawn",
  "dreamt",
  "driven",
  "dug",
  "eaten",
  "fed",
  "felt",
  "fought",
  "forecast",
  "found",
  "forbidden",
  "forgiven",
  "forgotten",
  "freed",
  "frozen",
  "given",
  "got",
  "gotten",
  "ground",
  "grown",
  "guaranteed",
  "heard",
  "held",
  "hidden",
  "hit",
  "hung",
  "kept",
  "knelt",
  "knit",
  "known",
  "laid",
  "leant",
  "leapt",
  "learnt",
  "led",
  "left",
  "lent",
  "let",
  "lit",
  "lost",
  "made",
  "meant",
  "met",
  "misunderstood",
  "mistaken",
  "mown",
  "overcome",
  "overtaken",
  "overwritten",
  "paid",
  "proven",
  "put",
  "quit",
  "read",
  "recast",
  "reread",
  "rewritten",
  "rid",
  "ridden",
  "run",
  "rung",
  "said",
  "sat",
  "seen",
  "sent",
  "set",
  "sewn",
  "shaken",
  "shone",
  "shot",
  "shown",
  "shrunk",
  "shut",
  "slain",
  "slept",
  "slid",
  "sold",
  "sought",
  "spoken",
  "spent",
  "spilt",
  "split",
  "spread",
  "sprung",
  "spun",
  "stood",
  "stolen",
  "stuck",
  "stung",
  "struck",
  "strung",
  "sung",
  "sunk",
  "swept",
  "swollen",
  "sworn",
  "swum",
  "swung",
  "taken",
  "taught",
  "thought",
  "thrown",
  "thrust",
  "told",
  "torn",
  "trodden",
  "understood",
  "undone",
  "upset",
  "wed",
  "withdrawn",
  "woken",
  "won",
  "worn",
  "wound",
  "woven",
  "written",
  "wrung",
]);

const ADVERBS = new Set([
  "not",
  "never",
  "already",
  "also",
  "still",
  "just",
  "often",
  "always",
  "sometimes",
  "ever",
  "yet",
  "now",
  "then",
  "well",
  "quite",
  "rather",
  "very",
  "so",
  "too",
  "even",
  "only",
  "soon",
  "once",
  "twice",
  "indeed",
  "instead",
  "perhaps",
  "maybe",
  "almost",
  "however",
]);

const NOT_ADVERBS = new Set([
  "family",
  "apply",
  "reply",
  "supply",
  "imply",
  "rely",
  "july",
  "italy",
  "assembly",
  "ugly",
  "holy",
  "ally",
  "belly",
  "bully",
  "jelly",
  "lily",
  "fly",
  "comply",
  "multiply",
]);

type Word = { key: string; index: number; end: number };

export const passiveVoice: Rule = {
  id: "passive-voice",
  labelKey: "rules.passive.label",
  languages: ["en"],
  defaultEnabled: true,
  params: [{ key: "maxAdverbs", kind: "int", labelKey: "rules.passive.maxAdverbs", default: 3, min: 0, max: 6 }],
  defaultScope: "narration",
  check: (doc, config) =>
    passiveSpans(doc.text, Number(config.params.maxAdverbs)).map((span) => ({
      ruleId: "passive-voice",
      start: span.index,
      length: span.length,
      messageKey: "hints.passive" as const,
    })),
};

function passiveSpans(text: string, maxAdverbs: number): { index: number; length: number }[] {
  const found = words(text);
  const hints: { index: number; length: number }[] = [];

  for (let i = 0; i < found.length; i++) {
    const be = found[i];
    if (!be || !BE_VERBS.has(be.key)) continue;

    const participle = participleAfter(text, found, i, maxAdverbs);
    if (!participle) continue;

    hints.push({
      index: be.index,
      length: participle.word.end - be.index,
    });
    i = participle.index;
  }

  return hints;
}

function participleAfter(
  text: string,
  found: readonly Word[],
  beIndex: number,
  maxAdverbs: number,
): { word: Word; index: number } | null {
  const be = found[beIndex];
  if (!be) return null;

  let end = be.end;
  let adverbs = 0;

  for (let j = beIndex + 1; j < found.length; j++) {
    const next = found[j];
    if (!next || !isOpenGap(text.slice(end, next.index))) return null;

    if (adverbs < maxAdverbs && isAdverb(next.key) && !isParticiple(next.key)) {
      adverbs += 1;
      end = next.end;
      continue;
    }

    return isParticiple(next.key) ? { word: next, index: j } : null;
  }

  return null;
}

function words(text: string): Word[] {
  const found: Word[] = [];
  for (const match of text.matchAll(/[A-Za-z]+(?:['’‘][A-Za-z]+)*/g)) {
    if (match.index === undefined) continue;
    const raw = match[0];
    found.push({
      key: raw.toLowerCase().replaceAll("’", "'").replaceAll("‘", "'"),
      index: match.index,
      end: match.index + raw.length,
    });
  }
  return found;
}

function isOpenGap(gap: string): boolean {
  return !/[\r\n.?!:;0-9]/.test(gap);
}

function isAdverb(key: string): boolean {
  if (ADVERBS.has(key)) return true;
  return key.length >= 5 && key.endsWith("ly") && !NOT_ADVERBS.has(key);
}

function isParticiple(key: string): boolean {
  if (ADJECTIVAL.has(key)) return false;
  if (IRREGULAR.has(key)) return true;
  if (NOT_PARTICIPLES.has(key)) return false;
  if (key.length < 4 || !key.endsWith("ed")) return false;
  return !key.endsWith("eed");
}
