import { readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, test } from "@playwright/test";
import { seedDoc, watch } from "./helpers.ts";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("verbatim-lang", "en"));
});

async function openSeeded(page: import("@playwright/test").Page) {
  await page.goto("./");
  const id = await seedDoc(page);
  await page.goto(`./#/t/${id}`);
  await expect(page.locator(".tracklist .track")).toHaveCount(6);
  return id;
}

test("select words, press Delete to cut them, undo with the keyboard", async ({ page }) => {
  const w = await watch(page);
  await openSeeded(page);
  // Select "nous allons parler" in segment 3.
  await page.evaluate(() => {
    const words = [...document.querySelectorAll<HTMLElement>('li[data-s="2"] .w')];
    const from = words.find((x) => x.textContent?.includes("nous"))!;
    const to = words.find((x) => x.textContent?.includes("parler"))!;
    const r = document.createRange();
    r.setStart(from.firstChild!.nextSibling ?? from.firstChild!, 0);
    r.setEnd(to.lastChild!, (to.lastChild as Text).length);
    getSelection()!.removeAllRanges();
    getSelection()!.addRange(r);
  });
  await expect(page.getByRole("button", { name: "Cut selection" })).toBeEnabled();
  await page.keyboard.press("Delete");
  await expect(page.locator(".cut-list .cuts li")).toHaveCount(1);
  await expect(page.locator('li[data-s="2"] .w.cut')).toHaveCount(3);
  await page.keyboard.press("ControlOrMeta+z");
  await expect(page.locator(".cut-list .cuts li")).toHaveCount(0);
  await w.assertClean();
});

test("a cut list leaves and comes back, in the CLI's format", async ({ page }) => {
  const w = await watch(page);
  await openSeeded(page);
  await page.getByRole("button", { name: "Cut segment 2" }).click({ force: true });
  await page.getByRole("button", { name: "Cut segment 6" }).click({ force: true });
  await page.locator(".handoff summary").click();
  const [d] = await Promise.all([page.waitForEvent("download"), page.getByRole("button", { name: "Download the cut list" }).click()]);
  const list = JSON.parse(await readFile(await d.path(), "utf8"));
  expect(list.version).toBe(1);
  expect(list.source).toBe("talk.mp4");
  expect(list.delete).toHaveLength(2);
  await expect(page.locator(".command code")).toContainText(/--delete 0:01\.\d+-0:01\.\d+ --delete 0:09\.\d+-0:1\d/);

  for (let i = 0; i < 2; i++) await page.locator(".cut-list .cuts li .icon-button").first().click();
  await expect(page.locator(".cut-list .cuts li")).toHaveCount(0);
  const file = join(tmpdir(), "verbatim-e2e-cuts.json");
  await writeFile(file, JSON.stringify(list));
  await page.locator(".handoff input[type=file]").setInputFiles(file);
  await expect(page.locator(".cut-list .cuts li")).toHaveCount(2);
  await w.assertClean();
});

test("saying what to cut asks for an AI provider when none is set up", async ({ page }) => {
  await openSeeded(page);
  await page.getByLabel("Say what to cut").fill("enlève le passage sur le prix");
  await page.getByRole("button", { name: "Plan" }).click();
  await expect(page.locator(".montage [role=status]")).toContainText("needs an AI provider");
  await expect(page.locator(".proposal")).toHaveCount(0);
});

test("history lists, opens and deletes transcripts", async ({ page }) => {
  const w = await watch(page);
  await openSeeded(page);
  await page.getByRole("link", { name: "History" }).click();
  await expect(page.locator(".history li")).toHaveCount(1);
  await expect(page.locator(".history li")).toContainText("Parakeet TDT 0.6B v3");
  await page.getByRole("link", { name: "talk.mp4" }).click();
  await expect(page.getByRole("heading", { name: "talk.mp4" })).toBeVisible();
  await page.goBack();
  await page.getByRole("button", { name: "Delete talk.mp4" }).click();
  await expect(page.getByText("Nothing transcribed yet.")).toBeVisible();
  await w.assertClean();
});

test("a model on this machine can be set up without any request leaving the page", async ({ page }) => {
  const w = await watch(page);
  await openSeeded(page);
  await page.getByRole("link", { name: "AI", exact: true }).click();
  await page.getByRole("button", { name: "On my machine" }).click();
  await page.getByRole("button", { name: "Ollama" }).click();
  await expect(page.getByLabel("Server address")).toHaveValue("http://localhost:11434/v1");
  await page.getByLabel("Model", { exact: true }).fill("llama3.2");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("status")).toContainText("Saved.");
  await page.goBack();
  await page.getByRole("tab", { name: "AI" }).click();
  await expect(page.locator(".ai-panel")).toContainText("goes to http://localhost:11434/v1 (llama3.2)");
  await expect(page.getByRole("button", { name: "Summarize" })).toBeEnabled();
  await w.assertClean();
});

test("the app opens offline once visited", async ({ page, context }) => {
  await page.goto("./");
  await page.evaluate(() => navigator.serviceWorker.ready);
  await page.reload();
  await expect(page.locator(".sleeve h1")).toBeVisible();
  await context.setOffline(true);
  await page.reload();
  await expect(page.locator(".sleeve h1")).toBeVisible();
  await expect(page.locator(".liner-cost")).toContainText("Parakeet");
  await context.setOffline(false);
});
