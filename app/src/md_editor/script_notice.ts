import { defaultLocale, formatMessage } from "../i18n/messages";

export function scriptNotice(addError: string | null, changed: boolean, notice: string | null): { text: string; tone: "error" | "warning" } | null {
  if (addError !== null) return { text: addError, tone: "error" };
  const text = changed ? formatMessage(defaultLocale, "script.scenesChanged") : notice;
  return text === null ? null : { text, tone: "warning" };
}
