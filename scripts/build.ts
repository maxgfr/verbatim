// Build the site into dist/: bundle the app, the speech worker and the
// service worker, copy the cut engine's wasm and ONNX Runtime's, and write
// the offline manifest. `--serve` then serves dist/ under /verbatim/ like
// GitHub Pages does. Run `bun run build:wasm` first (`bun run build` does).
import { createHash } from "node:crypto";
import { cp, mkdir, readdir, rm, stat } from "node:fs/promises";
import { dirname, join, relative } from "node:path";

const root = new URL("../", import.meta.url).pathname;
const dist = `${root}dist`;
const serve = process.argv.includes("--serve");
const BASE = "/verbatim/";

await rm(dist, { recursive: true, force: true });

function check(result: Awaited<ReturnType<typeof Bun.build>>) {
  if (!result.success) {
    for (const log of result.logs) console.error(log);
    process.exit(1);
  }
}

// transformers.js and onnxruntime-web require Node built-ins on their Node.js
// path only: resolve them to an empty module instead of bundling polyfills.
const noNodeBuiltins: Bun.BunPlugin = {
  name: "no-node-builtins",
  setup(build) {
    build.onResolve({ filter: /^(node:)?(fs|path|crypto|buffer|util|events|stream|url|os|child_process|worker_threads|module)$/ }, () => ({ path: `${root}scripts/node-stub.ts` }));
  },
};

check(
  await Bun.build({
    entrypoints: [`${root}index.html`],
    outdir: dist,
    minify: true,
    splitting: true,
    naming: { entry: "[name].[ext]", chunk: "assets/[name]-[hash].[ext]", asset: "assets/[name]-[hash].[ext]" },
    define: { "process.env.NODE_ENV": '"production"' },
  }),
);
check(
  await Bun.build({
    entrypoints: [`${root}src/worker.ts`],
    outdir: dist,
    minify: true,
    target: "browser",
    format: "esm",
    splitting: true,
    naming: { entry: "worker.js", chunk: "assets/w-[name]-[hash].[ext]", asset: "assets/[name]-[hash].[ext]" },
    define: { "process.env.NODE_ENV": '"production"' },
    plugins: [noNodeBuiltins],
  }),
);

// The cut and export engine (crates/edl through crates/wasm).
await mkdir(`${dist}/wasm`, { recursive: true });
await cp(`${root}src/edl/pkg/edl_bg.wasm`, `${dist}/wasm/edl.wasm`);

// ONNX Runtime for the speech models, served from here, never from a CDN:
// onnxruntime-web picks the JSEP or plain build (Parakeet, Canary) and
// transformers.js the asyncify one (Whisper on WebGPU); each loads only when used.
await mkdir(`${dist}/ort`, { recursive: true });
for (const name of ["ort-wasm-simd-threaded", "ort-wasm-simd-threaded.jsep", "ort-wasm-simd-threaded.asyncify"]) {
  for (const ext of ["mjs", "wasm"]) await cp(`${root}node_modules/onnxruntime-web/dist/${name}.${ext}`, `${dist}/ort/${name}.${ext}`);
}

await cp(`${root}public`, dist, { recursive: true });
// The font, as files beside its stylesheet (Bun's CSS bundler would inline them as data: URIs).
await mkdir(`${dist}/fonts`, { recursive: true });
const fontCss = await Bun.file(`${root}src/fonts.css`).text();
for (const m of fontCss.matchAll(/\.\/fonts\/([\w.-]+\.woff2)/g)) await cp(`${root}node_modules/@fontsource-variable/hanken-grotesk/files/${m[1]}`, `${dist}/fonts/${m[1]}`);
await Bun.write(`${dist}/fonts.css`, fontCss);
// Links to files under public/ are added after bundling: the HTML bundler
// would otherwise try to resolve and rename them.
const html = await Bun.file(`${dist}/index.html`).text();
await Bun.write(
  `${dist}/index.html`,
  html.replace(
    "</head>",
    '<link rel="stylesheet" href="./fonts.css"><link rel="icon" href="./icon.svg" type="image/svg+xml"><link rel="apple-touch-icon" href="./icon-192.png"><link rel="manifest" href="./manifest.webmanifest"></head>',
  ),
);
await Bun.write(`${dist}/.nojekyll`, "");

