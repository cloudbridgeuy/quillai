import { expect, test } from "bun:test";
import { checkScript, scriptNudge, scriptRefetch, scriptSchedule, scriptSuspend } from "./script_scheduler";

const facts = [{ id: 1, revision: 4, pending: false, busy: false, reserved: false }, { id: 2, revision: 7, pending: false, busy: false, reserved: false }];

test("script idle fallback and pending saves read the complete group", () => {
  const state = scriptSchedule(0);
  expect(checkScript(state, facts, 14_999).ids).toEqual([]);
  expect(checkScript(state, facts, 15_000).ids).toEqual([1, 2]);
  expect(checkScript(state, [{ ...facts[0]!, pending: true }, facts[1]!], 1000).ids).toEqual([1, 2]);
});

test("script nudge during a running round produces exactly one subsequent group round", () => {
  let result = checkScript(scriptRefetch(scriptSchedule(0)), facts, 1);
  const rounds = [result.ids];
  const nudged = scriptNudge(scriptNudge(result.state, 2, 9), 2, 8);
  result = checkScript(nudged, [{ ...facts[0]!, busy: true }, facts[1]!], 2);
  expect(result.ids).toEqual([]);
  result = checkScript(result.state, [facts[0]!, { ...facts[1]!, revision: 8 }], 3);
  rounds.push(result.ids);
  result = checkScript(result.state, [facts[0]!, { ...facts[1]!, revision: 9 }], 4);
  expect(result.ids).toEqual([]);
  expect(checkScript(result.state, [facts[0]!, { ...facts[1]!, revision: 9 }], 1000).ids).toEqual([]);
  expect(rounds).toEqual([[1, 2], [1, 2]]);
});

test("script drops stale and nonmember nudges and defers the complete group during reservation", () => {
  let state = scriptNudge(scriptNudge(scriptSchedule(0), 2, 7), 99, 50);
  expect(checkScript(state, facts, 1).ids).toEqual([]);
  state = scriptNudge(state, 2, 9);
  const held = checkScript(state, [{ ...facts[0]!, reserved: true }, facts[1]!], 2);
  expect(held.ids).toEqual([]);
  const released = checkScript(held.state, facts, 3);
  expect(released.ids).toEqual([1, 2]);
  expect(checkScript(released.state, [facts[0]!, { ...facts[1]!, revision: 9 }], 4).ids).toEqual([]);
});

test("script refetch survives running rounds and hidden suspension", () => {
  let state = scriptRefetch(scriptSchedule(0));
  let result = checkScript(state, [{ ...facts[0]!, busy: true }, facts[1]!], 1);
  expect(result.ids).toEqual([]);
  state = scriptSuspend(scriptRefetch(result.state), true);
  expect(checkScript(state, facts, 20_000).ids).toEqual([]);
  state = scriptSuspend(state, false);
  expect(checkScript(state, facts, 20_001).ids).toEqual([1, 2]);
});
