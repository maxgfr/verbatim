// The sleeve: drop a file, see what it will cost, start. Several files queue
// and run one after the other.

import { useEffect, useRef, useState } from "react";
import { go } from "./app.tsx";
import { cachedEngines, ENGINE_IDS, engineLabel, engineProblem, engineSize, type EngineId, LANGUAGES, resolveEngine } from "./engine/engines.ts";
import { cancelRun, type Stage, transcribeFile } from "./engine/run.ts";
import { media, saveDoc } from "./engine/store.ts";
import { clock } from "./engine/time.ts";
import { bytes, languageLabel, useLang } from "./i18n.ts";
import { Icon } from "./icons.tsx";

interface Job {
  key: string;
  file: File;
  state: "waiting" | "running" | "done" | "error" | "cancelled";
  stage?: Stage;
  error?: string;
  docId?: string;
}

const ENGINE_KEY = "verbatim-engine";
const LANGUAGE_KEY = "verbatim-language";
const ACCEPT = "audio/*,video/*,.mkv,.mov,.m4a,.opus,.flac";

function browserLanguage(): string {
  const code = navigator.language?.toLowerCase().split(/[-_]/)[0] ?? "";
  return (LANGUAGES as readonly string[]).includes(code) ? code : "auto";
}

function usePersisted<T extends string>(key: string, initial: T): [T, (v: T) => void] {
  const [v, setV] = useState<T>(() => (localStorage.getItem(key) as T | null) ?? initial);
  return [v, (next) => (localStorage.setItem(key, next), setV(next))];
}

function engineName(id: EngineId, t: (en: string, fr: string) => string): string {
  switch (id) {
    case "auto":
      return t("Automatic", "Automatique");
    case "parakeet":
      return "Parakeet";
    case "canary":
      return "Canary";
    case "whisper-fast":
      return "Whisper base";
    case "whisper-balanced":
      return "Whisper small";
    case "whisper-accurate":
      return "Whisper large v3 turbo";
    case "cloud":
      return t("Your AI provider (uploads)", "Votre fournisseur d'IA (envoi)");
  }
}

function StageLine({ stage }: { stage?: Stage }) {
  const { t, lang } = useLang();
  if (!stage) return <span>{t("Waiting", "En attente")}</span>;
  switch (stage.stage) {
    case "decode":
      return <span>{t("Reading the audio…", "Lecture de l'audio…")}</span>;
    case "upload":
      return <span>{t("Sending to your provider…", "Envoi à votre fournisseur…")}</span>;
    case "download":
      return (
        <span>
          {t("Downloading the model, once", "Téléchargement du modèle, une seule fois")} · <span className="num">{Math.round(stage.ratio * 100)} %</span>
          {stage.total ? <span className="num"> · {bytes(stage.loaded, lang)} / {bytes(stage.total, lang)}</span> : null}
        </span>
      );
    case "transcribe": {
      const left = stage.ratio && stage.ratio > 0.02 ? (stage.elapsed / stage.ratio) * (1 - stage.ratio) : null;
      return (
        <span>
          {t("Transcribing", "Transcription")} {clock(stage.duration)}
          {stage.ratio !== null ? <span className="num"> · {Math.round(stage.ratio * 100)} %</span> : null}
          <span className="num"> · {clock(stage.elapsed)}</span>
          {left !== null ? <span className="num"> · {t(`about ${clock(left)} left`, `environ ${clock(left)} restantes`)}</span> : null}
        </span>
      );
    }
  }
}

function ratioOf(stage?: Stage): number | null {
  if (!stage) return 0;
  if (stage.stage === "download") return stage.ratio;
  if (stage.stage === "transcribe") return stage.ratio;
  return null;
}

