// The CSP sets `require-trusted-types-for 'script'`: script URLs must come
// from a policy. This one only ever vouches for our own worker and service
// worker, at this origin.
interface TrustedTypePolicyLike {
  createScriptURL(input: string): string;
}
interface TrustedTypesLike {
  createPolicy(name: string, rules: { createScriptURL: (s: string) => string }): TrustedTypePolicyLike;
}

const ALLOWED = new Set(["worker.js", "sw.js"]);

function check(url: string): string {
  const u = new URL(url, location.href);
  const file = u.pathname.split("/").pop() ?? "";
  if (u.origin !== location.origin || !ALLOWED.has(file)) throw new TypeError(`blocked script URL ${url}`);
  return u.href;
}

const tt = (globalThis as { trustedTypes?: TrustedTypesLike }).trustedTypes;
const policy: TrustedTypePolicyLike = tt ? tt.createPolicy("verbatim", { createScriptURL: check }) : { createScriptURL: check };

/** A script URL the CSP will accept, for `new Worker` and service worker registration. */
export const scriptURL = (path: string): string => policy.createScriptURL(path);
