// Downloads: every format comes from the Rust engine (crates/edl/src/export.rs),
// the same code `verbatim transcribe -f …` runs. A translation is exported by
// swapping the segment texts and keeping their times.

import { strToU8, zipSync } from "fflate";
import { type ExportDoc, type Format, render } from "../edl/index.ts";
import type { Doc } from "./types.ts";

export const FORMATS: { id: Format; ext: string; mime: string; label: string }[] = [
  { id: "txt", ext: "txt", mime: "text/plain", label: "TXT" },
  { id: "txt-timed", ext: "txt", mime: "text/plain", label: "TXT + time" },
  { id: "srt", ext: "srt", mime: "application/x-subrip", label: "SRT" },
  { id: "vtt", ext: "vtt", mime: "text/vtt", label: "VTT" },
  { id: "md", ext: "md", mime: "text/markdown", label: "Markdown" },
  { id: "json", ext: "json", mime: "application/json", label: "JSON" },
  { id: "csv", ext: "csv", mime: "text/csv", label: "CSV" },
];

export function exportDoc(doc: Doc, lang?: string): ExportDoc {
  const tr = lang ? doc.translations?.[lang] : undefined;
  const segments = tr ? doc.transcript.segments.map((s, i) => ({ ...s, text: tr[i] ?? s.text })) : doc.transcript.segments;
  return {
    name: doc.name,
    engine: doc.engine,
    transcript: { ...doc.transcript, segments, language: lang ?? doc.transcript.language },
    summary: doc.summary,
    chapters: doc.chapters,
  };
}

export const renderDoc = (doc: Doc, format: Format, lang?: string) => render(exportDoc(doc, lang), format);

/** The media's name without its extension, safe as a file name. */
export function baseName(name: string): string {
  const stem = name.replace(/\.[^.]+$/, "").trim();
  // oxlint-disable-next-line no-control-regex
  return stem.replace(/[\\/:*?"<>|\u0000-\u001f]+/g, "_") || "transcript";
}

export function fileName(doc: Doc, format: Format, lang?: string): string {
  const f = FORMATS.find((x) => x.id === format)!;
  return `${baseName(doc.name)}${format === "txt-timed" ? ".timed" : ""}${lang ? `.${lang}` : ""}.${f.ext}`;
}

/** Every format in one zip, plus each translation's subtitles and text. */
export function toZip(doc: Doc): Uint8Array {
  const files: Record<string, Uint8Array> = {};
  for (const f of FORMATS) files[fileName(doc, f.id)] = strToU8(renderDoc(doc, f.id));
  for (const lang of Object.keys(doc.translations ?? {})) {
    for (const f of ["srt", "vtt", "txt"] as const) files[fileName(doc, f, lang)] = strToU8(renderDoc(doc, f, lang));
  }
  return zipSync(files, { level: 6 });
}

/** Hand a file to the browser's downloads. */
export function download(name: string, body: BlobPart, mime: string) {
  const url = URL.createObjectURL(new Blob([body], { type: mime }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}
