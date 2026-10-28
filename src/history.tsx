// Every transcript kept on this device, newest first.

import { clearDocs, deleteDoc, useDocs } from "./engine/store.ts";
import { clock } from "./engine/time.ts";
import { dateTime, languageLabel, useLang } from "./i18n.ts";
import { Icon } from "./icons.tsx";

export function History() {
  const { t, lang } = useLang();
  const docs = useDocs();
  return (
    <div className="page">
      <div className="page-head">
        <h1>{t("History", "Historique")}</h1>
        <p className="note">{t("Transcripts stay in this browser. The media files are never stored.", "Les transcriptions restent dans ce navigateur. Les fichiers média ne sont jamais enregistrés.")}</p>
      </div>
      {docs.length ? (
        <>
          <ol className="history">
            {docs.map((d, i) => (
              <li key={d.id}>
                <span className="track-no num" aria-hidden="true">
                  {i + 1}
                </span>
                <a href={`#/t/${d.id}`} className="history-name">
                  {d.name}
                </a>
                <span className="num note">
                  {clock(d.transcript.duration ?? 0)}
                  {d.transcript.language ? ` · ${languageLabel(d.transcript.language, lang)}` : ""} · {d.engine}
                  {d.cuts?.length ? ` · ${t(`${d.cuts.length} cut(s)`, `${d.cuts.length} coupe(s)`)}` : ""}
                </span>
                <span className="num note">{dateTime(d.updatedAt, lang)}</span>
                <button type="button" className="icon-button" onClick={() => void deleteDoc(d.id)} aria-label={t(`Delete ${d.name}`, `Supprimer ${d.name}`)}>
                  <Icon name="trash" size={16} />
                </button>
              </li>
            ))}
          </ol>
          <button type="button" className="link danger" onClick={() => void clearDocs()}>
            {t("Delete every transcript", "Supprimer toutes les transcriptions")}
          </button>
        </>
      ) : (
        <div className="empty">
          <p>{t("Nothing transcribed yet.", "Rien de transcrit pour l'instant.")}</p>
          <a className="button primary" href="#/">
            {t("Transcribe a file", "Transcrire un fichier")}
          </a>
        </div>
      )}
    </div>
  );
}
