/// <reference lib="webworker" />
// The speech models run here, off the page: Parakeet and Canary through
// onnxruntime-web, Whisper through transformers.js, both from llm-connect.
// The page sends 16 kHz mono samples; the worker keeps the last model loaded
// so the next file starts at once.

import { joinWords, type Transcript, type TranscriptPiece, wordsFromSegments } from "llm-connect";
import { loadNemo, type NemoTranscriber, speechSpans } from "llm-connect/nemo";
import { loadWhisper, type Whisper } from "llm-connect/whisper";
import type { Engine } from "./engine/engines.ts";
import { collapseLoops } from "./engine/loops.ts";

export type WorkerIn = { type: "run"; engine: Exclude<Engine, { kind: "cloud" }>; samples: Float32Array; language?: string };
export type WorkerOut =
  | { type: "load"; ratio: number; loaded: number; total: number }
  | { type: "ready"; device: string }
  | { type: "progress"; ratio: number }
  /** A stretch of the audio is done: its segments, on the file's timeline, and how far the run got. */
  | { type: "partial"; segments: TranscriptPiece[]; upTo: number; duration: number }
  | { type: "done"; transcript: Transcript }
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

const RATE = 16_000;

function shift(pieces: TranscriptPiece[], by: number): TranscriptPiece[] {
  return pieces.map((p) => ({ text: p.text, start: p.start + by, end: p.end + by }));
}

ctx.onmessage = async (e: MessageEvent<WorkerIn>) => {
  const { engine, samples, language } = e.data;
  try {
    const m = await model(engine);
    const duration = samples.length / RATE;
    // Stretches of at most 30 s, cut at the quietest moment: each one's text is sent as soon as it
    // is ready, so the page shows the transcript growing instead of waiting for the end.
    const spans = speechSpans(samples, RATE, { maxSeconds: 30 });
    const segments: TranscriptPiece[] = [];
    const words: TranscriptPiece[] = [];
    let detected: string | undefined;
    let timings: Transcript["wordTimings"] = "none";
    for (const span of spans) {
      const part = samples.subarray(Math.floor(span.start * RATE), Math.ceil(span.end * RATE));
      const t = m.nemo ? await m.nemo.transcribe(part, { language }) : await m.whisper!.transcribe(part, { language });
      detected ??= t.language;
      if (t.wordTimings !== "none") timings = t.wordTimings;
      let segs = shift(t.segments, span.start);
      let ws = shift(t.words, span.start);
      // Canary (phrases, estimated words) can loop: clean the phrase and spread its words again.
      if (t.wordTimings === "estimated" && segs.some((s) => collapseLoops(s.text) !== s.text.trim())) {
        segs = segs.map((s) => ({ ...s, text: collapseLoops(s.text) }));
        ws = wordsFromSegments(segs);
      }
      segments.push(...segs);
      words.push(...ws);
      send({ type: "partial", segments: segs, upTo: span.end, duration });
    }
    const transcript: Transcript = {
      text: joinWords(segments.map((s) => s.text)),
      ...(detected ? { language: detected } : language ? { language } : {}),
      duration,
      segments,
      words,
      wordTimings: segments.length ? timings : "none",
    };
    send({ type: "done", transcript });
  } catch (err) {
    send({ type: "error", message: err instanceof Error ? err.message : String(err) });
  }
};
