import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import page_html from "./fixtures/ui-components/index.html";

let server: ReturnType<typeof Bun.serve>;

test.beforeAll(() => {
  server = Bun.serve({ port: 0, hostname: "127.0.0.1", routes: { "/": page_html } });
});

test.afterAll(async () => {
  await server.stop(true);
});

for (const width of [1100, 1440]) {
  test(`design styles remain unchanged at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(`http://127.0.0.1:${server.port}/`);
    await expect(page.getByLabel("Yellow note")).toBeVisible();
    await page.evaluate(() => {
      const drawer = document.createElement("div");
      drawer.className = "editor-drawer-left";
      drawer.innerHTML = '<div class="editor-lore-panel"></div>';
      document.body.append(drawer);
    });

    const styles = await page.evaluate(() => {
      const probe = document.createElement("div");
      document.body.append(probe);
      const expected = (property: string, value: string) => {
        probe.style.setProperty(property, value);
        return getComputedStyle(probe).getPropertyValue(property);
      };
      const sample = (selector: string, property: string) =>
        getComputedStyle(document.querySelector(selector)!).getPropertyValue(property);
      return {
        drawer: [sample(".editor-lore-panel", "background-image"), expected("background-image", "linear-gradient(rgb(255, 255, 255), rgb(255, 255, 255)), linear-gradient(transparent, rgba(31, 41, 51, .08))")],
        gradient: [sample('[aria-label="Yellow note"]', "background-image"), expected("background-image", "linear-gradient(rgba(31, 41, 51, .045), transparent 12px)")],
        yellow: [sample('[aria-label="Yellow note"]', "background-color"), expected("background-color", "#f4e8b5")],
        pink: [sample('[aria-label="Pink note"]', "background-color"), expected("background-color", "#f3d6d8")],
        green: [sample('[aria-label="Green note"]', "background-color"), expected("background-color", "#d7e6d0")],
        sheetShadow: [sample('[aria-label="Yellow note"]', "box-shadow"), expected("box-shadow", "0 1px 2px rgba(31, 41, 51, .06), 0 10px 34px rgba(31, 41, 51, .09)")],
        noteShadow: [sample('[aria-label="Pink note"]', "box-shadow"), expected("box-shadow", "0 1px 1px rgba(31, 41, 51, .06), 0 6px 14px -6px rgba(31, 41, 51, .22)")],
        stream: [sample("body", "--ai-stream-color").trim(), expected("color", "#4e6272")],
      };
    });
    for (const [actual, expected] of Object.values(styles)) expect(actual).toBe(expected);

    const note = page.getByLabel("Yellow note");
    await note.focus();
    const focusShadow = await note.evaluate((element) => {
      const probe = document.createElement("div");
      probe.style.boxShadow = "0 0 0 2px rgba(78, 98, 114, .35), 0 1px 2px rgba(31, 41, 51, .06), 0 10px 34px rgba(31, 41, 51, .09)";
      document.body.append(probe);
      return [getComputedStyle(element).boxShadow, getComputedStyle(probe).boxShadow];
    });
    expect(focusShadow[0]).toBe(focusShadow[1]);

    await page.getByRole("button", { name: "Open modal" }).click();
    const backdrop = await page.getByRole("dialog", { name: "Example modal" }).evaluate((dialog) => {
      const probe = document.createElement("div");
      probe.style.backgroundColor = "color-mix(in oklab, #1f2933 50%, transparent)";
      document.body.append(probe);
      return [getComputedStyle(dialog, "::backdrop").backgroundColor, getComputedStyle(probe).backgroundColor];
    });
    expect(backdrop[0]).toBe(backdrop[1]);
  });
}
