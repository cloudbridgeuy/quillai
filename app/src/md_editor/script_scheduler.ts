import { checkPush, pushState, requestRefetch, type PushState } from "./sync_push";

export type SceneFacts = { id: number; revision: number; pending: boolean; busy: boolean; reserved: boolean };
export type ScriptSchedule = { push: PushState; revisions: Map<number, number> };
export function scriptSchedule(now: number): ScriptSchedule {
  return { push: pushState(now), revisions: new Map() };
}
export function scriptNudge(state: ScriptSchedule, id: number, revision: number): ScriptSchedule {
  const revisions = new Map(state.revisions);
  revisions.set(id, Math.max(revisions.get(id) ?? 0, revision));
  return { ...state, revisions };
}
export function scriptRefetch(state: ScriptSchedule): ScriptSchedule {
  return { ...state, push: requestRefetch(state.push) };
}
export function scriptSuspend(state: ScriptSchedule, suspended: boolean): ScriptSchedule {
  const next = { ...state, push: { ...state.push, suspended } };
  return suspended ? next : scriptRefetch(next);
}
export function checkScript(state: ScriptSchedule, facts: SceneFacts[], now: number): { state: ScriptSchedule; ids: number[] } {
  const revisions = new Map(facts.flatMap(({ id, revision }) => {
    const advertised = state.revisions.get(id) ?? 0;
    return advertised > revision ? [[id, advertised] as const] : [];
  }));
  const busy = facts.some(scene => scene.busy);
  const reserved = facts.length === 0 || facts.some(scene => scene.reserved);
  const push = revisions.size > 0 && !busy && !reserved ? requestRefetch(state.push) : state.push;
  const result = checkPush(push, { revision: 0, pending: facts.some(scene => scene.pending), busy, reserved }, now);
  return { state: { push: result.state, revisions }, ids: result.tick ? facts.map(scene => scene.id) : [] };
}
