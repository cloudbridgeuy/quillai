import { useId } from "react";
import { Field } from "./Field";
import { TextField } from "./TextField";

export type SelectOption = { value: string; label: string };

type SelectProps = {
  label: string;
  value: string;
  options: readonly SelectOption[];
  onChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
  "aria-describedby"?: string;
};

/** Native select with a visible label. Values are strings; the caller maps them. */
export function Select({ label, value, options, onChange, disabled, className, "aria-describedby": describedBy }: SelectProps) {
  const id = useId();
  return (
    <Field label={label} htmlFor={id} className={className}>
      <TextField as="select" id={id} value={value} disabled={disabled} aria-describedby={describedBy}
        onChange={(event) => onChange(event.currentTarget.value)}>
        {options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
      </TextField>
    </Field>
  );
}
