import { expect, test } from "bun:test";

import { feedScriptExchange, finishScriptExchange, SCRIPT_EXCHANGE_START } from "./script_exchange";

test("complete lines map to character, parenthetical, and dialogue blocks", () => {
  const feed = feedScriptExchange(SCRIPT_EXCHANGE_START, "MARA\n(quietly)\nYou came back.\n\nILAN (V.O.)\nI never left.\n");
  expect(feed).toEqual({
    lines: [
      { type: "character", text: "MARA" },
      { type: "parenthetical", text: "(quietly)" },
      { type: "dialogue", text: "You came back." },
      { type: "character", text: "ILAN (V.O.)" },
      { type: "dialogue", text: "I never left." },
    ],
    state: { pending: "", previous: "dialogue" },
  });
});

test("partial lines wait for their newline across chunks", () => {
  const first = feedScriptExchange(SCRIPT_EXCHANGE_START, "MA");
  expect(first).toEqual({ lines: [], state: { pending: "MA", previous: null } });
  const second = feedScriptExchange(first.state, "RA\nWhere ");
  expect(second).toEqual({ lines: [{ type: "character", text: "MARA" }], state: { pending: "Where ", previous: "character" } });
  const third = feedScriptExchange(second.state, "were you?\r\n  ");
  expect(third).toEqual({ lines: [{ type: "dialogue", text: "Where were you?" }], state: { pending: "  ", previous: "dialogue" } });
});

test("finish flushes the last unterminated line and drops blank remainders", () => {
  expect(finishScriptExchange({ pending: "Gone.", previous: "character" })).toEqual([{ type: "dialogue", text: "Gone." }]);
  expect(finishScriptExchange({ pending: "  ", previous: "dialogue" })).toEqual([]);
});

test("a short shouted dialogue line after a character stays dialogue", () => {
  expect(feedScriptExchange(SCRIPT_EXCHANGE_START, "MARA\nNO!\nILAN\n(beat)\nOK.\n").lines).toEqual([
    { type: "character", text: "MARA" },
    { type: "dialogue", text: "NO!" },
    { type: "character", text: "ILAN" },
    { type: "parenthetical", text: "(beat)" },
    { type: "dialogue", text: "OK." },
  ]);
});
