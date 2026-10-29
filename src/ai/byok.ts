// Optional AI with the user's own provider, through llm-connect: the key is
// encrypted at rest under a non-extractable WebCrypto key, the catalog is the
// static ./models.json, and nothing is sent until the user saves a provider
// and asks for something. What is sent is the transcript's text, never the
// media, except for cloud transcription, which the page announces first.

import "./zod-jitless.ts";
import { generateText } from "ai";
import {
  callProblem,
  createLlmSettingsStore,
  describeError,
  isConfigured,
  type LlmSettings,
  loadModel,
  modelIdOf,
  providerKindOf,
  resolveCapabilities,
  secureStorage,
  transcribe,
  transcriptionProblem,
} from "llm-connect";
import { parsePlan, planPrompt, planSystem, type Plan } from "../edl/index.ts";
import { tr } from "../i18n.ts";
import type { Chapter, Doc, Transcript } from "../engine/types.ts";
import { batches, parseChapters, parseLines, timedLines } from "./parse.ts";

export const llm = createLlmSettingsStore({
  catalog: "./models.json",
  storage: secureStorage({ prefsKey: "verbatim-ai", dbName: "verbatim-secrets" }),
  recommended: ["anthropic", "openai", "google", "mistral", "groq", "openrouter"],
});

let loaded: Promise<unknown> | null = null;
export const loadLlm = () => (loaded ??= llm.load().catch(() => null));

export function savedSettings(): LlmSettings | null {
  const s = llm.getState().saved;
  return s && isConfigured(s) ? (s as LlmSettings) : null;
}

function settings(): LlmSettings {
  const s = savedSettings();
  if (!s) throw new Error(tr("Set up an AI provider first (AI settings).", "Configurez d'abord un fournisseur d'IA (réglages IA)."));
  return s;
}

/** Why cloud transcription cannot run with the saved provider, or null. */
export function cloudProblem(): string | null {
  const s = savedSettings();
  if (!s) return tr("No AI provider is set up.", "Aucun fournisseur d'IA n'est configuré.");
  return transcriptionProblem(s)?.message ?? null;
}

export async function cloudTranscribe(file: File, language?: string): Promise<Transcript> {
  await loadLlm();
  const s = settings();
  const problem = transcriptionProblem(s);
  if (problem) throw new Error(problem.message);
  try {
    return await transcribe(s, file, { language });
  } catch (e) {
    throw new Error(describeError(e).message);
  }
}

const LANG_NAMES: Record<string, string> = { en: "English", fr: "French", de: "German", es: "Spanish", it: "Italian", pt: "Portuguese", nl: "Dutch", pl: "Polish", ru: "Russian", uk: "Ukrainian" };
export const languageName = (code: string) => LANG_NAMES[code] ?? new Intl.DisplayNames(["en"], { type: "language" }).of(code) ?? code;

async function ask(system: string, prompt: string, signal?: AbortSignal): Promise<string> {
  await loadLlm();
  const s = settings();
  // AI SDK 7 takes the system prompt as `instructions`, never as a message.
  const messages = [{ role: "user" as const, content: prompt }];
  const problem = callProblem(s, { messages }, resolveCapabilities({ kind: providerKindOf(s), modelId: modelIdOf(s) }));
  if (problem) throw new Error(problem.message);
  try {
    const res = await generateText({ model: await loadModel(s), instructions: system, messages, maxRetries: 1, abortSignal: signal });
    return res.text;
  } catch (e) {
    throw new Error(describeError(e).message);
  }
}

/** The language answers are written in: the transcript's, else the interface's. */
const writeIn = (doc: Doc, ui: string) => languageName(doc.transcript.language ?? ui);

export async function summarize(doc: Doc, ui: string, signal?: AbortSignal): Promise<string> {
  const text = await ask(
    `You summarize transcripts. Write in ${writeIn(doc, ui)}. Start with two or three sentences on what it is about, then the key points as a short "-" list. Plain Markdown, no title, nothing invented.`,
    timedLines(doc.transcript.segments),
    signal,
  );
  return text.trim();
}

export async function chapters(doc: Doc, ui: string, signal?: AbortSignal): Promise<Chapter[]> {
  const duration = doc.transcript.duration ?? doc.transcript.segments.at(-1)?.end ?? 0;
  const text = await ask(
    `You split transcripts into chapters. Answer with a JSON array only: [{"start": "m:ss", "title": "…"}]. The first chapter starts at 0:00; start each one where its topic begins, at a time shown in the transcript; 3 to 12 chapters depending on length; titles of 2 to 6 words in ${writeIn(doc, ui)}.`,
    timedLines(doc.transcript.segments),
    signal,
  );
  const out = parseChapters(text, duration);
  if (!out.length) throw new Error(tr("The model gave no usable chapters.", "Le modèle n'a pas donné de chapitres utilisables."));
  return out;
}

/** Each segment rewritten by the model, `size` at a time, keeping the count so the times still fit. */
async function rewrite(doc: Doc, system: string, onProgress: (done: number) => void, signal?: AbortSignal): Promise<string[]> {
  const segs = doc.transcript.segments.map((s) => s.text);
  const out: string[] = [];
  for (const group of batches(segs, 40)) {
    const text = await ask(`${system} Answer with a JSON array of exactly ${group.length} strings, one per input line, in the same order. Never merge or split lines.`, JSON.stringify(group), signal);
    const lines = parseLines(text, group.length);
    if (!lines) throw new Error(tr("The model did not return one line per segment; try again or another model.", "Le modèle n'a pas rendu une ligne par segment ; réessayez ou changez de modèle."));
    out.push(...lines);
    onProgress(out.length / segs.length);
  }
  return out;
}

export const translate = (doc: Doc, lang: string, onProgress: (done: number) => void, signal?: AbortSignal) =>
  rewrite(doc, `Translate each line of this transcript into ${languageName(lang)}, as natural spoken ${languageName(lang)} suitable for subtitles.`, onProgress, signal);

export const cleanUp = (doc: Doc, onProgress: (done: number) => void, signal?: AbortSignal) =>
  rewrite(doc, "Correct each line of this transcript: punctuation, capitals, obvious mishearings, and remove hesitations (euh, um, uh). Keep the speaker's words and language; do not summarize.", onProgress, signal);

/** An edit the phrase reader could not follow, planned by the model (the same question the CLI asks). */
export async function planEdit(instruction: string, duration: number, transcript: Transcript, signal?: AbortSignal): Promise<Plan> {
  const text = await ask(planSystem(), planPrompt(instruction, duration, transcript), signal);
  return parsePlan(text);
}
