// The work surface: the media and its graduated timeline on one side, the
// transcript set as a tracklist on the other. The segment that plays is the
// track that plays; the word that plays is underlined; clicking a word seeks.
// Words keep the model's timings; a segment's text can be corrected, and then
// reads as one block.

import { joinWords } from "llm-connect";
import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { cutList, type Range } from "./edl/index.ts";
import { ExportPanel } from "./export-panel.tsx";
import { getDoc, media, saveDoc, useDocs } from "./engine/store.ts";
import { clock } from "./engine/time.ts";
import type { Doc, TranscriptPiece } from "./engine/types.ts";
import { languageLabel, useLang } from "./i18n.ts";
import { Icon } from "./icons.tsx";
import { Montage } from "./montage.tsx";
import { Timeline } from "./timeline.tsx";

const AiPanel = lazy(() => import("./ai-panel.tsx").then((m) => ({ default: m.AiPanel })));

export interface EditorApi {
  doc: Doc;
  file: File | null;
  duration: number;
  time: number;
  seek: (t: number) => void;
  update: (fn: (d: Doc) => Doc) => void;
  /** Add ranges to the cuts (undoable). */
  addCuts: (ranges: Range[]) => void;
  setCuts: (ranges: Range[]) => void;
  undo: (() => void) | null;
  preview: boolean;
  setPreview: (on: boolean) => void;
}

/** First index whose start is after `t`, minus one: the piece playing at `t`, or -1. */
function at(pieces: TranscriptPiece[], t: number): number {
  let lo = 0;
  let hi = pieces.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (pieces[mid]!.start <= t) {
      found = mid;
      lo = mid + 1;
    } else hi = mid - 1;
  }
  return found >= 0 && t <= pieces[found]!.end + 0.25 ? found : -1;
}

