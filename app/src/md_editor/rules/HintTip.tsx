import Quill from "quill";
import { useEffect, useMemo, useRef, useState } from "react";
import { defaultLocale, formatMessage, type Locale } from "../../i18n/messages";
import { Popover } from "../../ui/Popover";
import { RULES, type Hint, type Rule } from "./engine";
import { locateHint, type At, type Located } from "./hint_geometry";
import { browserTimers, type Timers } from "./hint_runner";

export const HINT_TIP_DELAY_MS = 300;

const sameHint = (a: Hint, b: Hint) => a.ruleId === b.ruleId && a.start === b.start && a.length === b.length;

export function HintTip({ quill, hints, rules = RULES, timers = browserTimers, locale = defaultLocale }: { quill: Quill; hints: readonly Hint[]; rules?: readonly Rule[]; timers?: Timers; locale?: Locale }) {
  const t = (key: Hint["messageKey"]) => formatMessage(locale, key);
  const [tip, setTip] = useState<{ hint: Hint; anchor: { left: number; top: number } } | null>(null);
  const hintsRef = useRef(hints);
  hintsRef.current = hints;
  const tipRef = useRef(tip);
  tipRef.current = tip;
  const rootRef = useMemo(() => ({ current: quill.root }), [quill]);
  const ignoreRefs = useMemo(() => [rootRef], [rootRef]);

  useEffect(() => {
    const locate = (at: At): Located | undefined => {
      const container = quill.container.getBoundingClientRect();
      const visible = { top: -container.top, bottom: window.innerHeight - container.top };
      return locateHint(hintsRef.current, quill.getText(), (index, length) => quill.getBounds(index, length), visible, at);
    };
    const open = (found: Located | undefined) => {
      if (!found) return setTip(null);
      const container = quill.container.getBoundingClientRect();
      setTip({ hint: found.hint, anchor: { left: container.left + found.rect.left, top: container.top + found.rect.top + found.rect.height } });
    };
    const pointAt = (event: MouseEvent): At => {
      const container = quill.container.getBoundingClientRect();
      return { point: { x: event.clientX - container.left, y: event.clientY - container.top } };
    };
    let rest: unknown;
    const cancelRest = () => { if (rest !== undefined) timers.clear(rest); rest = undefined; };
    let wasOpenOnHint = false;
    const onMove = (event: MouseEvent) => {
      cancelRest();
      const at = pointAt(event);
      rest = timers.set(() => { rest = undefined; open(locate(at)); }, HINT_TIP_DELAY_MS);
    };
    const onDown = (event: MouseEvent) => {
      const found = locate(pointAt(event));
      wasOpenOnHint = found !== undefined && tipRef.current !== null && sameHint(found.hint, tipRef.current.hint);
    };
    const onClick = (event: MouseEvent) => {
      const found = locate(pointAt(event));
      if (wasOpenOnHint) setTip(null);
      else if (found) open(found);
    };
    const onSelection = (range: { index: number; length: number } | null) => {
      if (!range || range.length > 0) return setTip(null);
      open(locate({ index: range.index }));
    };
    quill.root.addEventListener("pointermove", onMove);
    quill.root.addEventListener("pointerdown", onDown);
    quill.root.addEventListener("click", onClick);
    quill.on(Quill.events.SELECTION_CHANGE, onSelection);
    return () => {
      cancelRest();
      quill.root.removeEventListener("pointermove", onMove);
      quill.root.removeEventListener("pointerdown", onDown);
      quill.root.removeEventListener("click", onClick);
      quill.off(Quill.events.SELECTION_CHANGE, onSelection);
    };
  }, [quill, timers]);

  const current = tip && hints.some((hint) => sameHint(hint, tip.hint)) ? tip : null;
  if (!current) return null;
  const rule = rules.find((candidate) => candidate.id === current.hint.ruleId);
  return <Popover open anchor={current.anchor} onOpenChange={(open) => { if (!open) setTip(null); }} ignoreRefs={ignoreRefs} role="tooltip" onMouseDown={(event) => event.preventDefault()}>
    {rule ? <strong className="block">{t(rule.labelKey)}</strong> : null}
    <span>{t(current.hint.messageKey)}</span>
  </Popover>;
}
