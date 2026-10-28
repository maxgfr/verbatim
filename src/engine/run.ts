// One transcription, from a file to a document: decode on the page, run the
// model in the worker (or the user's provider for "cloud"), then keep the
// result in the history.

import { decodeAudio, spokenLanguage } from "llm-connect/whisper";
import { scriptURL } from "../trusted.ts";
import { type Engine, engineLabel } from "./engines.ts";
import type { Doc, Transcript } from "./types.ts";
import type { WorkerIn, WorkerOut } from "../worker.ts";

export type Stage =
  | { stage: "decode" }
  | { stage: "download"; ratio: number; loaded: number; total: number }
  | { stage: "transcribe"; ratio: number | null; elapsed: number; duration: number }
  | { stage: "upload" };

let worker: Worker | null = null;
let abort: ((e: Error) => void) | null = null;
const getWorker = () => (worker ??= new Worker(scriptURL("./worker.js"), { type: "module", name: "verbatim-speech" }));

/** Stop whatever runs: the worker and its model are dropped, the run fails with "cancelled". */
export function cancelRun() {
  worker?.terminate();
  worker = null;
  abort?.(new Error("cancelled"));
}

export const newId = () => crypto.randomUUID();

export async function transcribeFile(file: File, engine: Engine, language: string, onStage: (s: Stage) => void): Promise<Doc> {
  const lang = language === "auto" ? undefined : language;
  let transcript: Transcript;
  if (engine.kind === "cloud") {
    onStage({ stage: "upload" });
    const { cloudTranscribe } = await import("../ai/byok.ts");
    transcript = await cloudTranscribe(file, lang);
  } else {
    onStage({ stage: "decode" });
    const samples = await decodeAudio(file);
    const duration = samples.length / 16_000;
    const started = performance.now();
    const w = getWorker();
    transcript = await new Promise<Transcript>((resolve, reject) => {
      let ratio: number | null = null;
      let transcribing = false;
      const tick = setInterval(() => transcribing && onStage({ stage: "transcribe", ratio, elapsed: (performance.now() - started) / 1000, duration }), 500);
      const done = () => {
        clearInterval(tick);
        abort = null;
      };
      abort = (e) => {
        done();
        reject(e);
      };
      w.onmessage = (e: MessageEvent<WorkerOut>) => {
        const m = e.data;
        if (m.type === "load") onStage({ stage: "download", ratio: m.ratio, loaded: m.loaded, total: m.total });
        else if (m.type === "ready" || m.type === "progress") {
          transcribing = true;
          if (m.type === "progress") ratio = m.ratio;
          onStage({ stage: "transcribe", ratio, elapsed: (performance.now() - started) / 1000, duration });
        } else if (m.type === "done") {
          done();
          resolve(m.transcript);
        } else {
          done();
          reject(new Error(m.message));
        }
      };
      w.onerror = (e) => {
        done();
        reject(new Error(e.message || "the speech worker stopped"));
      };
      // Whisper does not detect the language: without one it listens for the browser's.
      const msg: WorkerIn = { type: "run", engine, samples, language: lang ?? (engine.kind === "whisper" ? spokenLanguage() : undefined) };
      w.postMessage(msg, [samples.buffer]);
    });
    // The media's length, not where the speech ends: "the last 3 seconds" counts from the real end.
    transcript = { ...transcript, duration };
  }
  const now = Date.now();
  return { id: newId(), name: file.name, mime: file.type, size: file.size, createdAt: now, updatedAt: now, engine: engine.kind === "cloud" ? "Cloud" : engineLabel(engine), transcript };
}
