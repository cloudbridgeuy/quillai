// English is the source shape. Spanish must satisfy MessageShape<typeof en>,
// so a missing key, an extra key, or a string/plural mismatch is a type error
// on a compiler that can check this project. See docs/i18n.md.

export const locales = ["en", "es"] as const;
export type Locale = (typeof locales)[number];
export const defaultLocale: Locale = "en";

export type PluralMessage = {
  readonly other: string;
  readonly zero?: string;
  readonly one?: string;
  readonly two?: string;
  readonly few?: string;
  readonly many?: string;
};

export type MessageShape<Source> = {
  [Key in keyof Source]: Source[Key] extends string ? string : PluralMessage;
};

export const en = {
  "inspector.fullPrompt": "Full prompt",
  "inspector.copy": "Copy",
  "inspector.copyFailed": "Could not copy prompt.",
  "inspector.caretAfterSelection": "Caret after “{text}”: Continue writes here.",
  "action.save": "Save",
  "action.saving": "Saving…",
  "shortcut.title": "{action} ({keys})",
  "shortcut.continue": "Continue writing",
  "shortcut.stop": "Stop AI",
  "shortcut.edit": "Edit selection",
  "thread.cutOff": "Continuing from a cut-off reply",
  "thread.discard": "Discard",
  "thread.resumeIndicator": "from a cut-off reply",
  "thread.resumeAction": "Continue writing from a cut-off reply",
  "thread.resumeOption": "Continue · from a cut-off reply",
  "thread.resuming": "Continuing from a cut-off reply…",
  "thread.noResume": "No saved reply will be resumed.",
  "greeting.hello": "Hello, {name}",
  "chapter.count": {
    one: "{count} chapter",
    other: "{count} chapters",
  },
  "draft.words": "{count} words",
  "edit.summary": "Edited {when} for {price}",
  "edit.failure": "The model didn't return an edit. Try again or switch models.",
  "error.storage": "Couldn't reach your manuscript's storage. Your text is still here; try again.",
  "error.server": "Something went wrong on the server. Your text is still here; try again.",
} as const satisfies Record<string, string | PluralMessage>;

export type MessageKey = keyof typeof en;

export const es = {
  "inspector.fullPrompt": "Prompt completo",
  "inspector.copy": "Copiar",
  "inspector.copyFailed": "No se pudo copiar el prompt.",
  "inspector.caretAfterSelection": "Cursor después de «{text}»: Continuar escribe aquí.",
  "action.save": "Guardar",
  "action.saving": "Guardando…",
  "shortcut.title": "{action} ({keys})",
  "shortcut.continue": "Continuar escribiendo",
  "shortcut.stop": "Detener IA",
  "shortcut.edit": "Editar selección",
  "thread.cutOff": "Continuando una respuesta interrumpida",
  "thread.discard": "Descartar",
  "thread.resumeIndicator": "desde una respuesta interrumpida",
  "thread.resumeAction": "Continuar escribiendo desde una respuesta interrumpida",
  "thread.resumeOption": "Continuar · desde una respuesta interrumpida",
  "thread.resuming": "Continuando desde una respuesta interrumpida…",
  "thread.noResume": "No se continuará ninguna respuesta guardada.",
  "greeting.hello": "Hola, {name}",
  "chapter.count": {
    one: "{count} capítulo",
    other: "{count} capítulos",
  },
  "draft.words": "{count} palabras",
  "edit.summary": "Editado el {when} por {price}",
  "edit.failure": "El modelo no devolvió una edición. Inténtalo de nuevo o cambia de modelo.",
  "error.storage": "No se pudo acceder al almacenamiento de tu manuscrito. Tu texto sigue aquí; inténtalo de nuevo.",
  "error.server": "Ocurrió un error en el servidor. Tu texto sigue aquí; inténtalo de nuevo.",
} as const satisfies MessageShape<typeof en>;

export type CurrencyAmount = {
  readonly amount: number;
  readonly currency: string;
};

export type FormatValue = string | number | Date | CurrencyAmount;
export type FormatValues = Readonly<Record<string, FormatValue>>;

export type MessageCatalog = {
  readonly en: Partial<Record<MessageKey, string | PluralMessage>>;
  readonly es: Partial<Record<MessageKey, string | PluralMessage>>;
};

export const messages: MessageCatalog = { en, es };

const messageKeys = new Set<string>(Object.keys(en));

function isMessageKey(key: string): key is MessageKey {
  return messageKeys.has(key);
}

export function resolveLocale(locale: string): Locale {
  const tag = locale.trim().toLowerCase();
  if (tag === "es" || tag.startsWith("es-")) return "es";
  return defaultLocale;
}

function lookup(catalog: MessageCatalog, locale: Locale, key: string): string | PluralMessage | undefined {
  if (!isMessageKey(key)) return undefined;
  const local = catalog[locale][key];
  if (local !== undefined) return local;
  if (locale === defaultLocale) return undefined;
  return catalog[defaultLocale][key];
}

function categoryText(message: PluralMessage, category: string): string {
  switch (category) {
    case "zero":
      return message.zero ?? message.other;
    case "one":
      return message.one ?? message.other;
    case "two":
      return message.two ?? message.other;
    case "few":
      return message.few ?? message.other;
    case "many":
      return message.many ?? message.other;
    default:
      return message.other;
  }
}

function pluralTemplate(locale: Locale, message: PluralMessage, values: FormatValues | undefined): string {
  const count = values?.count;
  if (typeof count !== "number" || !Number.isFinite(count)) return message.other;
  return categoryText(message, new Intl.PluralRules(locale).select(count));
}

function formatValue(locale: Locale, value: FormatValue): string {
  if (typeof value === "string") return value;
  if (typeof value === "number") return new Intl.NumberFormat(locale).format(value);
  if (value instanceof Date) {
    return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeZone: "UTC" }).format(value);
  }
  return new Intl.NumberFormat(locale, { style: "currency", currency: value.currency }).format(value.amount);
}

function interpolate(locale: Locale, template: string, values: FormatValues | undefined): string {
  return template.replace(/\{([A-Za-z_][A-Za-z0-9_]*)\}/g, (raw, name: string) => {
    if (values === undefined || !Object.prototype.hasOwnProperty.call(values, name)) return raw;
    return formatValue(locale, values[name]);
  });
}

export function formatMessage(
  locale: string,
  key: string,
  values?: FormatValues,
  catalog: MessageCatalog = messages,
): string {
  const resolved = resolveLocale(locale);
  const message = lookup(catalog, resolved, key);
  if (message === undefined) return key;
  const template = typeof message === "string" ? message : pluralTemplate(resolved, message, values);
  return interpolate(resolved, template, values);
}
