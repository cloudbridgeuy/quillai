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
  "app.loading": "Loading…",
  "app.name": "QuillAI",
  "error.unexpected": "Something went wrong: {message}",
  "error.retry": "Try again",
  "landing.activity.title": "Recent activity",
  "landing.activity.loadFailed": "Could not load recent activity.",
  "landing.open": "Open",
  "landing.checking": "Checking…",
  "landing.checkFailed": "Could not check it: {message}",
  "landing.forget": "Forget",
  "landing.empty": "Nothing edited in this browser yet. Open or create a piece below.",
  "landing.yourLibrary": "Your library",
  "landing.resume": "Resume",
  "landing.library": "Library",
  "landing.tips": "Tips",
  "alert.dismiss": "Dismiss",
  "landing.deleted.title": "Recently deleted",
  "landing.deleted.loadFailed": "Could not load deleted items",
  "landing.deleted.loadFailedEmpty": "Could not load deleted items.",
  "landing.deleted.actionFailed": "Action failed",
  "landing.deleted.retry": "Retry",
  "landing.deleted.empty": "No recently deleted items.",
  "landing.deleted.restoreNotice": "Restoring this item also restores: {parents}.",
  "landing.deleted.deletePrompt": "Delete {title} forever?",
  "landing.deleted.deleteWithCounts": "Delete {title} forever with {counts}?",
  "landing.deleted.deleteWithParents": "Delete {title} forever? This also restores: {parents}.",
  "landing.deleted.deleteWithCountsAndParents": "Delete {title} forever with {counts}? This also restores: {parents}.",
  "landing.deleted.confirmRestore": "Confirm restore",
  "landing.deleted.restore": "Restore",
  "landing.deleted.confirmDelete": "Confirm delete",
  "landing.deleted.deleteForever": "Delete forever",
  "landing.deleted.chapters": { other: "{countText} chapters" },
  "landing.deleted.pieces": { other: "{countText} pieces" },
  "landing.deleted.lore": { other: "{countText} lore" },
  "settings.ai.loadFailed": "Could not load AI settings.",
  "settings.ai.saveFailed": "Could not save AI settings.",
  "settings.ai.savedHere": "Saved here",
  "settings.ai.fromEnv": "From quillai.env",
  "settings.ai.notSet": "Not set",
  "settings.ai.title": "AI",
  "settings.ai.baseUrl": "Base URL",
  "settings.ai.model": "Model",
  "settings.ai.keySaved": "Key saved",
  "settings.ai.noKey": "No key set",
  "settings.ai.replace": "Replace",
  "settings.ai.remove": "Remove",
  "settings.ai.replacementKey": "Replacement API key",
  "settings.ai.save": "Save AI settings",
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
  "error.aiNotConfigured": "AI isn't set up yet. Open Settings → AI to add your provider.",
  "error.storage": "Couldn't reach your manuscript's storage. Your text is still here; try again.",
  "error.server": "Something went wrong on the server. Your text is still here; try again.",
} as const satisfies Record<string, string | PluralMessage>;

export type MessageKey = keyof typeof en;

export const es = {
  "app.loading": "Cargando…",
  "app.name": "QuillAI",
  "error.unexpected": "Ocurrió un error: {message}",
  "error.retry": "Intentar de nuevo",
  "landing.activity.title": "Actividad reciente",
  "landing.activity.loadFailed": "No se pudo cargar la actividad reciente.",
  "landing.open": "Abrir",
  "landing.checking": "Verificando…",
  "landing.checkFailed": "No se pudo verificar: {message}",
  "landing.forget": "Olvidar",
  "landing.empty": "Aún no has editado nada en este navegador. Abre o crea un texto abajo.",
  "landing.yourLibrary": "Tu biblioteca",
  "landing.resume": "Retomar",
  "landing.library": "Biblioteca",
  "landing.tips": "Consejos",
  "alert.dismiss": "Descartar",
  "landing.deleted.title": "Eliminados recientemente",
  "landing.deleted.loadFailed": "No se pudieron cargar los elementos eliminados",
  "landing.deleted.loadFailedEmpty": "No se pudieron cargar los elementos eliminados.",
  "landing.deleted.actionFailed": "No se pudo completar la acción",
  "landing.deleted.retry": "Reintentar",
  "landing.deleted.empty": "No hay elementos eliminados recientemente.",
  "landing.deleted.restoreNotice": "Al restaurar este elemento también se restaura: {parents}.",
  "landing.deleted.deletePrompt": "¿Eliminar {title} para siempre?",
  "landing.deleted.deleteWithCounts": "¿Eliminar {title} para siempre junto con {counts}?",
  "landing.deleted.deleteWithParents": "¿Eliminar {title} para siempre? Esto también restaura: {parents}.",
  "landing.deleted.deleteWithCountsAndParents": "¿Eliminar {title} para siempre junto con {counts}? Esto también restaura: {parents}.",
  "landing.deleted.confirmRestore": "Confirmar restauración",
  "landing.deleted.restore": "Restaurar",
  "landing.deleted.confirmDelete": "Confirmar eliminación",
  "landing.deleted.deleteForever": "Eliminar para siempre",
  "landing.deleted.chapters": { one: "{countText} capítulo", other: "{countText} capítulos" },
  "landing.deleted.pieces": { one: "{countText} texto", other: "{countText} textos" },
  "landing.deleted.lore": { one: "{countText} entrada de trasfondo", other: "{countText} entradas de trasfondo" },
  "settings.ai.loadFailed": "No se pudo cargar la configuración de IA.",
  "settings.ai.saveFailed": "No se pudo guardar la configuración de IA.",
  "settings.ai.savedHere": "Guardado aquí",
  "settings.ai.fromEnv": "De quillai.env",
  "settings.ai.notSet": "Sin configurar",
  "settings.ai.title": "IA",
  "settings.ai.baseUrl": "URL base",
  "settings.ai.model": "Modelo",
  "settings.ai.keySaved": "Clave guardada",
  "settings.ai.noKey": "Sin clave configurada",
  "settings.ai.replace": "Reemplazar",
  "settings.ai.remove": "Quitar",
  "settings.ai.replacementKey": "Nueva clave de API",
  "settings.ai.save": "Guardar configuración de IA",
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
  "error.aiNotConfigured": "La IA aún no está configurada. Abre Configuración → IA para agregar tu proveedor.",
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
