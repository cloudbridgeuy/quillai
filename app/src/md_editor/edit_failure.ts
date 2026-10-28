import { defaultLocale, formatMessage } from "../i18n/messages";

export function getEditFailurePresentation(error: string): { label: string; title?: string } {
  if (error === "AI edit response did not include a <replacement> tag." ||
      error === "AI edit response ended before the closing </replacement> tag.") {
    return { label: formatMessage(defaultLocale, "edit.failure"), title: error };
  }
  return { label: error };
}
