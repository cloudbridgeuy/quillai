import { expect, test } from "bun:test";
import { apiErrorMessage } from "./api_error";
import { createManuscriptClient } from "./manuscripts/client";
import { createLoreClient } from "./manuscripts/lore_client";
import { createSettingsClient } from "./settings/client";
import { createChapterSession } from "./md_editor/chapter_session";

test("maps server errors in both languages and preserves other errors", () => {
  expect(apiErrorMessage("SQLX", "sqlx error", "en")).toBe("Couldn't reach your manuscript's storage. Your text is still here; try again.");
  expect(apiErrorMessage("SQLX", "sqlx error", "es")).toBe("No se pudo acceder al almacenamiento de tu manuscrito. Tu texto sigue aquí; inténtalo de nuevo.");
  expect(apiErrorMessage("INTERNAL_SERVER_ERROR", "unexpected failure", "en")).toBe("Something went wrong on the server. Your text is still here; try again.");
  expect(apiErrorMessage("INTERNAL_SERVER_ERROR", "unexpected failure", "es")).toBe("Ocurrió un error en el servidor. Tu texto sigue aquí; inténtalo de nuevo.");
  expect(apiErrorMessage("UNKNOWN", "original error")).toBe("Something went wrong. Try again.");
});

test("clients retain raw server error details", async () => {
  for (const [code, detail, message] of [
    ["SQLX", "sqlx error", "Couldn't reach your manuscript's storage. Your text is still here; try again."],
    ["INTERNAL_SERVER_ERROR", "unexpected failure", "Something went wrong on the server. Your text is still here; try again."],
  ]) {
    const fetch = async () => Response.json({ error: detail, code }, { status: 500 });
    const calls = [
      () => createManuscriptClient({ fetch }).getChapter({ seriesId: 1, bookId: 1, chapterId: 1 }),
      () => createLoreClient({ fetch }).list({ universeId: 1 }),
      () => createSettingsClient({ fetch }).getAuthorGuidance(),
    ];
    for (const call of calls) {
      await expect(call()).rejects.toMatchObject({ message, detail, code, status: 500 });
    }
  }
});

test("blocked storage detail survives repeated flush failures without another publish", async () => {
  const fetch = async () => Response.json({ error: "sqlx error", code: "SQLX" }, { status: 500 });
  const details: (string | undefined)[] = [];
  const session = createChapterSession(createManuscriptClient({ fetch }), { seriesId: 1, bookId: 1, chapterId: 1 },
    { content: { ops: [{ insert: "Text\n" }] }, head_rev: 0 },
    (_document, _dirty, problem) => { details.push(problem?.detail); });
  await session.tick();
  for (let attempt = 0; attempt < 2; attempt++) {
    await expect(session.flush()).rejects.toThrow("Couldn't reach your manuscript's storage.");
  }
  expect(details).toEqual(["sqlx error"]);
});

test("hides server details when AI is not configured", () => {
  expect(apiErrorMessage("AI_NOT_CONFIGURED", "missing QUILLAI_AI_API_KEY", "en")).toBe("AI isn't set up yet. Open Settings → AI to add your provider.");
  expect(apiErrorMessage("AI_NOT_CONFIGURED", "missing QUILLAI_AI_MODEL", "es")).toBe("La IA aún no está configurada. Abre Configuración → IA para agregar tu proveedor.");
});

const translated: Record<string, [string, string]> = {
  SQLX: ["Couldn't reach your manuscript's storage. Your text is still here; try again.", "No se pudo acceder al almacenamiento de tu manuscrito. Tu texto sigue aquí; inténtalo de nuevo."],
  INVALID_LOG_LEVEL: ["That log level isn't supported. Pick another one.", "Ese nivel de registro no es compatible. Elige otro."],
  NOT_FOUND: ["We couldn't find that. It may have been moved or deleted.", "No pudimos encontrarlo. Quizá se movió o se eliminó."],
  CONFLICT: ["That changed somewhere else. Reload and try again.", "Cambió en otro lugar. Recarga e inténtalo de nuevo."],
  VALIDATION_ERROR: ["Some of that doesn't look right. Check it and try again.", "Algo no parece correcto. Revísalo e inténtalo de nuevo."],
  INVALID_JSON: ["That request couldn't be read. Try again.", "No se pudo leer esa solicitud. Inténtalo de nuevo."],
  AI_NOT_CONFIGURED: ["AI isn't set up yet. Open Settings → AI to add your provider.", "La IA aún no está configurada. Abre Configuración → IA para agregar tu proveedor."],
  AI_PROVIDER_CONFIG_ERROR: ["Your AI provider settings need attention. Open Settings → AI to check them.", "La configuración de tu proveedor de IA necesita atención. Abre Configuración → IA para revisarla."],
  INTERNAL_SERVER_ERROR: ["Something went wrong on the server. Your text is still here; try again.", "Ocurrió un error en el servidor. Tu texto sigue aquí; inténtalo de nuevo."],
  INVALID_RESPONSE: ["The server sent back something unexpected. Try again.", "El servidor devolvió algo inesperado. Inténtalo de nuevo."],
};

test("every known API error code has its own translation, never server text", () => {
  for (const [code, [en, es]] of Object.entries(translated)) {
    expect(apiErrorMessage(code, "raw server text", "en")).toBe(en);
    expect(apiErrorMessage(code, "raw server text", "es")).toBe(es);
  }
});

test("unknown or missing codes use a generic translated message", () => {
  expect(apiErrorMessage("PROVIDER_ERROR", "raw server text", "en")).toBe("Something went wrong. Try again.");
  expect(apiErrorMessage("PROVIDER_ERROR", "raw server text", "es")).toBe("Algo salió mal. Inténtalo de nuevo.");
  expect(apiErrorMessage(undefined as unknown as string, "raw server text", "en")).toBe("Something went wrong. Try again.");
  expect(apiErrorMessage("", "raw server text", "es")).toBe("Algo salió mal. Inténtalo de nuevo.");
});
