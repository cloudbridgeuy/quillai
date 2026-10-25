import { expect, test, type APIRequestContext, type Page } from "../app/node_modules/@playwright/test/index.js";
import { existsSync } from "node:fs";
import { mkdtemp, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createServer } from "node:net";
import { expectEditorReady } from "./editor-ready";

const seeded = "The harbour was quiet before the restart.";
const typed = " It was still there after.";
const chapterText = `${seeded}${typed}`;

type FileSnapshot = { exists: boolean; mtimeMs: number; size: number };

type RunningApi = {
  url: string;
  stop: () => Promise<void>;
};

function workspaceRoot(): string {
  let dir = process.cwd();
  for (let i = 0; i < 4; i++) {
    if (existsSync(join(dir, "e2e/fixtures/ai-models.json")) && existsSync(join(dir, "xtask/src/e2e.rs"))) return dir;
    const parent = resolve(dir, "..");
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(`workspace root not found from ${process.cwd()}`);
}

function apiBinary(root: string): string {
  const target = process.env.CARGO_TARGET_DIR ? resolve(process.env.CARGO_TARGET_DIR) : join(root, "target");
  return join(target, "debug", "api");
}

async function snapshot(path: string): Promise<FileSnapshot> {
  try {
    const info = await stat(path);
    return { exists: true, mtimeMs: info.mtimeMs, size: info.size };
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return { exists: false, mtimeMs: 0, size: 0 };
    throw error;
  }
}

async function freePort(): Promise<number> {
  const server = createServer();
  await new Promise<void>((resolveListen, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolveListen());
  });
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("no free port");
  const port = address.port;
  await new Promise<void>((resolveClose, reject) => server.close(error => error ? reject(error) : resolveClose()));
  return port;
}

async function startApi(directory: string, appUrl: string): Promise<RunningApi> {
  const root = workspaceRoot();
  const binary = apiBinary(root);
  if (!existsSync(binary)) throw new Error(`API binary missing at ${binary}; run cargo xtask e2e so it can build`);
  const port = await freePort();
  const env: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (value !== undefined && key !== "QUILLAI_API_DB_URL") env[key] = value;
  }
  const stderrChunks: Uint8Array[] = [];
  const proc = Bun.spawn({
    cmd: [
      binary,
      "--host", "127.0.0.1",
      "--port", String(port),
      "--cors-origin", appUrl,
      "--log-level", "warn",
      "--ai-models", join(root, "e2e/fixtures/ai-models.json"),
    ],
    cwd: directory,
    env,
    stdin: "ignore",
    stdout: "ignore",
    stderr: "pipe",
  });
  const stderr = proc.stderr;
  const drained = (async () => {
    if (!stderr || typeof stderr === "number") return;
    const reader = stderr.getReader();
    try {
      for (;;) {
        const next = await reader.read();
        if (next.done) break;
        if (next.value) stderrChunks.push(next.value);
      }
    } catch {
      // The process was killed while stderr was still open.
    }
  })();
  const url = `http://127.0.0.1:${port}`;
  const stop = async () => {
    proc.kill(9);
    await proc.exited;
    await drained;
  };
  try {
    await waitReady(url, proc);
  } catch (error) {
    await stop();
    const detail = Buffer.concat(stderrChunks).toString();
    throw new Error(`${error instanceof Error ? error.message : error}\n${detail}`);
  }
  return { url, stop };
}

async function waitReady(url: string, proc: ReturnType<typeof Bun.spawn>): Promise<void> {
  const deadline = Date.now() + 20_000;
  let last = "no response";
  while (Date.now() < deadline) {
    if (proc.exitCode !== null) throw new Error(`API exited ${proc.exitCode} before it accepted requests`);
    try {
      const response = await fetch(`${url}/series`, { signal: AbortSignal.timeout(1_000) });
      if (response.ok) return;
      last = String(response.status);
    } catch (error) {
      last = error instanceof Error ? error.message : String(error);
    }
    await Bun.sleep(50);
  }
  throw new Error(`API not ready: ${last}`);
}

