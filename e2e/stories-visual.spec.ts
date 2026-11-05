import { existsSync, readFileSync, statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "../app/node_modules/@playwright/test/index.js";

const root = fileURLToPath(new URL("../storybook/storybook-static", import.meta.url));
const built = existsSync(join(root, "index.json"));

const types: Record<string, string> = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".png": "image/png",
};

const ids: string[] = built
  ? Object.values(
      (JSON.parse(readFileSync(join(root, "index.json"), "utf8")) as { entries: Record<string, { id: string; type: string }> }).entries,
    )
      .filter((entry) => entry.type === "story")
      .map((entry) => entry.id)
  : [];

test.skip(!built, "storybook/storybook-static is missing; run `bun run build` in storybook/");
test.use({ viewport: { width: 1280, height: 720 } });

let server: Server;
let origin = "";

test.beforeAll(async () => {
  if (!built) return;
  server = createServer((request, response) => {
    const path = normalize(decodeURIComponent((request.url ?? "/").split("?")[0]!)).replace(/^(\.\.[/\\])+/, "");
    const file = join(root, path);
    if (!file.startsWith(root) || !existsSync(file) || !statSync(file).isFile()) {
      response.writeHead(404).end();
      return;
    }
    response.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
    response.end(readFileSync(file));
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  origin = `http://127.0.0.1:${typeof address === "object" && address ? address.port : 0}`;
});

test.afterAll(async () => {
  if (built) await new Promise((resolve) => server.close(resolve));
});

for (const id of ids) {
  test(`story ${id}`, async ({ page }) => {
    await page.goto(`${origin}/iframe.html?id=${encodeURIComponent(id)}&viewMode=story`);
    // Wait for render and play to finish; some stories end with an empty root (dismissed alert).
    await page.waitForFunction(() => (window as any).__STORYBOOK_PREVIEW__?.currentRender?.phase === "finished");
    await page.addStyleTag({ content: "*,*::before,*::after{animation:none!important;transition:none!important;caret-color:transparent!important}" });
    await expect(page).toHaveScreenshot(`${id}.png`, {
      animations: "disabled",
      caret: "hide",
      maxDiffPixelRatio: 0.01,
    });
  });
}
