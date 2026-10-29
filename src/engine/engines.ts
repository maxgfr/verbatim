// Which speech model runs. "Auto" leaves detection to Parakeet, sends a named
// English, French, German or Spanish to Canary (told the language), other
// Parakeet languages to Parakeet, the rest to Whisper; every choice shows its
// download before anything is fetched.

import { NEMO_MODELS, nemoSupports } from "llm-connect/nemo";
import { WHISPER_PRESETS, type WhisperPreset } from "llm-connect/whisper";

export type EngineId = "auto" | "parakeet" | "canary" | "whisper-fast" | "whisper-balanced" | "whisper-accurate" | "cloud";

/** What the worker runs (or the cloud). */
export type Engine = { kind: "nemo"; model: "parakeet" | "canary" } | { kind: "whisper"; preset: WhisperPreset } | { kind: "cloud" };

export const ENGINE_IDS: EngineId[] = ["auto", "parakeet", "canary", "whisper-fast", "whisper-balanced", "whisper-accurate", "cloud"];

/** Languages offered in the picker: Parakeet's 25, which Whisper also knows. "auto" leaves it to the model. */
export const LANGUAGES = ["auto", ...NEMO_MODELS.parakeet.languages] as const;

export function resolveEngine(id: EngineId, language: string): Engine {
  switch (id) {
    case "auto":
      // Parakeet finds the language itself but cannot be told it, and takes accented
      // speech for English (a French talk came out as English). A language the user
      // names goes to Canary, which is told it, when Canary knows it.
      if (language === "auto") return { kind: "nemo", model: "parakeet" };
      if (nemoSupports("canary", language)) return { kind: "nemo", model: "canary" };
      return nemoSupports("parakeet", language) ? { kind: "nemo", model: "parakeet" } : { kind: "whisper", preset: "balanced" };
    case "parakeet":
    case "canary":
      return { kind: "nemo", model: id };
    case "cloud":
      return { kind: "cloud" };
    default:
      return { kind: "whisper", preset: id.slice("whisper-".length) as WhisperPreset };
  }
}

/** Download in MB, or 0 for the cloud. */
export function engineSize(e: Engine): number {
  if (e.kind === "nemo") return NEMO_MODELS[e.model].size;
  if (e.kind === "whisper") return WHISPER_PRESETS[e.preset].size;
  return 0;
}

export function engineLabel(e: Engine): string {
  if (e.kind === "nemo") return e.model === "parakeet" ? "Parakeet TDT 0.6B v3" : "Canary 180M Flash";
  if (e.kind === "whisper") return `Whisper (${e.preset})`;
  return "Cloud";
}

/** Why this engine cannot take this language, or null. */
export function engineProblem(e: Engine, language: string): "canary-language" | "whisper-auto" | null {
  if (e.kind === "nemo" && e.model === "canary" && (language === "auto" || !nemoSupports("canary", language))) return "canary-language";
  if (e.kind === "whisper" && language === "auto") return "whisper-auto";
  return null;
}

/** The model files a worker may already hold in Cache Storage: llm-connect keeps NeMo there, transformers.js keeps Whisper. */
export async function cachedEngines(): Promise<Set<string>> {
  const out = new Set<string>();
  try {
    for (const name of await caches.keys()) {
      const cache = await caches.open(name);
      for (const req of await cache.keys()) {
        const u = req.url;
        if (u.includes(NEMO_MODELS.parakeet.repo)) out.add("parakeet");
        if (u.includes(NEMO_MODELS.canary.repo)) out.add("canary");
        for (const [p, info] of Object.entries(WHISPER_PRESETS)) if (u.includes(info.model)) out.add(`whisper-${p}`);
      }
    }
  } catch {}
  return out;
}
