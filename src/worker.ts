/// <reference lib="webworker" />
// The speech models run here, off the page: Parakeet and Canary through
// onnxruntime-web, Whisper through transformers.js, both from llm-connect.
// The page sends 16 kHz mono samples; the worker keeps the last model loaded
// so the next file starts at once.

import { loadNemo, type NemoTranscriber } from "llm-connect/nemo";
import { loadWhisper, type Whisper } from "llm-connect/whisper";
import type { Engine } from "./engine/engines.ts";

export type WorkerIn = { type: "run"; engine: Exclude<Engine, { kind: "cloud" }>; samples: Float32Array; language?: string };
export type WorkerOut =
  | { type: "load"; ratio: number; loaded: number; total: number }
  | { type: "ready"; device: string }
  | { type: "progress"; ratio: number }
  | { type: "done"; transcript: import("llm-connect").Transcript }
  | { type: "error"; message: string };

const ctx = self as unknown as DedicatedWorkerGlobalScope;
const send = (m: WorkerOut) => ctx.postMessage(m);
// onnxruntime-web's .wasm files, served by this site (scripts/build.ts), never from a CDN.
const ORT = new URL("./ort/", ctx.location.href).href;

let loaded: { key: string; nemo?: NemoTranscriber; whisper?: Whisper } | null = null;

async function model(engine: WorkerIn["engine"]) {
  const key = engine.kind === "nemo" ? engine.model : `whisper-${engine.preset}`;
  if (loaded?.key === key) return loaded;
  await loaded?.nemo?.dispose();
  await loaded?.whisper?.dispose();
  loaded = null;
  const onProgress = (p: { ratio: number; loaded: number; total: number }) => send({ type: "load", ...p });
  const load = async () =>
    engine.kind === "nemo"
      ? { key, nemo: await loadNemo({ model: engine.model, wasmPaths: ORT, onProgress }) }
      : { key, whisper: await loadWhisper({ preset: engine.preset, wasmPaths: ORT, onProgress }) };
  // Hundreds of megabytes over one connection: a dropped stream is retried
  // twice; files already complete come from the cache.
  for (let attempt = 0; ; attempt++) {
    try {
      loaded = await load();
      break;
    } catch (e) {
      const network = e instanceof TypeError || /network|fetch|load failed/i.test(String((e as Error)?.message));
      if (!network || attempt >= 2) throw e;
      await new Promise((r) => setTimeout(r, 1500 * (attempt + 1)));
    }
  }
  send({ type: "ready", device: loaded!.whisper?.device ?? "wasm" });
  return loaded!;
}

ctx.onmessage = async (e: MessageEvent<WorkerIn>) => {
  const { engine, samples, language } = e.data;
  try {
    const m = await model(engine);
    const transcript = m.nemo
      ? await m.nemo.transcribe(samples, { language, onProgress: (ratio) => send({ type: "progress", ratio }) })
      : await m.whisper!.transcribe(samples, { language });
    send({ type: "done", transcript });
  } catch (err) {
    send({ type: "error", message: err instanceof Error ? err.message : String(err) });
  }
};
