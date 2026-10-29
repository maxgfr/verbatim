import { expect, type Page, type Route, test } from "@playwright/test";
import { seedDoc } from "./helpers.ts";

// The AI features through the real AI SDK and llm-connect, against a stubbed
// Anthropic API: the request never leaves the test, and its shape is checked
// (AI SDK 7 refuses system messages inside `messages`).

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("verbatim-lang", "en"));
});

const bodies: { system?: unknown; messages: { role: string; content: unknown }[] }[] = [];

function answer(text: string) {
  return async (route: Route) => {
    bodies.push(route.request().postDataJSON());
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        id: "msg_test",
        type: "message",
        role: "assistant",
        model: "claude-sonnet-5-5",
        content: [{ type: "text", text }],
        stop_reason: "end_turn",
        stop_sequence: null,
        usage: { input_tokens: 10, output_tokens: 20 },
      }),
    });
  };
}

async function setUpAnthropic(page: Page) {
  await page.goto("./#/ai");
  await page.locator(".fields select").first().selectOption({ label: "Anthropic" });
  await page.locator("input[type=password]").fill("test-key-not-real");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("status")).toContainText("Saved.");
}

test("summary through the AI SDK sends the instructions apart from the messages", async ({ page }) => {
  bodies.length = 0;
  await page.route("https://api.anthropic.com/**", answer("A short French clip about software prices.\n- Two points are made."));
  await page.goto("./");
  const id = await seedDoc(page);
  await setUpAnthropic(page);
  await page.goto(`./#/t/${id}`);
  await page.getByRole("tab", { name: "AI" }).click();
  await expect(page.locator(".ai-panel")).toContainText("goes to Anthropic");
  await page.getByRole("button", { name: "Summarize" }).click();
  await expect(page.locator(".liner-notes")).toContainText("software prices");
  expect(bodies).toHaveLength(1);
  expect(JSON.stringify(bodies[0]!.system)).toContain("summarize transcripts");
  expect(bodies[0]!.messages.every((m) => m.role === "user")).toBe(true);
  expect(JSON.stringify(bodies[0]!.messages)).toContain("Bonjour et bienvenue.");
});

test("an edit described by content is planned by the model, shown, then applied", async ({ page }) => {
  bodies.length = 0;
  await page.route(
    "https://api.anthropic.com/**",
    answer('Here is the plan:\n{"edits":[{"op":"delete","span":"between","start":1.84,"end":4.72}],"fillers":true,"note":"Cut the sentence about software prices and the hesitations."}'),
  );
  await page.goto("./");
  const id = await seedDoc(page);
  await setUpAnthropic(page);
  await page.goto(`./#/t/${id}`);
  await page.getByLabel("Say what to cut").fill("enlève le passage sur le prix");
  await page.getByRole("button", { name: "Plan" }).click();
  await expect(page.locator(".proposal")).toContainText("Cut the sentence about software prices");
  await expect(page.locator(".proposal .cuts li")).toHaveCount(2);
  await page.getByRole("button", { name: /Apply 2 cut/ }).click();
  await expect(page.locator(".cut-list .cuts li")).toHaveCount(2);
  expect(JSON.stringify(bodies[0]!.system)).toContain("video editing instruction");
  expect(JSON.stringify(bodies[0]!.messages)).toContain("Instruction: enlève le passage sur le prix");
});
