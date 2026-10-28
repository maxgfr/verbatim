/// <reference lib="webworker" />
// Offline support. The build replaces __MANIFEST__ with the hashed list of
// files: the app shell is cached at install; ONNX Runtime and the big
// optional chunks the first time they are used. Speech models are not handled
// here: llm-connect and transformers.js keep them in their own caches.

const sw = self as unknown as ServiceWorkerGlobalScope;
declare const __MANIFEST__: { version: string; precache: string[]; lazy: string[] };

const { version, precache, lazy } = __MANIFEST__;
const CACHE = `verbatim-${version}`;
const scope = new URL(sw.registration.scope);
const abs = (p: string) => new URL(p, scope).href;
const lazySet = new Set(lazy.map(abs));

sw.addEventListener("install", (event: ExtendableEvent) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((c) => c.addAll(precache.map(abs)))
      .then(() => sw.skipWaiting()),
  );
});

// The previous version's files stay cached: a tab still open on it keeps
// finding its own code (hashed names the new deploy no longer has) until it
// reloads. Older versions are deleted.
const META = "verbatim-meta";
const KEEP_VERSIONS = 2;

sw.addEventListener("activate", (event: ExtendableEvent) => {
  event.waitUntil(
    (async () => {
      const meta = await caches.open(META);
      const res = await meta.match(abs("./__versions"));
      const versions = ((res ? await res.json() : []) as string[]).filter((v) => v !== CACHE);
      versions.push(CACHE);
      const keep = new Set([...versions.slice(-KEEP_VERSIONS), META]);
      await meta.put(abs("./__versions"), new Response(JSON.stringify(versions.slice(-KEEP_VERSIONS))));
      const keys = await caches.keys();
      await Promise.all(keys.filter((k) => k.startsWith("verbatim-") && !keep.has(k)).map((k) => caches.delete(k)));
      await sw.clients.claim();
    })(),
  );
});

sw.addEventListener("fetch", (event: FetchEvent) => {
  const req = event.request;
  const url = new URL(req.url);
  if (req.method !== "GET" || url.origin !== scope.origin || !url.pathname.startsWith(scope.pathname)) return;
  // Navigations always get the app shell: routing lives in the hash.
  const key = req.mode === "navigate" ? abs("./") : url.origin + url.pathname;
  event.respondWith(
    (async () => {
      const cache = await caches.open(CACHE);
      // This version first, then the previous one (for tabs still running it).
      const hit = (await cache.match(key)) ?? (req.mode === "navigate" ? undefined : await caches.match(key));
      if (hit) return hit;
      const res = await fetch(req);
      if (res.ok && (lazySet.has(key) || key === abs("./"))) await cache.put(key, res.clone());
      return res;
    })(),
  );
});
