import { expect, test } from "../app/node_modules/@playwright/test/index.js";

for (const width of [1100, 1440]) {
  test(`recently deleted items can be restored and permanently deleted at ${width}px`, async ({ page, request }, testInfo) => {
    const api = process.env.QUILLAI_E2E_API_URL;
    if (!api) throw new Error("Run with cargo xtask e2e");
    const prefix = `Trash ${testInfo.project.name} ${width}`;
    const name = (title: string) => `${prefix} ${title}`;
    const post = async (url: string, data: object) => {
      const response = await request.post(url, { data });
      expect(response.status(), await response.text()).toBe(201);
      return response.json();
    };
    const remove = async (url: string) => expect((await request.delete(url)).status()).toBe(204);
    const series = await post(`${api}/series`, { title: name("home"), position: 0 });
    const seriesUrl = `${api}/series/${series.id}`;
    const book = await post(`${seriesUrl}/books`, { title: name("Active book"), position: 0 });
    const bookUrl = `${seriesUrl}/books/${book.id}`;
    const chapter = await post(`${bookUrl}/chapters`, { title: name("Restore chapter"), position: 0, kind: "chapter" });
    await post(`${bookUrl}/chapters`, { title: name("Anchor chapter"), position: 1, kind: "chapter" });
    const lore = await post(`${seriesUrl}/lore`, { kind: "character", name: name("Deleted lore") });
    const container = await (await request.get(`${api}/containers/${series.id}`)).json();
    const universeUrl = `${api}/universes/${container.universeId}`;
    const piece = await post(`${universeUrl}/pieces`, { title: name("Deleted piece"), position: 0, label: "short_story", shape: "leaf" });
    const collection = await post(`${universeUrl}/containers`, { title: name("Deleted collection"), position: 1, kind: "collection" });
    const deletedBook = await post(`${seriesUrl}/books`, { title: name("Deleted book"), position: 1 });
    const deletedSeries = await post(`${api}/series`, { title: name("Deleted series"), position: 1 });
    const deletedUniverse = await post(`${api}/universes`, { title: name("Deleted universe"), position: 2 });
    const warningBook = await post(`${seriesUrl}/books`, { title: name("Deleted parent book"), position: 2 });
    const warningBookUrl = `${seriesUrl}/books/${warningBook.id}`;
    const warningChapter = await post(`${warningBookUrl}/chapters`, { title: name("Deleted child chapter"), position: 0, kind: "chapter" });
    const largeBook = await post(`${seriesUrl}/books`, { title: name("Large deleted book"), position: 3 });
    const largeBookUrl = `${seriesUrl}/books/${largeBook.id}`;
    for (let position = 0; position < 41; position++) {
      await post(`${largeBookUrl}/chapters`, { title: `Chapter ${position + 1}`, position, kind: "chapter" });
    }
    for (const url of [
      `${bookUrl}/chapters/${chapter.id}`, `${seriesUrl}/lore/${lore.id}`, `${api}/pieces/${piece.id}`,
      `${api}/containers/${collection.id}`, `${seriesUrl}/books/${deletedBook.id}`, `${api}/series/${deletedSeries.id}`,
      `${api}/universes/${deletedUniverse.id}`, `${warningBookUrl}/chapters/${warningChapter.id}`, warningBookUrl, largeBookUrl,
    ]) await remove(url);

    await page.setViewportSize({ width, height: 1000 });
    await page.goto(`/?${new URLSearchParams({ apiBaseUrl: api })}`);
    const library = page.getByRole("region", { name: "Library", exact: true });
    const home = library.locator("li").filter({ has: page.getByText(name("home"), { exact: true }) }).last();
    const openHome = async () => {
      const details = home.locator(":scope > details");
      if (await details.getAttribute("open") === null) await details.locator(":scope > summary").click();
    };
    const trash = page.getByRole("region", { name: "Recently deleted", exact: true });
    const row = (name: string) => trash.getByRole("listitem").filter({ has: page.getByText(name, { exact: true }) });
    for (const [itemName, type] of [
      [name("Deleted lore"), "Lore"], [name("Restore chapter"), "Chapter"], [name("Deleted piece"), "Piece"],
      [name("Deleted book"), "Book"], [name("Deleted series"), "Series"], [name("Deleted collection"), "Collection"],
      [name("Deleted universe"), "Universe"],
    ]) {
      const item = row(itemName);
      await expect(item).toBeVisible();
      await expect(item.getByText(type, { exact: true })).toBeVisible();
      await expect(item.locator("time[datetime]")).toBeVisible();
      await expect(item.locator("time")).not.toBeEmpty();
    }
    await row(name("Restore chapter")).getByRole("button", { name: "Restore", exact: true }).click();
    await expect(row(name("Restore chapter"))).toHaveCount(0);
    await openHome();
    await expect(home.getByRole("button", { name: name("Restore chapter"), exact: true })).toBeVisible();

    const child = row(name("Deleted child chapter"));
    await expect(child).toBeVisible();
    await child.getByRole("button", { name: "Restore", exact: true }).click();
    await expect(child.getByRole("status")).toContainText(name("Deleted parent book"));
    expect((await request.get(`${warningBookUrl}/chapters/${warningChapter.id}`)).status()).toBe(404);
    await child.getByRole("button", { name: "Confirm restore", exact: true }).click();
    await expect(child).toHaveCount(0);
    await openHome();
    await expect(home.getByRole("button", { name: name("Deleted child chapter"), exact: true })).toBeVisible();

    const large = row(name("Large deleted book"));
    await large.getByRole("button", { name: "Delete forever", exact: true }).click();
    await expect(large.getByRole("button", { name: "Confirm delete", exact: true })).toBeVisible();
    await expect(large).toContainText(/41\s+chapters/i);
    await expect(large).toBeVisible();
    await large.getByRole("button", { name: "Confirm delete", exact: true }).click();
    await expect(large).toHaveCount(0);
    expect((await request.post(`${largeBookUrl}/restore`)).status()).toBe(404);
  });
}