async function* walk(dir: string): AsyncGenerator<string> {
  for (const e of await readdir(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) yield* walk(p);
    else yield p;
  }
}

const files: string[] = [];
const hash = createHash("sha256");
for await (const p of walk(dist)) {
  const rel = relative(dist, p);
  if (rel === ".nojekyll") continue;
  files.push(rel);
  hash.update(rel).update(new Uint8Array(await Bun.file(p).arrayBuffer()));
}
files.sort();
// What the app needs to open, found by following static imports from the
// page and the worker; code loaded on demand (AI, PDF, the AI providers…) is
// cached only once used, like the rarer codecs.
async function staticGraph(entries: string[]): Promise<Set<string>> {
  const seen = new Set<string>();
  const queue = [...entries];
  while (queue.length) {
    const rel = queue.pop()!;
    if (seen.has(rel)) continue;
    seen.add(rel);
    if (!rel.endsWith(".js")) continue;
    const code = await Bun.file(join(dist, rel)).text();
    // `import{a}from"./x.js"` and `import"./x.js"`; never `import("./x.js")`.
    for (const m of code.matchAll(/(?:\bfrom|\bimport)\s*["'](\.{1,2}\/[^"']+\.js)["']/g)) {
      queue.push(relative(dist, join(dist, dirname(rel), m[1]!)));
    }
  }
  return seen;
}
const shell = await Bun.file(`${dist}/index.html`).text();
const pageEntries = [...shell.matchAll(/(?:src|href)="\.\/(assets\/[^"]+)"/g)].map((m) => m[1]!);
const eagerCode = await staticGraph([...pageEntries, "worker.js"]);
// The worker's big chunks (transformers.js, ONNX Runtime), the page's AI and
// rendering chunks and ONNX Runtime's wasm are cached the first time they are used.
const BIG = 150 * 1024;
const optional = async (f: string) => !eagerCode.has(f) && (!f.startsWith("assets/w-") || (await stat(`${dist}/${f}`)).size > BIG);
const lazy: string[] = [];
for (const f of files) {
  if (f.startsWith("ort/") || (f.startsWith("assets/") && f.endsWith(".js") && (await optional(f)))) lazy.push(f);
}
// Speech models are not the site's: llm-connect and transformers.js keep them in their own caches.
const precache = ["./", ...files.filter((f) => f !== "index.html" && !lazy.includes(f))];
const manifest = { version: hash.digest("hex").slice(0, 16), precache, lazy };

check(
  await Bun.build({
    entrypoints: [`${root}src/sw.ts`],
    outdir: dist,
    minify: true,
    target: "browser",
    naming: "sw.js",
    define: { __MANIFEST__: JSON.stringify(manifest) },
  }),
);

let total = 0;
for (const f of files) total += (await stat(`${dist}/${f}`)).size;
console.log(`site: ${files.length + 1} files, ${(total / 1048576).toFixed(1)} MiB, version ${manifest.version}`);

if (serve) {
  const port = Number(process.env.PORT ?? 4173);
  const types: Record<string, string> = { wasm: "application/wasm", webmanifest: "application/manifest+json", mjs: "text/javascript", onnx: "application/octet-stream" };
  Bun.serve({
    port,
    async fetch(req) {
      const url = new URL(req.url);
      if (!url.pathname.startsWith(BASE)) return Response.redirect(`${url.origin}${BASE}`, 302);
      let path = decodeURIComponent(url.pathname.slice(BASE.length)) || "index.html";
      if (path.endsWith("/")) path += "index.html";
      const file = Bun.file(join(dist, path));
      if (!(await file.exists())) return new Response("not found", { status: 404 });
      const ext = path.split(".").pop() ?? "";
      return new Response(file, types[ext] ? { headers: { "content-type": types[ext] } } : undefined);
    },
  });
  console.log(`serving http://localhost:${port}${BASE}`);
}
