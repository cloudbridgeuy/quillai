import { ManuscriptApiError } from "../manuscripts/client";
import type { Revision, ScenePoll } from "../manuscripts/contract";

type Waiter = { since: number; resolve: (revisions: Revision[]) => void; reject: (error: unknown) => void };
type Round = { expected: Set<number>; waiting: Map<number, Waiter> };
export type ScenePollTarget = { id: number; tick: () => Promise<void> };

export function createScenePoll(fetchGroup: (since: ReadonlyMap<number, number>) => Promise<ScenePoll>) {
  let round: Round | null = null;

  function request(waiting: Map<number, Waiter>) {
    fetchGroup(new Map([...waiting].map(([id, waiter]) => [id, waiter.since]))).then((poll) => {
      for (const [id, waiter] of waiting) {
        const scene = poll.scenes.get(id);
        if (scene === undefined) waiter.reject(new ManuscriptApiError(`Scene ${id} is missing from the poll.`, 200, "INVALID_RESPONSE", "invalid_response"));
        else if ("conflict" in scene) waiter.reject(new ManuscriptApiError(`Scene history is ${scene.conflict}.`, 409, "CONFLICT", "http"));
        else waiter.resolve(scene.revisions);
      }
    }, (error: unknown) => {
      for (const waiter of waiting.values()) waiter.reject(error);
    });
  }

  function settle(current: Round) {
    if (current.expected.size > 0 || round !== current) return;
    round = null;
    if (current.waiting.size > 0) request(current.waiting);
  }

  return {
    getDeltas(id: number, since: number): Promise<Revision[]> {
      return new Promise((resolve, reject) => {
        const current = round;
        if (current?.expected.delete(id)) {
          current.waiting.set(id, { since, resolve, reject });
          settle(current);
        } else {
          request(new Map([[id, { since, resolve, reject }]]));
        }
      });
    },
    tick(targets: readonly ScenePollTarget[]) {
      if (round !== null || targets.length === 0) return;
      const current: Round = { expected: new Set(targets.map(({ id }) => id)), waiting: new Map() };
      round = current;
      for (const { id, tick } of targets) {
        void tick().finally(() => {
          current.expected.delete(id);
          settle(current);
        });
      }
    },
  };
}