/** A model's **bold** and `code` as elements, never as HTML: everything else stays text. */
function inline(text: string): React.ReactNode[] {
  return text.split(/(\*\*[^*]+\*\*|`[^`]+`)/g).map((part, i) =>
    part.startsWith("**") && part.endsWith("**") && part.length > 4 ? (
      <strong key={i}>{part.slice(2, -2)}</strong>
    ) : part.startsWith("`") && part.endsWith("`") && part.length > 2 ? (
      <code key={i}>{part.slice(1, -1)}</code>
    ) : (
      part
    ),
  );
}

/** No space before closing punctuation or after an apostrophe, as joinWords does. */
const glued = (word: string, prev: string) => /^[,.!?;:)…%]/.test(word) || /['’(]$/.test(prev);

const inCut = (cuts: Range[], a: number, b: number) => cuts.some(([x, y]) => a >= x - 0.01 && b <= y + 0.01);

type Tab = "montage" | "ai" | "export";

export function Editor({ id }: { id: string }) {
  const { t, lang } = useLang();
  const docs = useDocs();
  const stored = docs.find((d) => d.id === id);
  const [file, setFile] = useState<File | null>(() => media.get(id) ?? null);
  const [time, setTime] = useState(0);
  const [tab, setTab] = useState<Tab>("montage");
  const [view, setView] = useState<string>("");
  const [editing, setEditing] = useState<number | null>(null);
  const [query, setQuery] = useState("");
  const [hasSelection, setHasSelection] = useState(false);
  const [preview, setPreview] = useState(false);
  const [mismatch, setMismatch] = useState(false);
  const history = useRef<Range[][]>([]);
  const player = useRef<HTMLVideoElement & HTMLAudioElement>(null);
  const list = useRef<HTMLOListElement>(null);
  const url = useMemo(() => (file ? URL.createObjectURL(file) : null), [file]);
  useEffect(() => () => void (url && URL.revokeObjectURL(url)), [url]);

  const doc = stored;
  const update = useCallback(
    (fn: (d: Doc) => Doc) => {
      // The store's latest copy, not this render's: several updates may land before the next render.
      const current = getDoc(id);
      if (current) void saveDoc(fn(current));
    },
    [id],
  );

  const segments = doc?.transcript.segments ?? [];
  const words = doc?.transcript.words ?? [];
  const duration = doc?.transcript.duration ?? segments.at(-1)?.end ?? 0;
  const cuts = (doc?.cuts ?? []) as Range[];
  const chapters = useMemo(() => [...(doc?.chapters ?? [])].sort((a, b) => a.start - b.start), [doc?.chapters]);
  const translation = view ? doc?.translations?.[view] : undefined;

  // Each segment's words, and whether its text is still theirs.
  const rows = useMemo(() => {
    let w = 0;
    return segments.map((s) => {
      while (w < words.length && words[w]!.start < s.start - 0.005) w++;
      const first = w;
      while (w < words.length && words[w]!.start < s.end - 0.005) w++;
      const own = words.slice(first, w);
      return { first, own, intact: own.length > 0 && joinWords(own.map((x) => x.text)) === s.text.trim() };
    });
  }, [segments, words]);

  const seek = useCallback((s: number) => {
    const p = player.current;
    const clamped = Math.min(Math.max(s, 0), duration);
    if (p) p.currentTime = clamped;
    setTime(clamped);
  }, [duration]);

  // The media's own length wins over the audio's: a video may run past its sound,
  // and "the last 3 seconds" counts from the real end.
  useEffect(() => {
    const p = player.current;
    if (!p) return;
    const onMeta = () => {
      if (Number.isFinite(p.duration) && p.duration > duration + 0.05) update((d) => ({ ...d, transcript: { ...d.transcript, duration: p.duration } }));
    };
    p.addEventListener("loadedmetadata", onMeta);
    if (p.readyState >= 1) onMeta();
    return () => p.removeEventListener("loadedmetadata", onMeta);
  }, [url, duration, update]);

  // Follow playback: smooth while playing, skipping the cuts in preview.
  useEffect(() => {
    const p = player.current;
    if (!p) return;
    let raf = 0;
    const tick = () => {
      let now = p.currentTime;
      if (preview) {
        const c = cuts.find(([a, b]) => now >= a && now < b - 0.05);
        if (c) {
          p.currentTime = c[1];
          now = c[1];
        }
      }
      setTime(now);
      if (!p.paused) raf = requestAnimationFrame(tick);
    };
    const start = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(tick);
    };
    p.addEventListener("play", start);
    p.addEventListener("seeked", tick);
    return () => {
      cancelAnimationFrame(raf);
      p.removeEventListener("play", start);
      p.removeEventListener("seeked", tick);
    };
  }, [url, preview, cuts]);

  // The playing word and segment, marked without re-rendering the list.
  const playingWord = at(words, time);
  const playingSeg = at(segments, time);
  useEffect(() => {
    const el = list.current;
    if (!el) return;
    el.querySelector(".w.now")?.classList.remove("now");
    if (playingWord >= 0) el.querySelector(`[data-w="${playingWord}"]`)?.classList.add("now");
  }, [playingWord]);
  const follow = useRef(true);
  useEffect(() => {
    if (playingSeg < 0 || !follow.current || player.current?.paused) return;
    list.current?.querySelector(`[data-s="${playingSeg}"]`)?.scrollIntoView({ block: "nearest", behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
  }, [playingSeg]);

  // Both read the document's latest cuts, not this render's: two quick clicks must both count.
  const setCuts = useCallback(
    (ranges: Range[]) => {
      update((d) => {
        history.current.push((d.cuts ?? []) as Range[]);
        return { ...d, cuts: cutList(ranges, duration).delete };
      });
    },
    [duration, update],
  );
  const addCuts = useCallback(
    (ranges: Range[]) => {
      update((d) => {
        history.current.push((d.cuts ?? []) as Range[]);
        return { ...d, cuts: cutList([...((d.cuts ?? []) as Range[]), ...ranges], duration).delete };
      });
    },
    [duration, update],
  );
  const undo = history.current.length
    ? () => {
        const prev = history.current.pop() ?? [];
        update((d) => ({ ...d, cuts: prev }));
      }
    : null;

  /** The selected words as a range, or null. */
  const selectionRange = useCallback((): Range | null => {
    const sel = getSelection();
    if (!sel || sel.isCollapsed || !list.current) return null;
    const pick = (n: Node | null) => (n instanceof Element ? n : n?.parentElement)?.closest<HTMLElement>("[data-w],[data-s]") ?? null;
    const a = pick(sel.anchorNode);
    const b = pick(sel.focusNode);
    if (!a || !b || !list.current.contains(a) || !list.current.contains(b)) return null;
    const span = (el: HTMLElement): Range => {
      if (el.dataset.w) {
        const w = words[Number(el.dataset.w)]!;
        return [w.start, w.end];
      }
      const s = segments[Number(el.dataset.s)]!;
      return [s.start, s.end];
    };
    const [x, y] = [span(a), span(b)];
    return [Math.min(x[0], y[0]), Math.max(x[1], y[1])];
  }, [words, segments]);

  useEffect(() => {
    const onSel = () => setHasSelection(selectionRange() !== null);
    document.addEventListener("selectionchange", onSel);
    return () => document.removeEventListener("selectionchange", onSel);
  }, [selectionRange]);

  const cutSelection = () => {
    const r = selectionRange();
    if (!r) return;
    addCuts([r]);
    getSelection()?.removeAllRanges();
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.closest("input, textarea, select, [contenteditable]")) return;
      if ((e.key === "Delete" || e.key === "Backspace") && selectionRange()) {
        e.preventDefault();
        cutSelection();
      } else if ((e.metaKey || e.ctrlKey) && e.key === "z" && undo) {
        e.preventDefault();
        undo();
      } else if (e.key === " " && player.current && !target.closest("button, a, [role=slider]")) {
        e.preventDefault();
        if (player.current.paused) void player.current.play();
        else player.current.pause();
      }
    };
    addEventListener("keydown", onKey);
    return () => removeEventListener("keydown", onKey);
  });

  if (!doc) {
    return (
      <div className="page narrow">
        <h1>{t("Transcript not found", "Transcription introuvable")}</h1>
        <p>
          <a href="#/history">{t("See the history", "Voir l'historique")}</a>
        </p>
      </div>
    );
  }

  const q = query.trim().toLowerCase();
  const matches = q ? segments.filter((s, i) => (translation?.[i] ?? s.text).toLowerCase().includes(q)).length : 0;
  const api: EditorApi = { doc, file, duration, time, seek, update, addCuts, setCuts, undo, preview, setPreview };
  const isVideo = file ? file.type.startsWith("video/") || /\.(mp4|mov|webm|mkv)$/i.test(file.name) : false;
  const chapterAt = (i: number) => chapters.filter((c) => c.start <= segments[i]!.start && (i === 0 || c.start > segments[i - 1]!.start));
  const removed = cuts.reduce((n, [a, b]) => n + b - a, 0);

  return (
    <div className="editor">
      <div className="deck">
        <div className={`player${isVideo ? " video" : " audio"}`}>
          {url ? (
            isVideo ? (
              <video ref={player} src={url} controls playsInline preload="metadata" />
            ) : (
              <audio ref={player} src={url} controls preload="metadata" />
            )
          ) : (
            <label className="reattach">
              <span>{t(`Open ${doc.name} again to play it and render edits. Its transcript is kept; the file never was.`, `Rouvrez ${doc.name} pour le lire et exporter le montage. La transcription est conservée, le fichier ne l'a jamais été.`)}</span>
              <input
                type="file"
                accept="audio/*,video/*"
                onChange={(e) => {
                  const f = e.currentTarget.files?.[0];
                  if (!f) return;
                  setMismatch(f.name !== doc.name || f.size !== doc.size);
                  media.set(doc.id, f);
                  setFile(f);
                }}
              />
              {mismatch ? <span className="warn">{t("This is not the same file: times may not match.", "Ce n'est pas le même fichier : les temps peuvent ne pas correspondre.")}</span> : null}
            </label>
          )}
        </div>
        <Timeline duration={duration} time={time} cuts={cuts} chapters={chapters} onSeek={seek} />
        <p className="deck-meta num">
          {clock(time)} / {clock(duration)}
          {cuts.length ? <> · {t(`${clock(duration - removed)} after cuts`, `${clock(duration - removed)} après coupes`)}</> : null}
        </p>
        <div className="tabs" role="tablist" aria-label={t("Tools", "Outils")}>
          {(
            [
              ["montage", t("Edit", "Montage")],
              ["ai", t("AI", "IA")],
              ["export", t("Export", "Exporter")],
            ] as const
          ).map(([k, label]) => (
            <button key={k} type="button" role="tab" id={`tab-${k}`} aria-selected={tab === k} aria-controls={`panel-${k}`} tabIndex={tab === k ? 0 : -1} onClick={() => setTab(k)}>
              {label}
            </button>
          ))}
        </div>
        <div className="panel" role="tabpanel" id={`panel-${tab}`} aria-labelledby={`tab-${tab}`}>
          {tab === "montage" ? <Montage api={api} /> : null}
          {tab === "ai" ? (
            <Suspense fallback={null}>
              <AiPanel api={api} onShow={setView} />
            </Suspense>
          ) : null}
          {tab === "export" ? <ExportPanel api={api} /> : null}
        </div>
      </div>

      <section className="sheet" aria-labelledby="doc-title">
        <header className="sheet-head">
          <h1 id="doc-title">{doc.name}</h1>
          <p className="sheet-meta num">
            {clock(duration)}
            {doc.transcript.language ? ` · ${languageLabel(doc.transcript.language, lang)}` : ""} · {doc.engine} · {segments.length} {t("segments", "segments")}
          </p>
        </header>
        {doc.summary ? (
          <div className="liner-notes">
            <h2>{t("Summary", "Résumé")}</h2>
            {doc.summary.split(/\n+/).map((p, i) => (
              <p key={i}>{inline(p.replace(/^\s*[-*]\s+/, "– "))}</p>
            ))}
          </div>
        ) : null}
        <div className="sheet-tools">
          <label className="search">
            <Icon name="search" size={16} />
            <span className="visually-hidden">{t("Search the transcript", "Chercher dans la transcription")}</span>
            <input type="search" value={query} placeholder={t("Search", "Chercher")} onChange={(e) => setQuery(e.currentTarget.value)} />
          </label>
          {q ? <span className="num note">{t(`${matches} segment(s)`, `${matches} segment(s)`)}</span> : null}
          {Object.keys(doc.translations ?? {}).length ? (
            <label className="field inline">
              <span>{t("Text", "Texte")}</span>
              <select value={view} onChange={(e) => setView(e.currentTarget.value)}>
                <option value="">{t("Original", "Original")}</option>
                {Object.keys(doc.translations ?? {}).map((l) => (
                  <option key={l} value={l}>
                    {languageLabel(l, lang)}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
          <button type="button" className="secondary small" disabled={!hasSelection} onClick={cutSelection} title={t("Select words in the transcript, then cut them (Delete key)", "Sélectionnez des mots, puis coupez-les (touche Suppr)")}>
            <Icon name="cut" size={16} /> {t("Cut selection", "Couper la sélection")}
          </button>
        </div>

        {segments.length ? (
          <ol className="tracklist" ref={list} onMouseEnter={() => (follow.current = false)} onMouseLeave={() => (follow.current = true)}>
            {segments.map((s, i) => {
              const row = rows[i]!;
              const text = translation?.[i] ?? s.text;
              const hidden = q && !text.toLowerCase().includes(q);
              const segCut = inCut(cuts, s.start, s.end);
              return (
                <li key={i} data-s={i} className={`track${i === playingSeg ? " playing" : ""}${segCut ? " cut" : ""}${hidden ? " dim" : ""}`} aria-current={i === playingSeg ? "true" : undefined}>
                  {chapterAt(i).map((c) => (
                    <h3 key={c.start} className="chapter-head">
                      <button type="button" className="time num" onClick={() => seek(c.start)}>
                        {clock(c.start)}
                      </button>
                      {c.title}
                    </h3>
                  ))}
                  <div className="track-row">
                    <span className="track-no num" aria-hidden="true">
                      {i === playingSeg ? <Icon name="play" size={12} /> : i + 1}
                    </span>
                    <button type="button" className="time num" onClick={() => seek(s.start)} aria-label={t(`Play from ${clock(s.start)}`, `Lire depuis ${clock(s.start)}`)}>
                      {clock(s.start)}
                    </button>
                    {editing === i ? (
                      <textarea
                        className="track-edit"
                        defaultValue={s.text}
                        rows={Math.max(2, Math.ceil(s.text.length / 60))}
                        aria-label={t(`Text of segment ${i + 1}`, `Texte du segment ${i + 1}`)}
                        // oxlint-disable-next-line jsx-a11y/no-autofocus
                        autoFocus
                        onKeyDown={(e) => {
                          if (e.key === "Escape") setEditing(null);
                          if (e.key === "Enter" && !e.shiftKey) {
                            e.preventDefault();
                            const value = e.currentTarget.value.trim();
                            update((d) => ({ ...d, transcript: { ...d.transcript, segments: d.transcript.segments.map((x, j) => (j === i ? { ...x, text: value } : x)) } }));
                            setEditing(null);
                          }
                        }}
                        onBlur={() => setEditing(null)}
                      />
                    ) : (
                      <p className="track-text" lang={view || doc.transcript.language}>
                        {!translation && row.intact
                          ? row.own.map((w, k) => {
                              const n = row.first + k;
                              return (
                                <span key={n} data-w={n} className={`w${!segCut && inCut(cuts, w.start, w.end) ? " cut" : ""}${q && w.text.toLowerCase().includes(q) ? " hit" : ""}`} onClick={() => getSelection()?.isCollapsed !== false && seek(w.start)}>
                                  {k && !glued(w.text, row.own[k - 1]!.text) ? " " : ""}
                                  {w.text}
                                </span>
                              );
                            })
                          : <span data-s={i} onClick={() => getSelection()?.isCollapsed !== false && seek(s.start)}>{text}</span>}
                      </p>
                    )}
                    <div className="track-actions">
                      <button type="button" className="icon-button" onClick={() => setEditing(i)} aria-label={t(`Correct segment ${i + 1}`, `Corriger le segment ${i + 1}`)}>
                        <Icon name="edit" size={16} />
                      </button>
                      <button
                        type="button"
                        className="icon-button"
                        aria-pressed={segCut}
                        onClick={() => (segCut ? setCuts(cuts.filter(([a, b]) => !(a <= s.start + 0.01 && b >= s.end - 0.01))) : addCuts([[s.start, s.end]]))}
                        aria-label={segCut ? t(`Keep segment ${i + 1}`, `Garder le segment ${i + 1}`) : t(`Cut segment ${i + 1}`, `Couper le segment ${i + 1}`)}
                      >
                        <Icon name={segCut ? "undo" : "cut"} size={16} />
                      </button>
                    </div>
                  </div>
                </li>
              );
            })}
          </ol>
        ) : (
          <p className="empty">{t("No speech was found in this file.", "Aucune parole n'a été trouvée dans ce fichier.")}</p>
        )}
      </section>
    </div>
  );
}
