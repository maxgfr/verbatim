// The model catalog for the AI features with your own key: built by
// llm-models, then cut down to the providers whose API hosts the page's CSP
// allows (index.html connect-src).
import { $ } from "bun";

export const PROVIDERS = ["anthropic", "openai", "google", "mistral", "groq", "openrouter", "xai"] as const;

const out = new URL("../public/models.json", import.meta.url).pathname;
await $`bunx llm-models catalog --npm callable --require ${PROVIDERS.slice(0, 4).join(",")} --out ${out}`.quiet();
const all = (await Bun.file(out).json()) as Record<string, unknown>;
const kept = Object.fromEntries(PROVIDERS.filter((id) => all[id]).map((id) => [id, all[id]]));
await Bun.write(out, JSON.stringify(kept));
console.log(`models.json: ${Object.keys(kept).length} providers`);
