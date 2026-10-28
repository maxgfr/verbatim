// The Rust engine the CLI uses (crates/edl), compiled to WebAssembly: cut
// lists, plain-language edits, fillers, silences, transcript remapping and
// every text export. Values cross as JSON, in the shapes `verbatim` reads.

import init, * as edl from "./pkg/edl.js";
import type { Chapter, Transcript, TranscriptPiece } from "../engine/types.ts";

export type Range = [number, number];

export type Span =
  | { span: "between"; start: number; end: number }
  | { span: "first"; seconds: number }
  | { span: "last"; seconds: number }
  | { span: "from"; start: number }
  | { span: "until"; end: number };

export type Edit = { op: "delete" | "keep" } & Span;

export interface CutList {
  version: number;
  duration: number;
  delete: Range[];
  source?: string;
}

export interface Parsed {
  edits: Edit[];
  unparsed: string[];
}

export type Format = "txt" | "txt-timed" | "srt" | "vtt" | "md" | "json" | "csv";

export interface ExportDoc {
  name: string;
  engine: string;
  transcript: Transcript;
  summary?: string;
  chapters?: Chapter[];
}

let ready: Promise<unknown> | null = null;

/** Load the engine once; `source` is the .wasm (a URL on the page, bytes in tests). */
export function loadEdl(source: string | URL | BufferSource = new URL("./wasm/edl.wasm", document.baseURI)): Promise<unknown> {
  return (ready ??= init({ module_or_path: source }));
}

const words = (w: TranscriptPiece[]) => JSON.stringify(w);

export const parseInstruction = (text: string): Parsed => JSON.parse(edl.parseInstruction(text));
export const resolve = (edits: Edit[], duration: number): CutList => JSON.parse(edl.resolve(JSON.stringify(edits), duration));
export const cutList = (ranges: Range[], duration: number): CutList => JSON.parse(edl.cutList(JSON.stringify(ranges), duration));
export const fillers = (w: TranscriptPiece[]): Range[] => JSON.parse(edl.fillers(words(w)));
export const silences = (w: TranscriptPiece[], duration: number, minGap: number, pad = 0.15): Range[] => JSON.parse(edl.silences(words(w), duration, minGap, pad));
export const findText = (w: TranscriptPiece[], query: string): Range[] => JSON.parse(edl.findText(words(w), query));
export const remap = (t: Transcript, cuts: CutList): Transcript => JSON.parse(edl.remap(JSON.stringify(t), JSON.stringify(cuts)));
export const cliArgs = (cuts: CutList): string => edl.cliArgs(JSON.stringify(cuts));
export const render = (doc: ExportDoc, format: Format): string => edl.render(JSON.stringify(doc), format);

/** The kept ranges, in order. */
export function keeps(cuts: CutList): Range[] {
  const out: Range[] = [];
  let at = 0;
  for (const [a, b] of cuts.delete) {
    if (a > at) out.push([at, a]);
    at = Math.max(at, b);
  }
  if (cuts.duration > at) out.push([at, cuts.duration]);
  return out;
}

export function removed(cuts: CutList): number {
  return cuts.delete.reduce((n, [a, b]) => n + b - a, 0);
}

/** What a model may answer when asked to plan an edit (the CLI reads the same). */
export interface Plan {
  edits: Edit[];
  fillers: boolean;
  silences: number | null;
  phrases: string[];
  note: string | null;
}

export const planSystem = (): string => edl.planSystem();
export const planPrompt = (instruction: string, duration: number, transcript?: Transcript): string => edl.planPrompt(instruction, duration, transcript ? JSON.stringify(transcript) : "");
export const parsePlan = (text: string): Plan => JSON.parse(edl.parsePlan(text));
