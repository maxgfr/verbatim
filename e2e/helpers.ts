import { readFile } from "node:fs/promises";
import { expect, type Page } from "@playwright/test";

export const siteURL = process.env.VERBATIM_URL ?? "http://localhost:4173/verbatim/";
const siteOrigin = new URL(siteURL).origin;

export const fixture = (name: string) => new URL(`./fixtures/${name}`, import.meta.url).pathname;

/** Records every request leaving the page's origin, every CSP violation and every page error. */
export async function watch(page: Page) {
  const external: string[] = [];
  page.on("request", (r) => {
    const u = new URL(r.url());
    if (![siteOrigin, "null"].includes(u.origin) && !["blob:", "data:"].includes(u.protocol)) external.push(r.url());
  });
  await page.addInitScript(() => {
    (window as unknown as { __csp: string[] }).__csp = [];
    document.addEventListener("securitypolicyviolation", (e) => (window as unknown as { __csp: string[] }).__csp.push(`${e.violatedDirective} ${e.blockedURI}`));
  });
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  return {
    async assertClean() {
      expect(external, "requests to other origins").toEqual([]);
      expect(await page.evaluate(() => (window as unknown as { __csp: string[] }).__csp), "CSP violations").toEqual([]);
      expect(errors, "page errors").toEqual([]);
    },
  };
}

/** Put the CLI's transcript of the fixture into the history, as if the page had made it. */
export async function seedDoc(page: Page, id = "e2e-talk") {
  const exported = JSON.parse(await readFile(fixture("talk.json"), "utf8"));
  await page.evaluate(
    async ({ id, exported }) => {
      const db = await new Promise<IDBDatabase>((resolve, reject) => {
        const req = indexedDB.open("verbatim", 1);
        req.onupgradeneeded = () => req.result.createObjectStore("docs", { keyPath: "id" });
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
      });
      const now = Date.now();
      const doc = { id, name: "talk.mp4", mime: "video/mp4", size: 129581, createdAt: now, updatedAt: now, engine: exported.engine, transcript: exported.transcript };
      await new Promise((resolve, reject) => {
        const tx = db.transaction("docs", "readwrite");
        tx.objectStore("docs").put(doc);
        tx.oncomplete = resolve;
        tx.onerror = () => reject(tx.error);
      });
    },
    { id, exported },
  );
  return id;
}
