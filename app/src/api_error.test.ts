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
  for (const code of ["VALIDATION_ERROR", "CONFLICT", "NOT_FOUND", "PROVIDER_ERROR", "UNKNOWN"]) {
    expect(apiErrorMessage(code, "original error")).toBe("original error");
  }
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