export function Home() {
  const { t, lang } = useLang();
  const [engineId, setEngineId] = usePersisted<EngineId>(ENGINE_KEY, "auto");
  // The browser's language when the models know it: told to the model, it is never guessed wrong.
  const [language, setLanguage] = usePersisted<string>(LANGUAGE_KEY, browserLanguage());
  const [jobs, setJobs] = useState<Job[]>([]);
  const [over, setOver] = useState(false);
  const [cached, setCached] = useState<Set<string>>(new Set());
  const input = useRef<HTMLInputElement>(null);
  const running = useRef(false);
  const stopped = useRef(false);
  const engine = resolveEngine(engineId, language);
  const problem = engineProblem(engine, language);
  const cacheKey = engine.kind === "nemo" ? engine.model : engine.kind === "whisper" ? `whisper-${engine.preset}` : "";

  useEffect(() => {
    void cachedEngines().then(setCached);
  }, [jobs.length]);

  const patch = (key: string, p: Partial<Job>) => setJobs((js) => js.map((j) => (j.key === key ? { ...j, ...p } : j)));

  async function pump(queue: Job[]) {
    if (running.current) return;
    running.current = true;
    stopped.current = false;
    let opened = false;
    for (const job of queue) {
      if (stopped.current) break;
      if (job.state !== "waiting") continue;
      patch(job.key, { state: "running" });
      try {
        const doc = await transcribeFile(job.file, engine, language, (stage) => patch(job.key, { stage }));
        media.set(doc.id, job.file);
        await saveDoc(doc);
        patch(job.key, { state: "done", docId: doc.id });
        if (queue.length === 1 && !opened) {
          opened = true;
          go(`/t/${doc.id}`);
        }
      } catch (e) {
        if ((e as Error).message !== "cancelled") patch(job.key, { state: "error", error: (e as Error).message });
      }
    }
    running.current = false;
  }

  function add(files: FileList | File[]) {
    const list = [...files].filter((f) => f.type.startsWith("audio/") || f.type.startsWith("video/") || /\.(mkv|mov|m4a|opus|flac|wav|mp3|mp4|webm|ogg)$/i.test(f.name));
    if (!list.length) return;
    const next = list.map((file) => ({ key: crypto.randomUUID(), file, state: "waiting" as const }));
    setJobs((js) => {
      const all = [...js.filter((j) => j.state !== "done" && j.state !== "cancelled"), ...next];
      queueMicrotask(() => void pump(all));
      return all;
    });
    // On a phone the queue sits below the settings: bring the progress into view.
    setTimeout(() => document.querySelector(".queue")?.scrollIntoView({ block: "nearest", behavior: "smooth" }), 60);
  }

  const busy = jobs.some((j) => j.state === "running" || j.state === "waiting");

  return (
    <div className="home">
      <section
        className={`sleeve${over ? " over" : ""}`}
        aria-labelledby="drop-title"
        onDragOver={(e) => {
          e.preventDefault();
          setOver(true);
        }}
        onDragLeave={() => setOver(false)}
        onDrop={(e) => {
          e.preventDefault();
          setOver(false);
          add(e.dataTransfer.files);
        }}
      >
        <p className="sleeve-credits" aria-hidden="true">
          <span>verbatim</span>
          <span>{t("Speech to text · on this device", "Parole en texte · sur cet appareil")}</span>
        </p>
        <ol className="sleeve-steps">
          <li>
            <span className="num">1</span> {t("Drop the file; the model runs here", "Déposez le fichier ; le modèle tourne ici")}
          </li>
          <li>
            <span className="num">2</span> {t("Read along, correct, cut", "Suivez, corrigez, coupez")}
          </li>
          <li>
            <span className="num">3</span> {t("Take the text, subtitles or the edit", "Emportez le texte, les sous-titres ou le montage")}
          </li>
        </ol>
        <h1 id="drop-title">{t("Drop a video or audio file", "Déposez une vidéo ou un fichier audio")}</h1>
        <p className="sleeve-note">{t("It is transcribed here, in this tab. The file never leaves your device.", "Elle est transcrite ici, dans cet onglet. Le fichier ne quitte jamais votre appareil.")}</p>
        <button type="button" className="primary large" onClick={() => input.current?.click()}>
          <Icon name="upload" /> {t("Choose a file", "Choisir un fichier")}
        </button>
        <input
          ref={input}
          type="file"
          accept={ACCEPT}
          multiple
          hidden
          onChange={(e) => {
            if (e.currentTarget.files) add(e.currentTarget.files);
            e.currentTarget.value = "";
          }}
        />
        <p className="sleeve-formats">MP4 · MOV · WEBM · MKV · MP3 · M4A · WAV · OGG · FLAC</p>
      </section>

      <aside className="side">
      <section className="liner" aria-label={t("Transcription settings", "Réglages de transcription")}>
        <label className="field">
          <span>{t("Engine", "Moteur")}</span>
          <select value={engineId} onChange={(e) => setEngineId(e.currentTarget.value as EngineId)}>
            {ENGINE_IDS.map((id) => (
              <option key={id} value={id}>
                {engineName(id, t)}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>{t("Spoken language", "Langue parlée")}</span>
          <select value={language} onChange={(e) => setLanguage(e.currentTarget.value)}>
            {LANGUAGES.map((code) => (
              <option key={code} value={code}>
                {code === "auto" ? t("Detect", "Détecter") : languageLabel(code, lang)}
              </option>
            ))}
          </select>
        </label>
        <p className="liner-cost" aria-live="polite">
          {engine.kind === "cloud" ? (
            t("The file is sent to the AI provider set up on the AI page. Up to 25 MB for most providers.", "Le fichier est envoyé au fournisseur d'IA configuré dans la page IA. 25 Mo au plus chez la plupart.")
          ) : (
            <>
              <strong>{engineLabel(engine)}</strong> ·{" "}
              {cached.has(cacheKey) ? t("on this device", "sur cet appareil") : t(`${engineSize(engine)} MB to download once`, `${engineSize(engine)} Mo à télécharger une fois`)}
              {engine.kind === "nemo" && engine.model === "parakeet" && language === "auto" ? (
                <> · {t("Parakeet finds the language itself. If the text comes out in another language, choose the spoken language and start again.", "Parakeet trouve la langue tout seul. Si le texte sort dans une autre langue, choisissez la langue parlée et relancez.")}</>
              ) : null}
              {problem === "whisper-auto" ? <> · {t(`Whisper does not detect the language: it will listen for ${languageLabel(navigator.language.slice(0, 2), lang)}.`, `Whisper ne détecte pas la langue : il écoutera du ${languageLabel(navigator.language.slice(0, 2), lang)}.`)}</> : null}
              {problem === "canary-language" ? <span className="warn"> · {t("Canary needs the spoken language: English, French, German or Spanish.", "Canary a besoin de la langue parlée : anglais, français, allemand ou espagnol.")}</span> : null}
            </>
          )}
        </p>
      </section>

      {jobs.length ? (
        <section className="queue" aria-label={t("Files", "Fichiers")}>
          <ol>
            {jobs.map((j) => {
              const r = ratioOf(j.stage);
              return (
                <li key={j.key} className={`job ${j.state}`}>
                  <div className="job-head">
                    <span className="job-name">{j.file.name}</span>
                    <span className="job-size num">{bytes(j.file.size, lang)}</span>
                  </div>
                  <div className="job-state" aria-live="polite">
                    {j.state === "running" || j.state === "waiting" ? <StageLine stage={j.stage} /> : null}
                    {j.state === "error" ? (
                      <span className="row">
                        <span className="error-text">{j.error}</span>
                        <button
                          type="button"
                          className="link"
                          onClick={() => {
                            const again = { ...j, state: "waiting" as const, error: undefined, stage: undefined };
                            setJobs((js) => {
                              const all = js.map((x) => (x.key === j.key ? again : x));
                              queueMicrotask(() => void pump(all));
                              return all;
                            });
                          }}
                        >
                          {t("Try again", "Réessayer")}
                        </button>
                      </span>
                    ) : null}
                    {j.state === "done" && j.docId ? <a href={`#/t/${j.docId}`}>{t("Open the transcript", "Ouvrir la transcription")}</a> : null}
                  </div>
                  {j.state === "running" && j.stage?.stage === "transcribe" && j.stage.live.length ? (
                    <ol className="live" aria-label={t("Transcript so far", "Transcription en cours")}>
                      {j.stage.live.slice(-5).map((s) => (
                        <li key={s.start}>
                          <span className="num">{clock(s.start)}</span> {s.text}
                        </li>
                      ))}
                    </ol>
                  ) : null}
                  {j.state === "running" ? (
                    <div className="meter" role="progressbar" aria-label={j.file.name} aria-valuemin={0} aria-valuemax={100} aria-valuenow={r === null ? undefined : Math.round(r * 100)}>
                      <span style={r === null ? undefined : { inlineSize: `${r * 100}%` }} className={r === null ? "indeterminate" : undefined} />
                    </div>
                  ) : null}
                </li>
              );
            })}
          </ol>
          {busy ? (
            <button
              type="button"
              className="link"
              onClick={() => {
                stopped.current = true;
                cancelRun();
                setJobs((js) => js.map((j) => (j.state === "running" || j.state === "waiting" ? { ...j, state: "cancelled" } : j)));
              }}
            >
              {t("Cancel", "Annuler")}
            </button>
          ) : null}
        </section>
      ) : null}
      </aside>
    </div>
  );
}
