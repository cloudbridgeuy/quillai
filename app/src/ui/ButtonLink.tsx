import type { ComponentProps } from "react";
import { buttonClassName, type ButtonSize, type ButtonVariant } from "./Button";
import { cx } from "./cx";

type ButtonLinkProps = ComponentProps<"a"> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
};

export function ButtonLink({ variant, size, className, ...props }: ButtonLinkProps) {
  return <a {...props} className={buttonClassName({ variant, size, className: cx("inline-flex items-center justify-center", className) })} />;
}
