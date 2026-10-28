// Reading models' answers: the JSON inside, whatever prose or fence surrounds it.

import { clock, parseClock } from "../engine/time.ts";
import type { Chapter, TranscriptPiece } from "../engine/types.ts";

/** The first JSON array in `text`, or null. */
export function jsonArray(text: string): unknown[] | null {
  const a = text.indexOf("[");
  const b = text.lastIndexOf("]");
  if (a < 0 || b <= a) return null;
  try {
    const v = JSON.parse(text.slice(a, b + 1));
    return Array.isArray(v) ? v : null;
  } catch {
    return null;
  }
}

/** Chapters from `[{ "start": "12:48" | 768, "title": "…" }]`, sorted, inside the duration, at most one per start. */
export function parseChapters(text: string, duration: number): Chapter[] {
  const out: Chapter[] = [];
  for (const item of jsonArray(text) ?? []) {
    if (!item || typeof item !== "object") continue;
    const { start, title } = item as { start?: unknown; title?: unknown };
    const s = typeof start === "number" ? start : typeof start === "string" ? parseClock(start) : null;
    const t = typeof title === "string" ? title.trim() : "";
    if (s === null || !Number.isFinite(s) || s < 0 || s > duration || !t) continue;
    if (!out.some((c) => Math.abs(c.start - s) < 1)) out.push({ start: s, title: t });
  }
  return out.sort((a, b) => a.start - b.start);
}

/** `n` strings from an answer that should be a JSON array of `n` strings; null when it is not. */
export function parseLines(text: string, n: number): string[] | null {
  const arr = jsonArray(text);
  if (!arr || arr.length !== n || !arr.every((x) => typeof x === "string")) return null;
  return arr as string[];
}

/** Segments as `[m:ss] text` lines, cut at `limit` characters. */
export function timedLines(segments: TranscriptPiece[], limit = 300_000): string {
  let out = "";
  for (const s of segments) {
    const line = `[${clock(s.start)}] ${s.text.trim()}\n`;
    if (out.length + line.length > limit) return `${out}[… truncated]\n`;
    out += line;
  }
  return out;
}

/** Consecutive groups of at most `size` items. */
export function batches<T>(items: T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) out.push(items.slice(i, i + size));
  return out;
}
