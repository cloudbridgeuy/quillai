import type { ComponentProps } from "react";
import { cx } from "./cx";

type TextFieldProps =
  | (ComponentProps<"input"> & { as?: "input"; square?: boolean })
  | (ComponentProps<"select"> & { as: "select"; square?: boolean });

export function textFieldClassName({ square, className }: { square?: boolean; className?: string }): string {
  return cx(
    "w-full border border-rule bg-paper px-3 py-2 text-ui text-ink outline-none",
    "focus:border-slate disabled:cursor-not-allowed disabled:opacity-75",
    square ? "rounded-none" : "rounded-card",
    className,
  );
}

export function TextField(props: TextFieldProps) {
  if (props.as === "select") {
    const { as: _as, square, className, ...rest } = props;
    return <select {...rest} className={textFieldClassName({ square, className })} />;
  }
  const { as: _as, square, className, ...rest } = props;
  return <input {...rest} className={textFieldClassName({ square, className })} />;
}
