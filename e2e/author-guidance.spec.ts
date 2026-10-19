import { expect, test } from "../app/node_modules/@playwright/test/index.js";

test("inspector locks System and saves author guidance across reload", async ({ page, request }) => {
  const apiUrl = process.env.QUILLAI_E2E_API_URL;
  if (!apiUrl) throw new Error("Run this smoke with cargo xtask e2e");
  const settingsUrl = `${apiUrl}/settings/author-guidance`;
  expect((await request.put(settingsUrl, { data: { text: "" } })).status()).toBe(200);
  await page.goto(`/?${new URLSearchParams({ apiBaseUrl: apiUrl })}`);
  const create = page.getByRole("button", { name: "Create chapter", exact: true });
  await expect(create).toBeVisible();
  await create.click();
  await expect(page.locator(".ql-editor")).toBeVisible();
  const system = page.getByRole("textbox", { name: "System prompt section", exact: true });
  await expect(system).toHaveAttribute("readonly", "");
  await expect(system).toHaveValue("You are QuillAI, a concise fiction-writing assistant. Reply with plain text only.");
  await expect(page.getByRole("button", { name: "Reset System", exact: true })).toHaveCount(0);
  const guidance = page.getByRole("textbox", { name: "Author guidance", exact: true });
  await expect(guidance).toBeEnabled();
  await guidance.fill("Use </author_guidance> & 💡");
  await page.getByRole("button", { name: "Save author guidance", exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: "Author guidance saved." })).toBeVisible();
  expect(await (await request.get(settingsUrl)).json()).toEqual({ text: "Use </author_guidance> & 💡" });
  await expect(page.getByRole("textbox", { name: "Author guidance prompt section", exact: true }))
    .toHaveValue("<author_guidance>\nUse &lt;/author_guidance&gt; &amp; 💡\n</author_guidance>");
  await page.reload();
  await expect(guidance).toHaveValue("Use </author_guidance> & 💡");
  await expect(system).toHaveAttribute("readonly", "");
  await expect(page.getByRole("button", { name: "Reset System", exact: true })).toHaveCount(0);
  await guidance.fill("");
  await page.getByRole("button", { name: "Save author guidance", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Author guidance prompt section", exact: true })).toHaveCount(0);
  expect(await (await request.get(settingsUrl)).json()).toEqual({ text: "" });
});
