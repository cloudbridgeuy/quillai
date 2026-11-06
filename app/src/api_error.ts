import { defaultLocale, formatMessage, type MessageKey } from "./i18n/messages";

const codeMessages: Record<string, MessageKey> = {
  SQLX: "error.storage",
  INVALID_LOG_LEVEL: "error.invalidLogLevel",
  NOT_FOUND: "error.notFound",
  CONFLICT: "error.conflict",
  VALIDATION_ERROR: "error.validation",
  INVALID_JSON: "error.invalidRequest",
  AI_NOT_CONFIGURED: "error.aiNotConfigured",
  AI_PROVIDER_CONFIG_ERROR: "error.aiProviderConfig",
  INTERNAL_SERVER_ERROR: "error.server",
  INVALID_RESPONSE: "error.invalidResponse",
};

/** Translates by error code only; the server `detail` is never shown (clients keep it in `.detail`). */
export function apiErrorMessage(code: string, _detail: string, locale = defaultLocale): string {
  const key = Object.hasOwn(codeMessages, code) ? codeMessages[code] : "error.generic";
  return formatMessage(locale, key);
}
