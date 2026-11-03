import { defaultLocale, formatMessage } from "./i18n/messages";

export function apiErrorMessage(code: string, detail: string, locale = defaultLocale): string {
  if (code === "AI_NOT_CONFIGURED") return formatMessage(locale, "error.aiNotConfigured");
  if (code === "SQLX") return formatMessage(locale, "error.storage");
  if (code === "INTERNAL_SERVER_ERROR") return formatMessage(locale, "error.server");
  return detail;
}
