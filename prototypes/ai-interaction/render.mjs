const root = process.argv[2];
const { chromium } = await import(root + "/playwright-core/index.mjs");
const dir = import.meta.dir;
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1180, height: 900 }, deviceScaleFactor: 2 });
await page.goto("file://" + dir + "/index.html");
await page.waitForTimeout(400);
const ids = await page.$$eval("section.state", els => els.map(e => e.id));
for (const id of ids) {
  await page.locator("#" + id).screenshot({ path: `${dir}/renders/${id}.png` });
  console.log(id);
}
await page.screenshot({ path: `${dir}/renders/00-full.png`, fullPage: true });
await browser.close();
