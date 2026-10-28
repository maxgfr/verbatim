import { defineConfig, devices } from "@playwright/test";

// VERBATIM_URL runs the suite against a deployed site instead of a local build.
const remote = process.env.VERBATIM_URL;
export const baseURL = remote ?? "http://localhost:4173/verbatim/";

export default defineConfig({
  testDir: "e2e",
  timeout: 120_000,
  fullyParallel: true,
  workers: process.env.CI ? 2 : undefined,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["github"], ["list"]] : "list",
  use: { baseURL, acceptDownloads: true, trace: "retain-on-failure" },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: remote
    ? undefined
    : {
        command: "bun scripts/build.ts --serve",
        url: "http://localhost:4173/verbatim/",
        reuseExistingServer: !process.env.CI,
        timeout: 180_000,
      },
});
