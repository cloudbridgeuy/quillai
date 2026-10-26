import { expect, test } from "../app/node_modules/@playwright/test/index.js";

test("e2e frontend serves Tailwind token styles at 1100px", async ({ page }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  await page.setViewportSize({ width: 1100, height: 900 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api })}`);
  await page.getByRole("button", { name: "Manuscript", exact: true }).click();
  const outline = page.getByRole("region", { name: "Manuscript outline", exact: true });
  await outline.getByRole("button", { name: "+ Universe", exact: true }).click();
  const create = outline.getByRole("button", { name: "Create", exact: true });
  await expect(create).toBeVisible();
  await expect(create).toHaveCSS("background-color", "rgb(78, 98, 114)");
  await expect(create).toHaveCSS("min-height", "36px");
});
