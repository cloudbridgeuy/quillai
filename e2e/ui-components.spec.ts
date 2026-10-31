import { expect, test } from "../app/node_modules/@playwright/test/index.js";
import page_html from "./fixtures/ui-components/index.html";
import behaviors_html from "./fixtures/ui-components/behaviors.html";

let server: ReturnType<typeof Bun.serve>;

test.beforeAll(() => {
  server = Bun.serve({ port: 0, hostname: "127.0.0.1", routes: { "/": page_html, "/behaviors": behaviors_html } });
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

for (const opening of ["mouse", "keyboard"] as const) {
  test(`popover returns focus after ${opening} opening and Escape`, async ({ page }) => {
    await page.goto(`http://127.0.0.1:${server.port}/behaviors`);
    const trigger = page.getByRole("button", { name: "Open popover" });
    if (opening === "mouse") {
      await trigger.click();
      await page.getByRole("textbox", { name: "Popover field" }).click();
    } else {
      await page.keyboard.press("Tab");
      await trigger.focus();
      await expect(trigger).toBeFocused();
      await page.keyboard.press("Enter");
      await page.getByRole("textbox", { name: "Popover field" }).focus();
    }
    const field = page.getByRole("textbox", { name: "Popover field" });
    await expect(field).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(field).toHaveCount(0);
    await expect(trigger).toBeFocused();
  });
}

test("button has a visible keyboard focus indicator", async ({ page }) => {
  await page.goto(`http://127.0.0.1:${server.port}/behaviors`);
  await page.keyboard.press("Tab");
  const button = page.getByRole("button", { name: "Focus button" });
  await button.focus();
  await expect(button).toBeFocused();
  const visible = await button.evaluate((element) => {
    const style = getComputedStyle(element);
    return (!["none", "auto"].includes(style.outlineStyle) && parseFloat(style.outlineWidth) > 0 && style.outlineColor !== "rgba(0, 0, 0, 0)") || style.boxShadow !== "none";
  });
  expect(visible).toBe(true);
});

test("outside click keeps focus on the clicked field", async ({ page }) => {
  await page.goto(`http://127.0.0.1:${server.port}/behaviors`);
  await page.getByRole("button", { name: "Open popover" }).click();
  const field = page.getByRole("textbox", { name: "Outside field" });
  await field.click();
  await expect(page.getByRole("textbox", { name: "Popover field" })).toHaveCount(0);
  await page.keyboard.type("Outside focus");
  await expect(field).toHaveValue("Outside focus");
  await expect(field).toBeFocused();
});

test("icon and list buttons invoke their click handlers", async ({ page }) => {
  await page.goto(`http://127.0.0.1:${server.port}/behaviors`);
  await page.getByRole("button", { name: "Count icon" }).click();
  await page.getByRole("button", { name: "Count icon" }).click();
  await page.getByRole("button", { name: "Count list" }).click();
  await expect(page.getByTestId("icon-count")).toHaveText("2");
  await expect(page.getByTestId("list-count")).toHaveText("1");
});
