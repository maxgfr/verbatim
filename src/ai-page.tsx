// AI settings: an optional provider with the user's own key, or a model on
// their machine through an OpenAI-compatible server (Ollama, LM Studio, Jan).

import "./ai/zod-jitless.ts";
import { ENDPOINT_PRESETS } from "llm-connect";
import { useLlmSettings } from "llm-connect/react";
import { useEffect, useState } from "react";
import { llm, loadLlm } from "./ai/byok.ts";
import { useLang } from "./i18n.ts";

export function AiPage() {
  const { t } = useLang();
  const s = useLlmSettings(llm);
  const [status, setStatus] = useState("");
  useEffect(() => {
    void loadLlm().then(() => llm.open());
  }, []);
  const providers = [...s.providers.recommended, ...s.providers.others];
  const custom = s.draft.custom;
  return (
    <div className="page narrow">
      <div className="page-head">
        <h1>{t("AI provider", "Fournisseur d'IA")}</h1>
        <p className="note">
          {t(
            "Optional. Transcription needs none: it runs on your device. A provider adds summaries, chapters, translations, cleanup and edits said in your own words, and can transcribe in the cloud if you pick it as the engine. Only what you ask for is sent; the key is encrypted in this browser.",
            "Facultatif. La transcription n'en a pas besoin : elle tourne sur votre appareil. Un fournisseur ajoute résumés, chapitres, traductions, nettoyage et montage dit avec vos mots, et peut transcrire dans le cloud si vous le choisissez comme moteur. Seul ce que vous demandez est envoyé ; la clé est chiffrée dans ce navigateur.",
          )}
        </p>
      </div>
      <div className="segmented wide" role="group" aria-label={t("Kind of provider", "Type de fournisseur")}>
        <button type="button" aria-pressed={!custom} onClick={() => llm.toggleCustom(false)}>
          {t("Online provider", "Fournisseur en ligne")}
        </button>
        <button type="button" aria-pressed={custom} onClick={() => llm.toggleCustom(true)}>
          {t("On my machine", "Sur ma machine")}
        </button>
      </div>
      <div className="fields">
        {custom ? (
          <>
            <div className="row">
              {Object.values(ENDPOINT_PRESETS).map((p) => (
                <button key={p.id} type="button" className="secondary small" onClick={() => llm.applyPreset(p)}>
                  {p.name}
                </button>
              ))}
            </div>
            <label className="field">
              <span>{t("Server address", "Adresse du serveur")}</span>
              <input type="url" spellCheck={false} value={s.draft.customEndpoint} placeholder="http://localhost:11434/v1" onChange={(e) => llm.setEndpoint(e.currentTarget.value)} />
            </label>
            <label className="field">
              <span>{t("Model", "Modèle")}</span>
              <input type="text" spellCheck={false} value={s.draft.customModel} onChange={(e) => llm.setCustomModel(e.currentTarget.value)} />
            </label>
            <p className="note">{t("Ollama must allow this site: OLLAMA_ORIGINS=https://maxgfr.github.io ollama serve", "Ollama doit autoriser ce site : OLLAMA_ORIGINS=https://maxgfr.github.io ollama serve")}</p>
          </>
        ) : (
          <>
            <label className="field">
              <span>{t("Provider", "Fournisseur")}</span>
              <select value={s.draft.providerId} onChange={(e) => llm.selectProvider(e.currentTarget.value)}>
                <option value="">{t("Choose…", "Choisir…")}</option>
                {providers.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </label>
            {s.provider ? (
              <label className="field">
                <span>{t("Model", "Modèle")}</span>
                <select value={s.draft.model} onChange={(e) => llm.selectModel(e.currentTarget.value)}>
                  {s.models.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.name ?? m.id}
                      {s.recommendedModel?.id === m.id ? ` · ${t("recommended", "recommandé")}` : ""}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
          </>
        )}
        <label className="field">
          <span>{custom ? t("API key (often none)", "Clé d'API (souvent aucune)") : t("API key", "Clé d'API")}</span>
          <input type="password" autoComplete="off" spellCheck={false} value={s.apiKey} onChange={(e) => llm.setApiKey(e.currentTarget.value)} />
        </label>
        <label className="check">
          <input type="checkbox" checked={s.draft.rememberKey} onChange={(e) => llm.setRememberKey(e.currentTarget.checked)} />
          {t("Remember the key on this device (encrypted)", "Retenir la clé sur cet appareil (chiffrée)")}
        </label>
      </div>
      <div className="row">
        <button
          type="button"
          className="secondary"
          disabled={!s.canTest || s.testing}
          onClick={async () => {
            const r = await llm.test();
            setStatus(r.ok ? t("The connection works.", "La connexion fonctionne.") : t(`Connection failed: ${r.message ?? r.reason}`, `Échec de la connexion : ${r.message ?? r.reason}`));
          }}
        >
          {s.testing ? t("Testing…", "Test…") : t("Test", "Tester")}
        </button>
        <button
          type="button"
          className="primary"
          disabled={!s.canSave}
          onClick={async () => {
            const r = await llm.save();
            setStatus(
              r.ok
                ? t("Saved.", "Enregistré.")
                : r.reason === "insecure-endpoint"
                  ? t("Use https, or http only on this machine (localhost).", "Utilisez https, ou http seulement sur cette machine (localhost).")
                  : t("Some settings are missing or invalid.", "Des réglages manquent ou sont invalides."),
            );
          }}
        >
          {t("Save", "Enregistrer")}
        </button>
        {s.saved ? (
          <button type="button" className="link" onClick={() => void llm.clear().then(() => setStatus(t("Removed.", "Supprimé.")))}>
            {t("Forget this provider", "Oublier ce fournisseur")}
          </button>
        ) : null}
      </div>
      <p className="note" role="status">
        {status}
      </p>
      {s.catalogStatus === "error" ? <p className="error-text">{t("The model list could not load.", "La liste des modèles n'a pas pu se charger.")}</p> : null}
    </div>
  );
}
