// Timecodes. Subtitles want milliseconds (SRT with a comma, VTT with a dot);
// the page shows m:ss, or h:mm:ss past an hour.

function parts(seconds: number) {
  const ms = Math.max(0, Math.round(seconds * 1000));
  return { h: Math.floor(ms / 3_600_000), m: Math.floor(ms / 60_000) % 60, s: Math.floor(ms / 1000) % 60, ms: ms % 1000 };
}

const pad = (n: number, w = 2) => String(n).padStart(w, "0");

export function srtTime(seconds: number): string {
  const { h, m, s, ms } = parts(seconds);
  return `${pad(h)}:${pad(m)}:${pad(s)},${pad(ms, 3)}`;
}

export function vttTime(seconds: number): string {
  return srtTime(seconds).replace(",", ".");
}

/** 0:07, 12:48, 1:02:03. */
export function clock(seconds: number): string {
  const { h, m, s } = parts(Math.floor(Math.max(0, seconds)));
  return h ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

/** "12:48", "1:02:03", "45" or "1h02" back to seconds; null when it is not a time. */
export function parseClock(text: string): number | null {
  const t = text.trim();
  if (/^\d+(\.\d+)?$/.test(t)) return Number(t);
  const hm = /^(\d+)h(\d{1,2})?$/i.exec(t);
  if (hm) return Number(hm[1]) * 3600 + Number(hm[2] ?? 0) * 60;
  const m = /^(?:(\d+):)?(\d{1,2}):(\d{1,2})(?:[.,](\d+))?$/.exec(t);
  if (!m) return null;
  return Number(m[1] ?? 0) * 3600 + Number(m[2]) * 60 + Number(m[3]) + (m[4] ? Number(`0.${m[4]}`) : 0);
}
