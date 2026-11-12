import { expect, test } from "../app/node_modules/@playwright/test/index.js";

test("stale saved ids return home and + Universe creates a universe", async ({ page }) => {
  const api = process.env.QUILLAI_E2E_API_URL;
  if (!api) throw new Error("Run with cargo xtask e2e");
  const chapter = { scope: { seriesId: 4, bookId: 5, chapterId: 6 }, title: "Gone chapter" };
  const piece = { scope: { universeId: 9, pieceId: 9 }, title: "Gone piece" };
  await page.addInitScript(({ recent, lastEdited }) => {
    localStorage.setItem("quillai.landing.recent.v1", recent);
    localStorage.setItem("quillai.landing.lastEdited.v1", lastEdited);
  }, { recent: JSON.stringify([chapter, piece]), lastEdited: JSON.stringify(chapter) });
  await page.setViewportSize({ width: 1100, height: 1000 });
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api, seriesId: "4", bookId: "5", chapterId: "6" })}`);
  await expect(page).not.toHaveURL(/seriesId=/);
  const resume = page.getByRole("region", { name: "Resume", exact: true });
  await expect(resume).toContainText("Nothing edited in this browser yet");
  await expect(resume.getByText("Could not check")).toHaveCount(0);
  const library = page.getByRole("region", { name: "Library", exact: true });
  await library.getByRole("button", { name: "+ Universe", exact: true }).click();
  const title = library.getByRole("textbox", { name: "Universe title", exact: true });
  await title.fill("Recovered universe");
  await title.press("Enter");
  await expect(library.locator("summary").filter({ hasText: "Recovered universe" })).toBeVisible();
  await expect.poll(() => page.evaluate(() => localStorage.getItem("quillai.landing.recent.v1"))).toBe("[]");
  await expect.poll(() => page.evaluate(() => localStorage.getItem("quillai.landing.lastEdited.v1"))).toBeNull();
});
