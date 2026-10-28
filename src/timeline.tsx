// The graduated hairline under the player: ticks at a readable step, the cuts
// as tinted spans, chapters as taller ticks, the playhead. Click or arrow keys
// to move through the media.

import { useMemo } from "react";
import { clock } from "./engine/time.ts";
import type { Chapter } from "./engine/types.ts";
import { useLang } from "./i18n.ts";

const STEPS = [1, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600];

export function Timeline({ duration, time, cuts, chapters, onSeek }: { duration: number; time: number; cuts: [number, number][]; chapters: Chapter[]; onSeek: (t: number) => void }) {
  const { t } = useLang();
  // About ten labelled ticks, four minor ones between them.
  const step = STEPS.find((s) => duration / s <= 10) ?? 3600;
  const ticks = useMemo(() => {
    const out: { at: number; major: boolean }[] = [];
    const minor = step / 5;
    for (let i = 0; i * minor <= duration + 1e-6; i++) out.push({ at: i * minor, major: i % 5 === 0 });
    return out;
  }, [duration, step]);
  if (!duration) return null;
  const pct = (s: number) => `${(Math.min(Math.max(s, 0), duration) / duration) * 100}%`;
  const seekFrom = (e: React.PointerEvent<HTMLDivElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    onSeek(((e.clientX - r.left) / r.width) * duration);
  };
  return (
    <div
      className="timeline"
      role="slider"
      tabIndex={0}
      aria-label={t("Position", "Position")}
      aria-valuemin={0}
      aria-valuemax={Math.round(duration)}
      aria-valuenow={Math.round(time)}
      aria-valuetext={`${clock(time)} / ${clock(duration)}`}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        seekFrom(e);
      }}
      onPointerMove={(e) => {
        if (e.buttons === 1) seekFrom(e);
      }}
      onKeyDown={(e) => {
        const d = e.shiftKey ? 30 : 5;
        if (e.key === "ArrowRight") onSeek(time + d);
        else if (e.key === "ArrowLeft") onSeek(time - d);
        else if (e.key === "Home") onSeek(0);
        else if (e.key === "End") onSeek(duration);
        else return;
        e.preventDefault();
      }}
    >
      <div className="timeline-track">
        {cuts.map(([a, b]) => (
          <span key={`${a}-${b}`} className="timeline-cut" style={{ insetInlineStart: pct(a), inlineSize: `calc(${pct(b)} - ${pct(a)})` }} />
        ))}
        {ticks.map((k) => (
          <span key={k.at} className={k.major ? "tick major" : "tick"} style={{ insetInlineStart: pct(k.at) }} />
        ))}
        {chapters.map((c) => (
          <span key={c.start} className="tick chapter" style={{ insetInlineStart: pct(c.start) }} title={c.title} />
        ))}
        <span className="playhead" style={{ insetInlineStart: pct(time) }} />
      </div>
      <div className="timeline-labels" aria-hidden="true">
        {ticks
          .filter((k) => k.major)
          .map((k) => (
            <span key={k.at} className="num" style={{ insetInlineStart: pct(k.at) }}>
              {clock(k.at)}
            </span>
          ))}
      </div>
    </div>
  );
}
