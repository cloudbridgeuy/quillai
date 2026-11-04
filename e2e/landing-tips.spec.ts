import { expect, test } from "../app/node_modules/@playwright/test/index.js";

test("tips dismissal survives reload", async ({ page }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api })}`);
  const tips = page.getByRole("region", { name: "Tips" });
  await expect(tips).toBeVisible();
  await tips.getByRole("button", { name: "Dismiss" }).click();
  await expect(tips).toHaveCount(0);
  await page.reload();
  await expect(tips).toHaveCount(0);
});
