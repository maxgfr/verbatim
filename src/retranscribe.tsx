// Not the right text (a language guessed wrong, a weaker model)? Transcribe
// the same file again with the language named, watching it come in. The cuts
// are kept (they are times); what was made from the old text is dropped.

import { useState } from "react";
import { engineLabel, LANGUAGES, resolveEngine } from "./engine/engines.ts";
import { cancelRun, type Stage, transcribeFile } from "./engine/run.ts";
import { clock } from "./engine/time.ts";
import type { Doc } from "./engine/types.ts";
import { languageLabel, useLang } from "./i18n.ts";

export function Retranscribe({ doc, file, update }: { doc: Doc; file: File | null; update: (fn: (d: Doc) => Doc) => void }) {
  const { t, lang } = useLang();
  const [language, setLanguage] = useState(() => doc.transcript.language ?? (lang === "fr" ? "fr" : "en"));
  const [stage, setStage] = useState<Stage | null>(null);
  const [error, setError] = useState("");
  const engine = resolveEngine("auto", language);

  async function run() {
    if (!file) return;
    setError("");
    setStage({ stage: "decode" });
    try {
      const fresh = await transcribeFile(file, engine, language, setStage);
      update((d) => ({ ...d, engine: fresh.engine, transcript: { ...fresh.transcript, duration: Math.max(fresh.transcript.duration ?? 0, d.transcript.duration ?? 0) }, summary: undefined, chapters: undefined, translations: undefined }));
    } catch (e) {
      if ((e as Error).message !== "cancelled") setError((e as Error).message);
    } finally {
      setStage(null);
    }
  }

  const live = stage?.stage === "transcribe" ? stage.live : [];
  const ratio = stage?.stage === "transcribe" ? stage.ratio : stage?.stage === "download" ? stage.ratio : null;

  return (
    <details className="retranscribe">
      <summary>{t("Not the right text? Transcribe again", "Pas le bon texte ? Retranscrire")}</summary>
      {file ? (
        <div className="retranscribe-body">
          <div className="row">
            <label className="field inline">
              <span>{t("Spoken language", "Langue parlée")}</span>
              <select value={language} disabled={!!stage} onChange={(e) => setLanguage(e.currentTarget.value)}>
                {LANGUAGES.map((code) => (
                  <option key={code} value={code}>
                    {code === "auto" ? t("Detect", "Détecter") : languageLabel(code, lang)}
                  </option>
                ))}
              </select>
            </label>
            {stage ? (
              <button type="button" className="link" onClick={cancelRun}>
                {t("Cancel", "Annuler")}
              </button>
            ) : (
              <button type="button" className="secondary small" onClick={() => void run()}>
                {t(`Transcribe with ${engineLabel(engine)}`, `Transcrire avec ${engineLabel(engine)}`)}
              </button>
            )}
          </div>
          <p className="note">{t("Cuts are kept; the summary, chapters and translations of the old text are removed.", "Les coupes sont gardées ; le résumé, les chapitres et les traductions de l'ancien texte sont retirés.")}</p>
          {stage ? (
            <div className="meter" role="progressbar" aria-label={t("Transcription", "Transcription")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={ratio === null ? undefined : Math.round(ratio * 100)}>
              <span style={ratio === null ? undefined : { inlineSize: `${ratio * 100}%` }} className={ratio === null ? "indeterminate" : undefined} />
            </div>
          ) : null}
          {live.length ? (
            <ol className="live" aria-label={t("Transcript so far", "Transcription en cours")}>
              {live.slice(-5).map((s) => (
                <li key={s.start}>
                  <span className="num">{clock(s.start)}</span> {s.text}
                </li>
              ))}
            </ol>
          ) : null}
          {error ? <p className="error-text">{error}</p> : null}
        </div>
      ) : (
        <p className="note">{t("Open the file again above to transcribe it again.", "Rouvrez le fichier ci-dessus pour le retranscrire.")}</p>
      )}
    </details>
  );
}