async function createChapter(request: APIRequestContext, apiUrl: string) {
  const seriesResponse = await request.post(`${apiUrl}/series`, { data: { title: "Restart series", position: 0 } });
  expect(seriesResponse.status()).toBe(201);
  const series = await seriesResponse.json();
  const bookResponse = await request.post(`${apiUrl}/series/${series.id}/books`, { data: { title: "Restart book", position: 0 } });
  expect(bookResponse.status()).toBe(201);
  const book = await bookResponse.json();
  const chaptersUrl = `${apiUrl}/series/${series.id}/books/${book.id}/chapters`;
  const chapterResponse = await request.post(chaptersUrl, { data: {
    title: "Restart chapter", position: 0, kind: "chapter",
    content: { ops: [{ insert: `${seeded}\n` }] },
  } });
  expect(chapterResponse.status()).toBe(201);
  const chapter = await chapterResponse.json();
  return {
    chapterUrl: `${chaptersUrl}/${chapter.id}`,
    query: new URLSearchParams({
      apiBaseUrl: apiUrl,
      seriesId: String(series.id),
      bookId: String(book.id),
      chapterId: String(chapter.id),
    }),
  };
}

async function openChapter(page: Page, query: URLSearchParams) {
  await page.setViewportSize({ width: 1100, height: 1000 });
  await page.goto(`/?${query}`);
  const editor = page.locator(".ql-editor");
  await expectEditorReady(editor);
  return editor;
}

function insertedText(content: { ops?: Array<{ insert?: unknown }> }): string {
  return (content.ops ?? []).map(op => typeof op.insert === "string" ? op.insert : "").join("");
}

test("chapter text survives an API restart on the default file database", async ({ page, request }) => {
  const appUrl = process.env.QUILLAI_E2E_APP_URL;
  if (!appUrl) throw new Error("Run this smoke with cargo xtask e2e");
  const root = workspaceRoot();
  const workspaceDb = join(root, "quillai.db");
  const before = await snapshot(workspaceDb);
  const directory = await mkdtemp(join(tmpdir(), "quillai-e2e-restart-"));
  if (directory.startsWith(root)) throw new Error(`refusing temp directory inside the workspace: ${directory}`);
  const database = join(directory, "quillai.db");
  let api: RunningApi | null = null;
  try {
    api = await startApi(directory, appUrl);
    expect((await stat(database)).isFile()).toBe(true);
    const created = await createChapter(request, api.url);
    const failures: string[] = [];
    let watched = api.url;
    page.on("pageerror", error => failures.push(error.message));
    page.on("response", response => {
      if (response.url().startsWith(watched) && response.status() >= 400) {
        failures.push(`${response.status()} ${response.request().method()} ${response.url()}`);
      }
    });
    const editor = await openChapter(page, created.query);
    await expect(editor).toHaveText(seeded);
    await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    await editor.evaluate((element) => {
      (element as HTMLElement).focus();
      const text = element.querySelector("p")?.firstChild;
      if (!text?.textContent) throw new Error("Missing chapter text");
      const range = document.createRange();
      range.setStart(text, text.textContent.length);
      range.collapse(true);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    });
    await page.keyboard.type(typed);
    await expect(editor).toHaveText(chapterText);
    await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    const saved = await request.get(created.chapterUrl);
    expect(saved.status()).toBe(200);
    expect(insertedText((await saved.json()).content)).toBe(`${chapterText}\n`);
    expect(failures).toEqual([]);

    const firstUrl = api.url;
    await api.stop();
    api = null;
    await expect.poll(async () => {
      try {
        await fetch(`${firstUrl}/series`, { signal: AbortSignal.timeout(500) });
        return "up";
      } catch {
        return "down";
      }
    }).toBe("down");

    api = await startApi(directory, appUrl);
    expect((await stat(database)).isFile()).toBe(true);
    watched = api.url;
    failures.length = 0;
    const reopenedUrl = created.chapterUrl.replace(firstUrl, api.url);
    const reopened = await request.get(reopenedUrl);
    expect(reopened.status()).toBe(200);
    expect(insertedText((await reopened.json()).content)).toBe(`${chapterText}\n`);
    const query = new URLSearchParams(created.query);
    query.set("apiBaseUrl", api.url);
    const restarted = await openChapter(page, query);
    await expect(restarted).toHaveText(chapterText);
    await expect(page.getByRole("status", { name: "Chapter save status", exact: true })).toHaveText("Saved");
    expect(failures).toEqual([]);
  } finally {
    try {
      if (api) await api.stop();
    } finally {
      await rm(directory, { recursive: true, force: true });
      expect(await snapshot(workspaceDb)).toEqual(before);
    }
  }
});
