import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

async function check(files: Record<string, string>) {
  const dir = await mkdtemp(join(tmpdir(), "nonet-"));
  for (const [name, body] of Object.entries(files)) await Bun.write(join(dir, name), body);
  const p = Bun.spawnSync(["bun", new URL("./check-no-network.ts", import.meta.url).pathname, dir]);
  await rm(dir, { recursive: true, force: true });
  return { code: p.exitCode, err: p.stderr.toString() };
}

test("passes clean bundles and allowlisted hosts", async () => {
  expect((await check({ "a.js": 'const x = "https://react.dev/errors/1"; fetch("./wasm/core.wasm")' })).code).toBe(0);
});

test("flags unknown hosts and literal network calls", async () => {
  const r = await check({ "a.js": 'fetch("https://tracker.example/collect")' });
  expect(r.code).toBe(1);
  expect(r.err).toContain("tracker.example");
  expect((await check({ "a.js": 'fetch("https://github.com/x")' })).err).toContain("network call to a literal URL");
});
