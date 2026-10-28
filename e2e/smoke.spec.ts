import { expect, test } from "@playwright/test";
import { watch } from "./helpers.ts";

test("the home page states the cost and makes no request elsewhere", async ({ page }) => {
  const w = await watch(page);
  await page.goto("./");
  await expect(page.getByRole("heading", { name: /Drop a video or audio file|Déposez une vidéo/ })).toBeVisible();
  await expect(page.locator(".liner-cost")).toContainText(/Parakeet TDT 0\.6B v3/);
  await expect(page.locator(".liner-cost")).toContainText(/670/);
  await page.getByRole("link", { name: /Privacy|Confidentialité/ }).click();
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await page.getByRole("button", { name: "FR" }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "fr");
  await w.assertClean();
});
