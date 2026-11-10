import type { SeriesRules } from "../../settings/client";
import type { Hint, Language, StoredRules } from "./engine";

export const HINT_DEBOUNCE_MS = 400;

export type Gate = { ready: boolean; aiActive: boolean; composing: boolean };
export type Timers = { set: (callback: () => void, ms: number) => unknown; clear: (handle: unknown) => void };
export type HintRunnerOptions = {
  engine: { run(text: string, language: Language, stored: StoredRules): Hint[] };
  getText: () => string;
  onHints: (hints: Hint[]) => void;
  timers?: Timers;
  delay?: number;
};

export const browserTimers: Timers = { set: (callback, ms) => setTimeout(callback, ms), clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>) };

export function createHintRunner({ engine, getText, onHints, timers = browserTimers, delay = HINT_DEBOUNCE_MS }: HintRunnerOptions) {
  let config: SeriesRules | null | undefined;
  let gate: Gate = { ready: false, aiActive: false, composing: false };
  let dirty = true;
  let handle: unknown;
  const blocked = () => !gate.ready || gate.aiActive || gate.composing;

  function cancel() {
    if (handle !== undefined) timers.clear(handle);
    handle = undefined;
  }

  function run() {
    cancel();
    if (blocked() || config === undefined) {
      dirty = true;
      return;
    }
    dirty = false;
    onHints(config ? engine.run(getText(), config.language, config.rules) : []);
  }

  return {
    textChanged(source: string) {
      if (source !== "user") {
        dirty = true;
        return;
      }
      cancel();
      handle = timers.set(run, delay);
    },
    setConfig(next: SeriesRules | null) {
      config = next;
      run();
    },
    setGate(next: Partial<Gate>) {
      const wasBlocked = blocked();
      gate = { ...gate, ...next };
      if (wasBlocked && !blocked() && dirty) run();
    },
    recheck() {
      dirty = true;
      run();
    },
    dispose: cancel,
  };
}
