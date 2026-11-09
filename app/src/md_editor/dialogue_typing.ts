export type DialogueReplacement = { remove: number; insert: string } | null;

const OPENING_CONTEXT = /^$|[\s(\[{—–“‘«]$/;

export function dialogueReplacement(lineBeforeCaret: string, typed: string, ruleMadeOpenQuote = false): DialogueReplacement {
  const opens = OPENING_CONTEXT.test(lineBeforeCaret);
  if (typed === "\"") return { remove: 1, insert: opens ? "“" : "”" };
  if (typed === "'") return { remove: 1, insert: opens ? "‘" : "’" };
  if (typed === "-" && lineBeforeCaret.endsWith("-")) return { remove: 2, insert: "—" };
  if (/^[0-9]$/.test(typed) && ruleMadeOpenQuote && lineBeforeCaret.endsWith("‘")) return { remove: 2, insert: `’${typed}` };
  return null;
}
