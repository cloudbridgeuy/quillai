import { useEffect, useId, useRef } from "react";
import type { ComponentProps, ReactNode, RefObject } from "react";
import { cx } from "./cx";

type ModalProps = Omit<ComponentProps<"dialog">, "title" | "onClose" | "children" | "open"> & {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  returnFocusRef?: RefObject<HTMLElement | null>;
};

export function modalClassName(className?: string): string {
  return cx("m-auto rounded-card border border-rule bg-paper p-6 text-ui text-ink shadow-panel backdrop:bg-ink/50", className);
}

export function modalReturnFocus<T extends Pick<HTMLElement, "focus" | "tagName">>(explicit: T | null | undefined, active: T | null): T | null {
  return explicit ?? (active?.tagName === "BODY" ? null : active);
}

export function Modal({ title, onClose, children, footer, returnFocusRef, className, ...props }: ModalProps) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();

  useEffect(() => {
    const dialog = ref.current!;
    const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const opener = modalReturnFocus(returnFocusRef?.current, active);
    dialog.showModal();
    return () => {
      dialog.close();
      opener?.focus();
    };
  }, [returnFocusRef]);

  return (
    <dialog {...props} ref={ref} aria-labelledby={titleId} className={modalClassName(className)}
      onCancel={(event) => { event.preventDefault(); onClose(); }}
      onClick={(event) => {
        const rect = event.currentTarget.getBoundingClientRect();
        if (event.target === event.currentTarget && (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom)) onClose();
      }}>
      <h2 id={titleId} className="m-0 text-title font-semibold text-slate">{title}</h2>
      <div className="py-4">{children}</div>
      {footer != null ? <footer className="flex justify-end gap-2">{footer}</footer> : null}
    </dialog>
  );
}
