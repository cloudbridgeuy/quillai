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
  test(`ui primitives render at ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(`http://127.0.0.1:${server.port}/`);

    for (const id of ["modal", "popover", "heading", "button", "icon-button", "list-button", "text-field", "toggle", "field", "alert", "sticky-note"]) {
      await expect(page.getByTestId(id)).toBeVisible();
    }
    await expect(page.getByRole("heading", { level: 1, name: "QuillAI app heading" })).toBeVisible();
    await expect(page.getByRole("heading", { level: 2, name: "Section heading" })).toBeVisible();

    await expect(page.getByRole("button", { name: "primary md", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "danger sm disabled", exact: true })).toBeDisabled();
    await expect(page.getByRole("button", { name: "Context", exact: true })).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByRole("button", { name: "Manuscript", exact: true })).toHaveAttribute("aria-pressed", "false");
    await expect(page.getByRole("button", { name: "Continue", exact: true })).toBeVisible();

    await expect(page.getByRole("button", { name: "Tree default", exact: true })).toHaveAttribute("type", "button");
    await expect(page.getByRole("button", { name: "Tree pressed", exact: true })).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByRole("button", { name: "Tree disabled", exact: true })).toBeDisabled();
    const lore = page.getByRole("button", { name: /Mira/ });
    await expect(lore).toHaveCSS("display", "grid");
    await expect(lore).toHaveCSS("border-top-width", "1px");

    await expect(page.getByLabel("Input", { exact: true })).toBeVisible();
    await expect(page.getByLabel("Disabled input")).toBeDisabled();
    await expect(page.getByRole("combobox", { name: "Select", exact: true })).toBeVisible();
    await expect(page.getByRole("checkbox", { name: "Checked", exact: true })).toBeChecked();
    await expect(page.getByRole("checkbox", { name: "Disabled", exact: true })).toBeDisabled();
    await expect(page.getByLabel("Title", { exact: true })).toBeVisible();
    await expect(page.getByText("Not saved", { exact: true })).toHaveAttribute("role", "alert");
    await expect(page.getByText("Could not load lore")).toBeVisible();

    await expect(page.getByLabel("Yellow note")).toHaveClass(/editor-sticky-note-yellow/);
    await expect(page.getByLabel("Pink note")).toHaveClass(/editor-sticky-note-pink/);
    await expect(page.getByLabel("Green note")).toHaveClass(/editor-sticky-note-green/);
    await expect(page.getByLabel("Overhang note")).toHaveClass(/editor-sticky-note-overhang/);
    await expect(page.getByLabel("Read-only note")).toHaveAttribute("readonly", "");
    await expect(page.getByLabel("Disabled note")).toBeDisabled();

    await page.screenshot({ fullPage: true, path: `../target/guz-874/${testInfo.project.name}/components-${width}.png` });
    await page.screenshot({ fullPage: true, path: `../target/guz-884/${testInfo.project.name}/components-${width}.png` });
  });
}

test("modal dismissal and focus", async ({ page }) => {
  await page.goto(`http://127.0.0.1:${server.port}/`);
  const opener = page.getByRole("button", { name: "Open modal" });
  const dialog = page.getByRole("dialog", { name: "Example modal" });
  await opener.click();
  await expect(dialog).toBeVisible();
  await dialog.click({ position: { x: 2, y: 2 } });
  await expect(dialog).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(opener).toBeFocused();
  await opener.click();
  await expect(dialog).toBeVisible();
  await page.mouse.click(1, 1);
  await expect(dialog).toHaveCount(0);
  await expect(opener).toBeFocused();
});

test("popover toggle and dismissal", async ({ page }) => {
  await page.goto(`http://127.0.0.1:${server.port}/`);
  const trigger = page.getByRole("button", { name: "Toggle popover" });
  const content = page.getByText("Popover content");
  await trigger.click();
  await expect(trigger).toHaveAttribute("aria-expanded", "true");
  await expect(content).toBeVisible();
  await trigger.click();
  await expect(content).toHaveCount(0);
  await trigger.click();
  await page.getByRole("heading", { name: "Popover", exact: true }).click();
  await expect(content).toHaveCount(0);
  await trigger.click();
  await page.keyboard.press("Escape");
  await expect(content).toHaveCount(0);
  await expect(trigger).toHaveAttribute("aria-expanded", "false");
});
