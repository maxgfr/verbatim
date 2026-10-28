// Every format at one click, in the original language or a translation.

import { useState } from "react";
import type { EditorApi } from "./editor.tsx";
import { download, FORMATS, fileName, renderDoc, toZip } from "./engine/export.ts";
import { languageLabel, useLang } from "./i18n.ts";
import { Icon } from "./icons.tsx";

export function ExportPanel({ api }: { api: EditorApi }) {
  const { t, lang } = useLang();
  const { doc } = api;
  const [which, setWhich] = useState("");
  const translations = Object.keys(doc.translations ?? {});
  const l = which || undefined;
  return (
    <div className="export-panel">
      {translations.length ? (
        <label className="field inline">
          <span>{t("Language", "Langue")}</span>
          <select value={which} onChange={(e) => setWhich(e.currentTarget.value)}>
            <option value="">{t("Original", "Original")}</option>
            {translations.map((x) => (
              <option key={x} value={x}>
                {languageLabel(x, lang)}
              </option>
            ))}
          </select>
        </label>
      ) : null}
      <ul className="formats">
        {FORMATS.map((f) => (
          <li key={f.id}>
            <button type="button" className="secondary" onClick={() => download(fileName(doc, f.id, l), renderDoc(doc, f.id, l), f.mime)}>
              <Icon name="download" size={16} /> {f.id === "txt-timed" ? t("TXT with times", "TXT horodaté") : f.label}
            </button>
          </li>
        ))}
      </ul>
      <div className="row">
        <button type="button" className="primary" onClick={() => download(`${fileName(doc, "txt").replace(/\.txt$/, "")}.zip`, toZip(doc) as BlobPart, "application/zip")}>
          <Icon name="download" size={16} /> {t("Everything (zip)", "Tout (zip)")}
        </button>
        <button type="button" className="secondary" onClick={() => void navigator.clipboard.writeText(renderDoc(doc, "txt", l))}>
          <Icon name="copy" size={16} /> {t("Copy the text", "Copier le texte")}
        </button>
      </div>
      <p className="note">{t("JSON opens in the CLI too: verbatim export file.json -f srt.", "Le JSON s'ouvre aussi dans la CLI : verbatim export fichier.json -f srt.")}</p>
    </div>
  );
}
