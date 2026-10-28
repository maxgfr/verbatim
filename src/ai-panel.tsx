// Summary, chapters, translation and cleanup with the user's own model. Only
// the transcript's text is sent; the panel says so and names the provider.

import { useLlmSettings } from "llm-connect/react";
import { useEffect, useRef, useState } from "react";
import { llm, loadLlm } from "./ai/byok.ts";
import type { EditorApi } from "./editor.tsx";
import { languageLabel, useLang } from "./i18n.ts";

const TARGETS = ["en", "fr", "de", "es", "it", "pt", "nl", "pl", "uk", "ja", "zh", "ar"];

export function AiPanel({ api, onShow }: { api: EditorApi; onShow: (lang: string) => void }) {
  const { t, lang } = useLang();
  const s = useLlmSettings(llm);
  const [busy, setBusy] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);
  const [status, setStatus] = useState("");
  const [target, setTarget] = useState<string>(lang === "fr" && api.doc.transcript.language !== "en" ? "en" : lang);
  const [before, setBefore] = useState<string[] | null>(null);
  const abort = useRef<AbortController | null>(null);
  useEffect(() => {
    void loadLlm();
  }, []);

  const saved = s.saved;
  if (!saved) {
    return (
      <div className="ai-panel">
        <p>{t("Summary, chapters, translation and cleanup use an AI provider you choose, with your own key, or a model on your machine (Ollama, LM Studio).", "Résumé, chapitres, traduction et nettoyage passent par le fournisseur d'IA de votre choix, avec votre clé, ou un modèle sur votre machine (Ollama, LM Studio).")}</p>
        <a className="button secondary" href="#/ai">
          {t("Set up a provider", "Configurer un fournisseur")}
        </a>
      </div>
    );
  }

  async function run(kind: string, job: (signal: AbortSignal) => Promise<void>) {
    abort.current = new AbortController();
    setBusy(kind);
    setProgress(0);
    setStatus("");
    try {
      await job(abort.current.signal);
    } catch (e) {
      if ((e as Error).name !== "AbortError") setStatus((e as Error).message);
    } finally {
      setBusy(null);
    }
  }

  const ai = () => import("./ai/byok.ts");
  const custom = !!saved.customEndpoint;
  const provider = custom ? saved.customEndpoint : (saved.providerName ?? saved.providerId);
  const modelName = custom ? saved.customModel : saved.model;

  return (
    <div className="ai-panel">
      <p className="note">
        {t(`The transcript's text goes to ${provider} (${modelName}). The media stays here.`, `Le texte de la transcription part chez ${provider} (${modelName}). Le média reste ici.`)}
      </p>
      <div className="ai-actions">
        <button
          type="button"
          className="secondary"
          disabled={!!busy}
          onClick={() =>
            void run("summary", async (signal) => {
              const summary = await (await ai()).summarize(api.doc, lang, signal);
              api.update((d) => ({ ...d, summary }));
            })
          }
        >
          {busy === "summary" ? t("Summarizing…", "Résumé…") : api.doc.summary ? t("Summarize again", "Refaire le résumé") : t("Summarize", "Résumer")}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={!!busy}
          onClick={() =>
            void run("chapters", async (signal) => {
              const chapters = await (await ai()).chapters(api.doc, lang, signal);
              api.update((d) => ({ ...d, chapters }));
            })
          }
        >
          {busy === "chapters" ? t("Finding chapters…", "Chapitrage…") : t("Chapters", "Chapitres")}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={!!busy}
          onClick={() =>
            void run("cleanup", async (signal) => {
              const old = api.doc.transcript.segments.map((x) => x.text);
              const lines = await (await ai()).cleanUp(api.doc, setProgress, signal);
              setBefore(old);
              api.update((d) => ({ ...d, transcript: { ...d.transcript, segments: d.transcript.segments.map((x, i) => ({ ...x, text: lines[i] ?? x.text })) } }));
            })
          }
        >
          {busy === "cleanup" ? t(`Cleaning up… ${Math.round(progress * 100)} %`, `Nettoyage… ${Math.round(progress * 100)} %`) : t("Clean up the text", "Nettoyer le texte")}
        </button>
        {before && !busy ? (
          <button
            type="button"
            className="link"
            onClick={() => {
              api.update((d) => ({ ...d, transcript: { ...d.transcript, segments: d.transcript.segments.map((x, i) => ({ ...x, text: before[i] ?? x.text })) } }));
              setBefore(null);
            }}
          >
            {t("Undo the cleanup", "Annuler le nettoyage")}
          </button>
        ) : null}
      </div>
      <div className="row">
        <label className="field inline">
          <span>{t("Translate into", "Traduire en")}</span>
          <select value={target} onChange={(e) => setTarget(e.currentTarget.value)}>
            {TARGETS.map((l) => (
              <option key={l} value={l}>
                {languageLabel(l, lang)}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="secondary"
          disabled={!!busy || target === api.doc.transcript.language}
          onClick={() =>
            void run("translate", async (signal) => {
              const lines = await (await ai()).translate(api.doc, target, setProgress, signal);
              api.update((d) => ({ ...d, translations: { ...d.translations, [target]: lines } }));
              onShow(target);
            })
          }
        >
          {busy === "translate" ? t(`Translating… ${Math.round(progress * 100)} %`, `Traduction… ${Math.round(progress * 100)} %`) : t("Translate", "Traduire")}
        </button>
      </div>
      {busy ? (
        <button type="button" className="link" onClick={() => abort.current?.abort()}>
          {t("Stop", "Arrêter")}
        </button>
      ) : null}
      <p className="note" role="status">
        {status}
      </p>
    </div>
  );
}
