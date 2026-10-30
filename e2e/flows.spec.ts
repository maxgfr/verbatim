import { readFile } from "node:fs/promises";
import { expect, test } from "@playwright/test";
import { fixture, seedDoc, watch } from "./helpers.ts";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("verbatim-lang", "en"));
});

test("cut segments and hesitations, render the edit and export subtitles, all in the page", async ({ page }) => {
  const w = await watch(page);
  await page.goto("./");
  const id = await seedDoc(page);
  await page.goto(`./#/t/${id}`);
  await expect(page.locator(".tracklist .track")).toHaveCount(6);
  await expect(page.locator(".tracklist")).toContainText("Bonjour et bienvenue.");

  // The media is not stored: open it again to play and render.
  await page.locator(".reattach input[type=file]").setInputFiles(fixture("talk.mp4"));
  await expect(page.locator(".deck-meta")).toContainText("0:14");

  // Cut "Le premier point est simple." (segment 4) and the last segment, by their scissors.
  await page.getByRole("button", { name: "Cut segment 4" }).click({ force: true });
  await page.getByRole("button", { name: "Cut segment 6" }).click({ force: true });
  await page.getByRole("button", { name: /Remove hesitations/ }).click();
  await expect(page.locator(".cut-list .cuts li")).toHaveCount(3);
  await expect(page.locator(".w.cut").first()).toBeVisible();

  const [subs] = await Promise.all([page.waitForEvent("download"), page.getByRole("button", { name: "Subtitles of the edit" }).click()]);
  const srt = await readFile(await subs.path(), "utf8");
  expect(subs.suggestedFilename()).toBe("talk.edit.srt");
  expect(srt).toContain("Bonjour et bienvenue.");
  expect(srt).not.toContain("Euh.");

  const [video] = await Promise.all([page.waitForEvent("download", { timeout: 60_000 }), page.getByRole("button", { name: "Export the edited file" }).click()]);
  expect(video.suggestedFilename()).toBe("talk.edit.mp4");
  expect((await readFile(await video.path())).byteLength).toBeGreaterThan(10_000);

  await page.locator(".handoff summary").click();
  await expect(page.locator(".command code")).toContainText("verbatim cut talk.mp4 --delete");

  // Undo takes the last cut back.
  await page.getByRole("button", { name: "Undo" }).click();
  await expect(page.locator(".cut-list .cuts li")).toHaveCount(2);
  await w.assertClean();
});

test("every export format downloads, and the transcript can be corrected", async ({ page }) => {
  const w = await watch(page);
  await page.goto("./");
  const id = await seedDoc(page);
  await page.goto(`./#/t/${id}`);
  await page.getByRole("button", { name: "Correct segment 5" }).click({ force: true });
  const box = page.getByLabel("Text of segment 5");
  await box.fill("Le deuxième point, euh, est plus difficile.");
  await box.press("Enter");
  await expect(page.locator('li[data-s="4"]')).toContainText("euh, est plus");

  await page.getByRole("tab", { name: "Export" }).click();
  for (const [label, name, needle] of [
    ["SRT", "talk.srt", "00:00:00,000 -->"],
    ["VTT", "talk.vtt", "WEBVTT"],
    ["Markdown", "talk.md", "# talk.mp4"],
    ["CSV", "talk.csv", "index,start,end,text"],
    ["JSON", "talk.json", '"transcript"'],
  ] as const) {
    const [d] = await Promise.all([page.waitForEvent("download"), page.getByRole("button", { name: label, exact: true }).click()]);
    expect(d.suggestedFilename()).toBe(name);
    const body = await readFile(await d.path(), "utf8");
    expect(body).toContain(needle);
    if (label === "SRT") expect(body).toContain("euh, est plus difficile");
  }
  await w.assertClean();
});
