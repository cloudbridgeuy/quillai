import Quill from "quill";
import { useCallback, useEffect, useRef, useState } from "react";
import type { createSettingsClient, SeriesRules } from "../../settings/client";
import type { Hint } from "./engine";
import { RULES, createEngine } from "./engine";
import { createHintRunner, type HintRunnerOptions, type Timers } from "./hint_runner";

export type UseHintRunnerOptions = {
  quill: Quill | null;
  client: Pick<ReturnType<typeof createSettingsClient>, "getSeriesRules">;
  seriesId: number | null;
  ready: boolean;
  aiActive: boolean;
  engine?: HintRunnerOptions["engine"];
  timers?: Timers;
};

const defaultEngine = createEngine(RULES);

export function useHintRunner({ quill, client, seriesId, ready, aiActive, engine = defaultEngine, timers }: UseHintRunnerOptions) {
  const [hints, setHints] = useState<Hint[]>([]);
  const runner = useRef<ReturnType<typeof createHintRunner> | null>(null);

  useEffect(() => {
    if (!quill) return;
    const created = createHintRunner({ engine, getText: () => quill.getText(), onHints: setHints, timers });
    runner.current = created;
    const onTextChange = (_delta: unknown, _old: unknown, source: string) => created.textChanged(source);
    const compose = (composing: boolean) => () => created.setGate({ composing });
    const start = compose(true);
    const end = compose(false);
    quill.on(Quill.events.TEXT_CHANGE, onTextChange);
    quill.root.addEventListener("compositionstart", start);
    quill.root.addEventListener("compositionend", end);
    return () => {
      quill.off(Quill.events.TEXT_CHANGE, onTextChange);
      quill.root.removeEventListener("compositionstart", start);
      quill.root.removeEventListener("compositionend", end);
      created.dispose();
      runner.current = null;
    };
  }, [quill, engine, timers]);

  useEffect(() => {
    runner.current?.setGate({ ready, aiActive });
  }, [quill, engine, timers, ready, aiActive]);

  useEffect(() => {
    if (!quill) return;
    if (seriesId === null) {
      runner.current?.setConfig(null);
      return;
    }
    let active = true;
    client.getSeriesRules(seriesId)
      .then((value) => active && runner.current?.setConfig(value))
      .catch(() => active && runner.current?.setConfig(null));
    return () => { active = false; };
  }, [quill, engine, timers, client, seriesId]);

  const setConfig = useCallback((config: SeriesRules | null) => runner.current?.setConfig(config), []);
  const recheck = useCallback(() => runner.current?.recheck(), []);
  return { hints, setConfig, recheck };
}
