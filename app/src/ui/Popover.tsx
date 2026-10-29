import { cloneElement, useEffect, useId, useRef } from "react";
import type { ComponentProps, ReactElement, ReactNode } from "react";
import { cx } from "./cx";

type PopoverProps = ComponentProps<"div"> & {
  trigger: ReactElement<ComponentProps<"button">>;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
};

export function popoverClassName(className?: string): string {
  return cx("absolute z-10 rounded-card border border-rule bg-paper p-4 text-ui text-ink shadow-popover", className);
}

export function Popover({ trigger, open, onOpenChange, children, className, ...props }: PopoverProps) {
  const ref = useRef<HTMLDivElement>(null);
  const id = useId();

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!ref.current?.contains(event.target as Node)) onOpenChange(false);
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
  }, [open, onOpenChange]);

  return (
    <div ref={ref} className="relative">
      {cloneElement(trigger, {
        "aria-expanded": open,
        "aria-controls": id,
        onClick: (event) => { trigger.props.onClick?.(event); if (!event.defaultPrevented) onOpenChange(!open); },
      })}
      {open ? <div {...props} id={id} className={popoverClassName(className)}>{children}</div> : null}
    </div>
  );
}
