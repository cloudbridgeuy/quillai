// Usage: bun render.mjs [path to node_modules holding playwright-core]
// Defaults to the repo's app/node_modules. Writes one PNG per section.state into renders/.
const dir = import.meta.dir;
const root = process.argv[2] ?? dir + "/../../app/node_modules";
const { chromium } = await import(root + "/playwright-core/index.mjs");
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1164, height: 900 }, deviceScaleFactor: 1.5 });
await page.goto("file://" + dir + "/index.html");
await page.waitForTimeout(300);
const ids = await page.$$eval("section.state", els => els.map(e => e.id));
for (const id of ids) {
  await page.locator("#" + id).screenshot({ path: `${dir}/renders/${id}.png` });
  console.log(id);
}
await browser.close();
