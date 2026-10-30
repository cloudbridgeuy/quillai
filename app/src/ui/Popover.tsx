import { cloneElement, useEffect, useId, useImperativeHandle, useRef } from "react";
import type { ComponentProps, CSSProperties, ReactElement, ReactNode, RefObject } from "react";
import { createPortal } from "react-dom";
import { cx } from "./cx";

type PopoverProps = ComponentProps<"div"> & {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
  ignoreRefs?: readonly RefObject<HTMLElement | null>[];
} & ({ anchor: { left: number; top: number }; trigger?: ReactElement<ComponentProps<"button">> }
  | { anchor?: never; trigger: ReactElement<ComponentProps<"button">> });

export function popoverClassName(className?: string): string {
  return cx("absolute z-10 rounded-card border border-rule bg-paper p-4 text-ui text-ink shadow-popover", className);
}

export function Popover({ trigger, anchor, open, onOpenChange, children, className, style, ignoreRefs, ref: forwardedRef, ...props }: PopoverProps) {
  const ref = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const id = useId();
  useImperativeHandle<HTMLDivElement | null, HTMLDivElement | null>(forwardedRef, () => panelRef.current, [open]);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (ignoreRefs?.some(ignored => ignored.current?.contains(event.target as Node))) return;
      if (!ref.current?.contains(event.target as Node) && !panelRef.current?.contains(event.target as Node)) onOpenChange(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onOpenChange(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open, onOpenChange, ignoreRefs]);

  const panel = open ? <div {...props} ref={panelRef} id={id} className={popoverClassName(className)}
    style={anchor ? { ...style, position: "fixed", left: "var(--popover-left, var(--popover-anchor-x))", top: "var(--popover-top, var(--popover-anchor-y))", "--popover-anchor-x": `${anchor.left}px`, "--popover-anchor-y": `${anchor.top}px` } as CSSProperties : style}>{children}</div> : null;
  return (
    <div ref={ref} className="relative">
      {trigger ? cloneElement(trigger, {
        "aria-expanded": open,
        "aria-controls": id,
        onClick: (event) => { trigger.props.onClick?.(event); if (!event.defaultPrevented) onOpenChange(!open); },
      }) : null}
      {anchor && typeof document !== "undefined" && panel ? createPortal(panel, document.body) : panel}
    </div>
  );
}
